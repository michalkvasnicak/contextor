import type { ExtensionAPI, ExtensionContext } from '@earendil-works/pi-coding-agent';
import { fuzzyFilter, Text } from '@earendil-works/pi-tui';

const MESSAGE_TYPE = 'contextor-features';
const LIST_CACHE_MS = 2_000;
const MAX_SUGGESTIONS = 20;
const COMPLETION_PREFIX = /(?:^|\s)(#([^\s#{}]*))$/u;
const FEATURE_REFERENCE = /#\{([^\s{}]+)\}/gu;

type QueryResult = { locations: string } | { error: string };

export default function (pi: ExtensionAPI): void {
  const draftQueries = new Map<string, Promise<QueryResult>>();

  async function runContextor(cwd: string, args: string[]): Promise<string> {
    const result = await pi.exec('contextor', args, { cwd, timeout: 5_000 });
    if (result.code !== 0) {
      throw new Error(result.stderr.trim() || `contextor exited with code ${result.code}`);
    }
    return result.stdout.trim();
  }

  async function queryFeature(cwd: string, feature: string): Promise<QueryResult> {
    try {
      return { locations: await runContextor(cwd, ['query', '--', feature]) };
    } catch (error) {
      return { error: String(error) };
    }
  }

  async function collectContext(prompt: string, ctx: ExtensionContext) {
    const features = [
      ...new Set(Array.from(prompt.matchAll(FEATURE_REFERENCE), (match) => match[1])),
    ];
    const queries = features.map(
      (feature) => draftQueries.get(feature) ?? queryFeature(ctx.cwd, feature),
    );
    draftQueries.clear();
    const results = await Promise.all(queries);
    const attached: string[] = [];
    const blocks: string[] = [];
    for (const [index, feature] of features.entries()) {
      const result = results[index];
      if ('error' in result) {
        ctx.ui.notify(`Contextor could not query ${feature}: ${result.error}`, 'warning');
        continue;
      }
      attached.push(feature);
      blocks.push(`#{${feature}}\n${result.locations || '(no indexed matches)'}`);
    }
    if (blocks.length > 0) {
      return {
        content: `Contextor feature locations (paths relative to the working directory):\n\n${blocks.join('\n\n')}`,
        features: attached,
      };
    }
  }

  pi.on('session_start', (_event, ctx) => {
    draftQueries.clear();
    if (ctx.mode !== 'tui') return;

    let listRequest: Promise<string[]> | undefined;
    let listExpiresAt = 0;
    async function listFeatures(): Promise<string[]> {
      try {
        const output = await runContextor(ctx.cwd, ['features', '--format', 'json']);
        const features: unknown = JSON.parse(output);
        if (!Array.isArray(features) || !features.every((feature) => typeof feature === 'string')) {
          throw new Error('expected a JSON array of feature identifiers');
        }
        return features;
      } catch (error) {
        ctx.ui.notify(`Contextor could not list features: ${String(error)}`, 'warning');
        return [];
      }
    }

    ctx.ui.addAutocompleteProvider((current) => ({
      triggerCharacters: ['#'],
      async getSuggestions(lines, cursorLine, cursorCol, options) {
        const beforeCursor = (lines[cursorLine] ?? '').slice(0, cursorCol);
        const match = beforeCursor.match(COMPLETION_PREFIX);
        if (!match) return current.getSuggestions(lines, cursorLine, cursorCol, options);

        if (!listRequest || Date.now() >= listExpiresAt) {
          listExpiresAt = Infinity;
          listRequest = listFeatures().finally(() => {
            listExpiresAt = Date.now() + LIST_CACHE_MS;
          });
        }
        const features = await listRequest;
        if (options.signal.aborted) return null;
        const candidates = match[2]
          ? fuzzyFilter(features, match[2], (feature) => feature)
          : features;
        const items = candidates.slice(0, MAX_SUGGESTIONS).map((feature) => ({
          label: feature,
          value: `#{${feature}}`,
        }));
        return items.length > 0 ? { prefix: match[1], items } : null;
      },
      applyCompletion(lines, cursorLine, cursorCol, item, prefix) {
        const feature = prefix.startsWith('#')
          ? item.value.match(/^#\{([^\s{}]+)\}$/u)?.[1]
          : undefined;
        if (feature === undefined) {
          return current.applyCompletion(lines, cursorLine, cursorCol, item, prefix);
        }
        if (!draftQueries.has(feature)) draftQueries.set(feature, queryFeature(ctx.cwd, feature));

        const line = lines[cursorLine] ?? '';
        const start = cursorCol - prefix.length;
        const suffix = line.slice(cursorCol);
        const insertion = suffix ? item.value : `${item.value} `;
        const updated = [...lines];
        updated[cursorLine] = `${line.slice(0, start)}${insertion}${suffix}`;
        return { lines: updated, cursorLine, cursorCol: start + insertion.length };
      },
      shouldTriggerFileCompletion(lines, cursorLine, cursorCol) {
        return current.shouldTriggerFileCompletion?.(lines, cursorLine, cursorCol) ?? true;
      },
    }));
  });

  pi.on('before_agent_start', async (event, ctx) => {
    if (event.messageType !== 'user') return;
    const context = await collectContext(event.prompt, ctx);
    if (context) {
      return {
        message: {
          customType: MESSAGE_TYPE,
          content: context.content,
          details: context.features,
          display: true,
        },
      };
    }
  });

  // Steering and follow-up messages can enter an active run without before_agent_start.
  pi.on('input', async (event, ctx) => {
    if (!event.streamingBehavior) return;
    const context = await collectContext(event.text, ctx);
    if (context) return { action: 'transform', text: `${event.text}\n\n${context.content}` };
  });

  pi.registerMessageRenderer(MESSAGE_TYPE, (message, { expanded, outputPad }, theme) => {
    const features = message.details as string[] | undefined;
    const summary = features?.map((feature) => `#{${feature}}`).join(', ');
    const title = theme.fg(
      'muted',
      summary ? `Contextor: ${summary}` : 'Contextor feature locations',
    );
    const content = expanded && typeof message.content === 'string' ? `\n${message.content}` : '';
    return new Text(`${title}${content}`, outputPad, 0);
  });
}
