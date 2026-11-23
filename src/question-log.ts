import { mkdir, open } from 'node:fs/promises';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { getAgentDir, type ExtensionContext } from '@earendil-works/pi-coding-agent';
import { sql } from 'drizzle-orm';
import { integer, sqliteTable, text } from 'drizzle-orm/sqlite-core';

const questions = sqliteTable('questions', {
  id: integer('id').primaryKey(),
  timestamp: text('timestamp').notNull(),
  toolCallId: text('tool_call_id').notNull(),
  question: text('question').notNull(),
  cwd: text('cwd').notNull(),
  sessionId: text('session_id').notNull(),
  sessionFile: text('session_file').notNull(),
  sessionName: text('session_name'),
  mode: text('mode').notNull(),
  provider: text('provider'),
  model: text('model'),
  thinkingLevel: text('thinking_level'),
});

const createQuestionsTable = sql`
  CREATE TABLE IF NOT EXISTS questions (
    id INTEGER PRIMARY KEY,
    timestamp TEXT NOT NULL,
    tool_call_id TEXT NOT NULL,
    question TEXT NOT NULL,
    cwd TEXT NOT NULL,
    session_id TEXT NOT NULL,
    session_file TEXT NOT NULL,
    session_name TEXT,
    mode TEXT NOT NULL,
    provider TEXT,
    model TEXT,
    thinking_level TEXT
  )
`;

export async function recordQuestion(toolCallId: string, question: string, ctx: ExtensionContext) {
  if (process.env.PI_MEMORY_LOG_QUESTIONS === '0') return;

  const sessionFile = ctx.sessionManager.getSessionFile();
  if (!sessionFile) return;

  const entry: typeof questions.$inferInsert = {
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
  const path = join(directory, 'questions.sqlite');
  const file = await open(path, 'a', 0o600);
  await file.close();

  const { drizzle } = await import('drizzle-orm/libsql/sqlite3');
  const db = drizzle({ connection: { url: pathToFileURL(path).href, timeout: 1000 } });
  try {
    await db.run(createQuestionsTable);
    await db.insert(questions).values(entry).run();
  } finally {
    db.$client.close();
  }
}
