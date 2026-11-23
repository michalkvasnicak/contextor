import { randomUUID } from 'node:crypto';
import { existsSync } from 'node:fs';
import { mkdtemp, rmdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  type ExtensionAPI,
  type ExtensionContext,
  SessionManager,
} from '@earendil-works/pi-coding-agent';
import { Type } from 'typebox';
import { loadLookupSettings } from './lookup-settings.ts';
import { lookupCodebase } from './lookup.ts';
import { recordQuestion } from './question-store.ts';

const miss = {
  status: 'miss',
  message: 'No codebase memory is available. Continue by inspecting the source.',
  findings: [],
} as const;

export default function (pi: ExtensionAPI) {
  let warnedAboutQuestionLog = false;
  const lifetime = new AbortController();
  pi.on('session_shutdown', () => lifetime.abort());

  async function ask(
    id: string,
    question: string,
    ctx: ExtensionContext,
    direct: boolean,
    signal?: AbortSignal,
    transcript?: SessionManager,
  ) {
    function warnAboutQuestionLog() {
      if (ctx.hasUI && !warnedAboutQuestionLog) {
        warnedAboutQuestionLog = true;
        ctx.ui.notify('Could not write the codebase lookup question log.', 'warning');
      }
    }

    const saveAnswer = await recordQuestion(id, question, ctx).catch(warnAboutQuestionLog);

    try {
      const settings = await loadLookupSettings(ctx);
      if (!direct && (settings.recordOnly || !settings.model)) return miss;
      if (!settings.model) {
        throw new Error(
          'Set model in .pi/pi-memory.json or agent-memory/config.json under your Pi agent directory.',
        );
      }
      const result = await lookupCodebase(
        ctx.cwd,
        question,
        { ...settings, model: settings.model },
        signal ? AbortSignal.any([signal, lifetime.signal]) : lifetime.signal,
        transcript,
      );
      if (saveAnswer) await saveAnswer(result.answer).catch(warnAboutQuestionLog);
      return result;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown lookup failure.';
      return { status: 'error', message, findings: [] } as const;
    }
  }

  pi.registerTool({
    name: 'codebase_lookup',
    label: 'Codebase Lookup',
    description:
      "Ask about the current repository's code layout and architecture: where a behavior is implemented, which modules own it, how a request or data flow crosses components, or where relevant tests live. Returns source references when available, or a miss.",
    promptSnippet: 'Find code by behavior, trace component boundaries, and locate related tests.',
    parameters: Type.Object(
      {
        question: Type.String({
          minLength: 1,
          description:
            'A question about where code lives or how it fits together in this repository.',
        }),
      },
      { additionalProperties: false },
    ),

    async execute(toolCallId, { question }, signal, _onUpdate, ctx) {
      const result = await ask(toolCallId, question, ctx, false, signal);
      return {
        content: [
          {
            type: 'text',
            text: result.status === 'answered' ? result.answer.markdown : JSON.stringify(result),
          },
        ],
        details: {},
      };
    },
  });

  pi.registerCommand('codebase_lookup', {
    description: 'Ask the read-only codebase guide. Add --transcript to save the lookup session.',
    async handler(args, ctx) {
      const input = args.trim();
      const saveTranscript = /^--transcript(?:\s|$)/.test(input);
      const question = saveTranscript ? input.slice('--transcript'.length).trim() : input;
      if (!question) {
        ctx.ui.notify('Usage: /codebase_lookup [--transcript] <question>', 'warning');
        return;
      }
      ctx.ui.setStatus('codebase_lookup', 'Looking up code…');
      try {
        let transcript: SessionManager | undefined;
        let transcriptDir: string | undefined;
        if (saveTranscript) {
          try {
            transcriptDir = await mkdtemp(join(tmpdir(), 'pi-memory-lookup-'));
            transcript = SessionManager.create(ctx.cwd, transcriptDir);
          } catch {
            ctx.ui.notify('Could not create a codebase lookup transcript.', 'warning');
          }
        }
        const result = await ask(
          `command:${randomUUID()}`,
          question,
          ctx,
          true,
          ctx.signal,
          transcript,
        );
        const transcriptPath = transcript?.getSessionFile();
        const saved = transcriptPath && existsSync(transcriptPath);
        if (!saved && transcriptDir) await rmdir(transcriptDir).catch(() => {});
        if (lifetime.signal.aborted) return;
        if (result.status === 'answered') {
          ctx.ui.notify(result.answer.markdown, 'info');
        } else {
          const hint =
            result.status === 'error' && !saveTranscript
              ? '\nRerun with /codebase_lookup --transcript <question> to debug.'
              : '';
          ctx.ui.notify(`${result.message}${hint}`, result.status === 'error' ? 'error' : 'info');
        }
        if (saved) ctx.ui.notify(`Lookup transcript: ${transcriptPath}`, 'info');
      } finally {
        if (!lifetime.signal.aborted) ctx.ui.setStatus('codebase_lookup', undefined);
      }
    },
  });
}
