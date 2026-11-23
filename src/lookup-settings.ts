import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { getAgentDir } from '@earendil-works/pi-coding-agent';
import { Type } from 'typebox';
import { Value } from 'typebox/value';

const schema = Type.Object(
  {
    recordOnly: Type.Optional(Type.Boolean()),
    model: Type.Optional(Type.String({ pattern: '^[^/\\s]+/\\S+$' })),
    thinkingLevel: Type.Optional(
      Type.Union([
        Type.Literal('off'),
        Type.Literal('minimal'),
        Type.Literal('low'),
        Type.Literal('medium'),
        Type.Literal('high'),
        Type.Literal('xhigh'),
      ]),
    ),
    timeoutMs: Type.Optional(Type.Integer({ minimum: 1, maximum: 2_147_483_647 })),
  },
  { additionalProperties: false },
);

export async function loadLookupSettings() {
  const path = join(getAgentDir(), 'agent-memory', 'readcode.json');
  let input: unknown;
  try {
    input = JSON.parse(await readFile(path, 'utf8'));
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') {
      throw new Error(`Could not read valid JSON from ${path}.`, { cause: error });
    }
    input = {};
  }
  if (!Value.Check(schema, input)) {
    const [issue] = Value.Errors(schema, input);
    throw new Error(
      `Invalid lookup settings in ${path}: ${issue?.path || '/'} ${issue?.message || ''}`,
    );
  }
  return { recordOnly: false, thinkingLevel: 'off' as const, timeoutMs: 20_000, ...input };
}

export type LookupSettings = Awaited<ReturnType<typeof loadLookupSettings>>;
