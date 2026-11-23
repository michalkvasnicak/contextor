# @michalkvasnicak/pi-memory

A Pi coding-agent extension that exposes `codebase_lookup` for questions about the current repository: where behavior is implemented, how components fit together, and where related tests live.

The extension records questions for later review. Without a configured lookup model, the tool returns a miss. With a model configured, it can search and read the current repository without editing files or running shell commands.

## Try it locally

Run `pnpm install` in the extension checkout first. Then, from any project, load the TypeScript extension for one session (no build required):

```sh
pi -e /absolute/path/to/agent-memory/src/extension.ts
```

To keep it enabled for just the current project, run this from that project, then start Pi and approve the project's local resources:

```sh
pi install /absolute/path/to/agent-memory -l
```

Omit `-l` to enable it across your projects.

Ask Pi to call `codebase_lookup` with “Where is request routing implemented, and which tests cover it?” to check that it loads. To see whether Pi chooses the tool on its own, start a fresh session and give it a normal coding task without mentioning the tool. For a no-transcript run, start Pi with `--no-session`.

## Configure live lookup

Install `fff-mcp` and make sure it is on Pi's `PATH`. Configure the answerer model in Pi's user model catalog (for example, a local OpenAI-compatible MLX endpoint in `models.json`), then create `$PI_CODING_AGENT_DIR/agent-memory/readcode.json` (default: `~/.pi/agent/agent-memory/readcode.json`):

```json
{
  "model": "local-mlx/qwen3.6-35b-a3b",
  "thinkingLevel": "off",
  "timeoutMs": 20000,
  "recordOnly": true
}
```

Replace `model` with your configured `provider/modelId`. The model must support tool calls; it receives the question and relevant repository excerpts. Use a local endpoint if source must stay on your machine. Settings are read at each lookup. The defaults are thinking off, a 20-second timeout, and `recordOnly: false`.

In `recordOnly` mode, the main agent's `codebase_lookup` tool logs the question and returns a miss without contacting the answerer. Type `/codebase_lookup Where is request routing implemented?` to run the live lookup anyway. Both entry points record questions; the extension does not save answers to its database. Tool results may still appear in the normal Pi transcript.

## Review recorded questions

For persistent Pi sessions, the extension stores the exact `codebase_lookup` question and session metadata in a local SQLite database shared across projects: `~/.pi/agent/agent-memory/questions.sqlite` (under `PI_CODING_AGENT_DIR` if set). Each entry includes the time, working directory, session ID and transcript path, and selected model. The log does not copy the transcript or send anything over the network. Existing `questions.jsonl` logs are left untouched and are not imported.

```sh
sqlite3 -readonly -header -column "${PI_CODING_AGENT_DIR:-$HOME/.pi/agent}/agent-memory/questions.sqlite" \
  'SELECT timestamp, cwd, question FROM questions ORDER BY id DESC LIMIT 20;'
```

Questions can contain sensitive text. Use `PI_MEMORY_LOG_QUESTIONS=0 pi` to turn off this separate log, or `pi --no-session` to skip both the Pi transcript and this log. Turning logging off does not delete existing entries; close any running Pi sessions and delete `questions.sqlite` to clear the shared history. Deleting a Pi session does not clear its questions from this log.

## Development

```sh
pnpm install
pnpm check
pnpm test
pnpm lint
pnpm format:check
```

When you run Pi in this repository, approve the project-local resources to load `@michalkvasnicak/pi-hooks`. It runs `pnpm format`, `pnpm lint`, and `pnpm test` before finishing a turn that changed repository files. The pre-commit hook separately formats staged JavaScript, TypeScript, and JSON files.
