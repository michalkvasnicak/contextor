# Contextor

Contextor finds feature annotations in a codebase and lets you attach relevant locations to a Pi prompt. See the [CLI guide](contextor/README.md) for supported annotations and indexing options.

## Use in Pi

Install the CLI from the repository root and make sure `contextor` is on Pi's `PATH`:

```sh
make install
```

From the project you want to work on:

```sh
contextor index
pi -e /absolute/path/to/agent-memory/pi-contextor/extension.ts
```

To keep the extension enabled for that project, use `pi install /absolute/path/to/agent-memory/pi-contextor -l` instead. Omit `-l` to enable it globally.

Type `#` in the Pi editor, then type part of a feature name and press Tab to select it. Pi inserts a compact reference such as `#{billing}`. Repeat to attach more features, or delete a reference before sending to leave it out. You can also type `#{billing}` directly if you know the exact identifier. The model gets matching paths, symbols, and line ranges, not full source files. Run `contextor index` again after changing annotations.

To install development dependencies, run `pnpm -C pi-contextor install` from this checkout.
