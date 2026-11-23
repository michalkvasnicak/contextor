# @michalkvasnicak/pi-memory

A Pi coding-agent extension that exposes `codebase_lookup` for questions about the current repository: where behavior is implemented, how components fit together, and where related tests live.

This first version always returns a miss. It does not inspect files, index code, log queries, write anything, or call another model. Pi can still include tool calls and their arguments in its normal session transcript.

## Try it locally

From any project, load the TypeScript extension for one session (no build required):

```sh
pi -e /absolute/path/to/agent-memory/src/extension.ts
```

To keep it enabled for just the current project, run this from that project, then start Pi and approve the project's local resources:

```sh
pi install /absolute/path/to/agent-memory -l
```

Omit `-l` to enable it across your projects.

Ask Pi to call `codebase_lookup` with “Where is request routing implemented, and which tests cover it?” to check that it loads. To see whether Pi chooses the tool on its own, start a fresh session and give it a normal coding task without mentioning the tool. For a no-transcript run, start Pi with `--no-session`.

## Development

```sh
pnpm install
pnpm check
pnpm test
pnpm lint
pnpm format:check
```

When you run Pi in this repository, approve the project-local resources to load `@michalkvasnicak/pi-hooks`. It runs `pnpm format`, `pnpm lint`, and `pnpm test` before finishing a turn that changed repository files. The pre-commit hook separately formats staged JavaScript, TypeScript, and JSON files.
