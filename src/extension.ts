import { randomUUID } from 'node:crypto';
import type { ExtensionAPI, ExtensionContext } from '@earendil-works/pi-coding-agent';
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
  ) {
    try {
      await recordQuestion(id, question, ctx);
    } catch {
      if (ctx.hasUI && !warnedAboutQuestionLog) {
        warnedAboutQuestionLog = true;
        ctx.ui.notify('Could not write the codebase lookup question log.', 'warning');
      }
    }

    try {
      const settings = await loadLookupSettings(ctx);
      if (!direct && (settings.recordOnly || !settings.model)) return miss;
      if (!settings.model) {
        throw new Error(
          'Set model in .pi/pi-memory.json or agent-memory/config.json under your Pi agent directory.',
        );
      }
      return await lookupCodebase(
        ctx.cwd,
        question,
        { ...settings, model: settings.model },
        signal ? AbortSignal.any([signal, lifetime.signal]) : lifetime.signal,
      );
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
        content: [{ type: 'text', text: JSON.stringify(result) }],
        details: {},
      };
    },
  });

  pi.registerCommand('codebase_lookup', {
    description:
      'Ask the read-only codebase guide (even when automatic lookups only record questions).',
    async handler(args, ctx) {
      const question = args.trim();
      if (!question) {
        ctx.ui.notify('Usage: /codebase_lookup <question>', 'warning');
        return;
      }
      ctx.ui.setStatus('codebase_lookup', 'Looking up code…');
      try {
        const result = await ask(`command:${randomUUID()}`, question, ctx, true, ctx.signal);
        if (lifetime.signal.aborted) return;
        if (result.status === 'found') {
          const citations = result.findings.map(({ path, line }) => `${path}:${line}`).join(', ');
          ctx.ui.notify(`${result.answer}\n\nSources: ${citations}`, 'info');
        } else {
          ctx.ui.notify(result.message, result.status === 'error' ? 'error' : 'info');
        }
      } finally {
        if (!lifetime.signal.aborted) ctx.ui.setStatus('codebase_lookup', undefined);
      }
    },
  });
}
