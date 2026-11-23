import { appendFile, mkdir } from 'node:fs/promises';
import { join } from 'node:path';
import { getAgentDir, type ExtensionContext } from '@earendil-works/pi-coding-agent';

export async function recordQuestion(toolCallId: string, question: string, ctx: ExtensionContext) {
  if (process.env.PI_MEMORY_LOG_QUESTIONS === '0') return;

  const sessionFile = ctx.sessionManager.getSessionFile();
  if (!sessionFile) return;

  const entry = {
    timestamp: new Date().toISOString(),
    toolCallId,
    question,
    cwd: ctx.cwd,
    sessionId: ctx.sessionManager.getSessionId(),
    sessionFile,
    sessionName: ctx.sessionManager.getSessionName() ?? null,
    mode: ctx.mode,
    provider: ctx.model?.provider ?? null,
    model: ctx.model?.id ?? null,
    thinkingLevel: ctx.thinkingLevel ?? null,
  };
  const directory = join(getAgentDir(), 'agent-memory');
  await mkdir(directory, { recursive: true, mode: 0o700 });
  await appendFile(join(directory, 'questions.jsonl'), `${JSON.stringify(entry)}\n`, {
    mode: 0o600,
  });
}
