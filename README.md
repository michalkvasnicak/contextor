# Contextor

Contextor finds feature annotations in a codebase and lets you attach relevant locations to a Pi prompt. See the [CLI guide](contextor/README.md) for supported annotations and indexing options.

## Use in Pi

Download the CLI archive for your platform from [GitHub Releases](https://github.com/michalkvasnicak/contextor/releases), extract it, and put the `contextor` executable on Pi's `PATH`.

Alternatively, install the CLI from source by running this from the repository root:

```sh
make install
```

From the project you want to work on:

```sh
contextor index
pi -e /absolute/path/to/agent-memory/pi-contextor/extension.ts
```

To keep the extension enabled for that project, use `pi install /absolute/path/to/agent-memory/pi-contextor -l` instead. Omit `-l` to enable it globally. The package also loads the Contextor skill; use `/skill:contextor` for guidance on annotating code and documentation.

Type `#` in the Pi editor, then type part of a feature name and press Tab to select it. Pi inserts a compact reference such as `#{billing}`. Repeat to attach more features, or delete a reference before sending to leave it out. You can also type `#{billing}` directly if you know the exact identifier. The model gets matching paths, symbols, and line ranges, not full source files.

In Pi, use `/contextor:query billing` to show indexed locations for the exact feature `billing` without starting a model turn. Run `/contextor:index` after changing annotations to refresh the index for the current project.

To install development dependencies, run `pnpm -C pi-contextor install` from this checkout.
