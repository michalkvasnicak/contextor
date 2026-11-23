import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import {
  CONFIG_DIR_NAME,
  getAgentDir,
  type ExtensionContext,
} from '@earendil-works/pi-coding-agent';
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

async function readLookupSettings(path: string) {
  let input: unknown;
  try {
    input = JSON.parse(await readFile(path, 'utf8'));
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') return undefined;
    throw new Error(`Could not read valid JSON from ${path}.`, { cause: error });
  }
  if (!Value.Check(schema, input)) {
    const [issue] = Value.Errors(schema, input);
    throw new Error(
      `Invalid lookup settings in ${path}: ${issue?.path || '/'} ${issue?.message || ''}`,
    );
  }
  return input;
}

export async function loadLookupSettings(ctx: Pick<ExtensionContext, 'cwd' | 'isProjectTrusted'>) {
  const globalSettings = await readLookupSettings(
    join(getAgentDir(), 'agent-memory', 'config.json'),
  );
  const projectSettings = ctx.isProjectTrusted()
    ? await readLookupSettings(join(ctx.cwd, CONFIG_DIR_NAME, 'pi-memory.json'))
    : undefined;

  return {
    recordOnly: false,
    thinkingLevel: 'off' as const,
    timeoutMs: 20_000,
    ...globalSettings,
    ...projectSettings,
  };
}

export type LookupSettings = Awaited<ReturnType<typeof loadLookupSettings>>;
