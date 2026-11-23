import { constants } from 'node:fs';
import { open, realpath, stat } from 'node:fs/promises';
import { isAbsolute, relative, resolve, sep } from 'node:path';
import { defineTool } from '@earendil-works/pi-coding-agent';
import type { McpClient } from '@earendil-works/pi-mcp';
import { Type } from 'typebox';

const maxFileBytes = 2 * 1024 * 1024;
const maxExcerptChars = 12_000;

async function resolveFile(root: string, path: string) {
  if (isAbsolute(path)) throw new Error('Use a repository-relative path.');
  const file = await realpath(resolve(root, path));
  const name = relative(root, file);
  if (!name || name === '..' || name.startsWith(`..${sep}`) || isAbsolute(name)) {
    throw new Error('Path is outside the repository.');
  }
  if (!(await stat(file)).isFile()) throw new Error('Path is not a regular file.');
  return { file, name };
}

export function createLookupTools(root: string, fff: McpClient) {
  const read = defineTool({
    name: 'read',
    label: 'Read source',
    description: 'Read a bounded, line-numbered excerpt from a source file in the repository.',
    parameters: Type.Object({
      path: Type.String({ minLength: 1 }),
      offset: Type.Optional(Type.Integer({ minimum: 1 })),
      limit: Type.Optional(Type.Integer({ minimum: 1, maximum: 200 })),
    }),
    async execute(_id, { path, offset = 1, limit = 100 }, signal) {
      const { file, name } = await resolveFile(root, path);
      const handle = await open(
        file,
        constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK,
      );
      try {
        const { size } = await handle.stat();
        if (size > maxFileBytes) throw new Error('Source file exceeds the 2 MiB read limit.');
        const { buffer, bytesRead } = await handle.read(Buffer.alloc(size));
        signal?.throwIfAborted();
        const bytes = buffer.subarray(0, bytesRead);
        if (bytes.includes(0)) throw new Error('Binary files are not supported.');
        const lines = bytes.toString('utf8').split(/\r?\n/);
        const excerpt: string[] = [];
        let remaining = maxExcerptChars;
        for (let index = offset - 1; index < Math.min(lines.length, offset - 1 + limit); index++) {
          if (remaining < 32) break;
          const row = `${index + 1}: ${lines[index]}`;
          excerpt.push(row.length > remaining ? `${row.slice(0, remaining - 1)}…` : row);
          remaining -= excerpt.at(-1)!.length + 1;
        }
        const next = offset + excerpt.length;
        const tail =
          excerpt.length && next <= lines.length ? `\nMore lines: use offset ${next}.` : '';
        return {
          content: [
            {
              type: 'text',
              text: `${name}\n${excerpt.join('\n') || 'No lines at this offset.'}${tail}`,
            },
          ],
          details: {},
        };
      } finally {
        await handle.close();
      }
    },
  });

  async function search(
    name: 'find_files' | 'grep' | 'multi_grep',
    args: Record<string, unknown>,
    signal?: AbortSignal,
  ) {
    const result = await fff.callTool(name, { ...args, maxResults: 20 }, { signal });
    if (result.isError) throw new Error(`FFF ${name} failed.`);
    const matches = new Set<string>();
    for (const block of result.content) {
      if (block.type !== 'text') continue;
      let path: string | undefined;
      for (const row of block.text.split('\n')) {
        signal?.throwIfAborted();
        // Return locations only, never forward unchecked snippets or server hints.
        const line = /^\s+(\d+)[:|]/.exec(row)?.[1];
        if (line && path && name !== 'find_files') matches.add(`${path}:${line}`);
        if (!row || /^\s|^→/.test(row)) continue;
        path = undefined;
        const candidate = row.replace(/ (?:git:\S+|\[def\])$/, '');
        try {
          path = (await resolveFile(root, candidate)).name;
          if (name === 'find_files') matches.add(path);
        } catch {
          // Headers, stale index entries, and paths outside the root aren't results.
        }
      }
    }
    return {
      content: [
        { type: 'text' as const, text: [...matches].join('\n') || 'No matching source locations.' },
      ],
      details: {},
    };
  }

  return [
    read,
    defineTool({
      name: 'fff_find_files',
      label: 'Find files with FFF',
      description: 'Find repository files by fuzzy name or path. Keep the query short.',
      parameters: Type.Object({ query: Type.String({ minLength: 1, maxLength: 300 }) }),
      execute: (_id, args, signal) => search('find_files', args, signal),
    }),
    defineTool({
      name: 'fff_grep',
      label: 'Search with FFF',
      description:
        'Find source locations matching a symbol or text. Supports FFF path and glob constraints.',
      parameters: Type.Object({ query: Type.String({ minLength: 1, maxLength: 300 }) }),
      execute: (_id, args, signal) => search('grep', args, signal),
    }),
    defineTool({
      name: 'fff_multi_grep',
      label: 'Search multiple terms with FFF',
      description: 'Find source locations matching any of several literal terms.',
      parameters: Type.Object({
        patterns: Type.Array(Type.String({ minLength: 1, maxLength: 120 }), {
          minItems: 1,
          maxItems: 8,
        }),
        constraints: Type.Optional(Type.String({ maxLength: 300 })),
      }),
      execute: (_id, args, signal) => search('multi_grep', args, signal),
    }),
  ];
}
