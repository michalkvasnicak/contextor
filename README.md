# @michalkvasnicak/pi-memory

A Pi coding-agent extension that exposes `codebase_lookup` for questions about the current repository: where behavior is implemented, how components fit together, and where related tests live.

The tool currently always returns a miss. It records questions for later review but does not inspect files, index code, or call another model.

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

## Review recorded questions

For persistent Pi sessions, the extension appends the exact `codebase_lookup` question and session metadata to a local log shared across projects: `~/.pi/agent/agent-memory/questions.jsonl` (under `PI_CODING_AGENT_DIR` if set). Each entry includes the time, working directory, session ID and transcript path, and selected model. The log does not copy the transcript or send anything over the network.

```sh
# Questions across all projects
jq -r '[.timestamp, .cwd, .question] | @tsv' "${PI_CODING_AGENT_DIR:-$HOME/.pi/agent}/agent-memory/questions.jsonl"

# Only questions asked from the current working directory
jq --arg cwd "$PWD" 'select(.cwd == $cwd)' "${PI_CODING_AGENT_DIR:-$HOME/.pi/agent}/agent-memory/questions.jsonl"
```

Questions can contain sensitive text. Use `PI_MEMORY_LOG_QUESTIONS=0 pi` to turn off this separate log, or `pi --no-session` to skip both the Pi transcript and this log. Turning logging off does not delete existing entries; delete `questions.jsonl` to clear the shared history. Deleting a Pi session does not clear its questions from this log.

## Development

```sh
pnpm install
pnpm check
pnpm test
pnpm lint
pnpm format:check
```

When you run Pi in this repository, approve the project-local resources to load `@michalkvasnicak/pi-hooks`. It runs `pnpm format`, `pnpm lint`, and `pnpm test` before finishing a turn that changed repository files. The pre-commit hook separately formats staged JavaScript, TypeScript, and JSON files.
