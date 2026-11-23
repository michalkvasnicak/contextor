# Repository instructions

- Never run tests, formatters, or linters manually, including check and fix modes. Pi runs them at the end of the turn. If a hook reports a failure, fix the cause and let the hook rerun the check.
- Do not write new tests unless the user explicitly asks for them.
- Write clean, readable, modular code. Use descriptive names, keep functions and modules focused on one responsibility, and avoid abstractions that don't simplify the code.
- Only update `README.md` for user-facing setup or configuration, such as a new setting, default, or usage example needed to configure it. Never use it to explain how the code is organized or how its internals work.
