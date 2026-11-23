import type { ExtensionAPI } from '@earendil-works/pi-coding-agent';
import { Type } from 'typebox';

export default function (pi: ExtensionAPI) {
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

    async execute() {
      const result = {
        status: 'miss',
        message: 'No codebase memory is available. Continue by inspecting the source.',
        findings: [],
      };
      return {
        content: [{ type: 'text', text: JSON.stringify(result) }],
        details: {},
      };
    },
  });
}
