---
name: contextor
description: Use when a prompt includes a Contextor feature reference or when adding, updating, or looking up feature annotations in project code or documentation.
---

# Contextor

Contextor links a project's feature identifiers to the code and documentation that explain or implement them. In Pi, a reference like `#{billing}` attaches indexed paths, symbols, and line ranges to the prompt, not the source itself. Use those locations as starting points and read the relevant files before editing.

## Use only confirmed identifiers

**Never invent, rename, or guess a feature identifier.** Run `contextor features` from the target project to list indexed identifiers, then use `contextor query <identifier>` and inspect existing annotations to confirm what a candidate means. Reuse the exact spelling and case. The index can be stale or missing; check project guidance and existing annotations before concluding there is no match.

If no existing identifier clearly applies, multiple identifiers are plausible, or a new one is needed, ask the user which exact identifier to use before adding or changing tags. Leave the annotation unchanged or omit it while waiting; do not add a placeholder. The identifiers below are syntax examples, not defaults.

## Annotate the smallest useful location

Tag the declarations, blocks, and documentation that a future agent should read to understand or change the feature. Reuse the same confirmed identifier across code and docs. Prefer a focused tag over a whole-file tag unless the entire file is about the feature.

- **JavaScript, JSX, TypeScript, TSX:** Use a leading `/** @file-feature billing */` for a whole file or place `/** @feature billing */` directly before a named declaration.
- **Markdown:** Set `feature: billing` (or `feature: [billing, refunds]` for multiple features) in YAML front matter for a whole file. For a section, put `<!-- @feature billing -->` immediately before its heading.
- **Blocks:** Surround the relevant lines with `@feature-start billing` and `@feature-end billing` inside block comments in code or HTML comments in Markdown. To tag a block with multiple identifiers, use a bare `@feature-start`, one `@feature <identifier>` per line in the opening comment, and a bare `@feature-end` in the closing comment. For declarations or sections with multiple features, also use one `@feature` per line.

After changing annotations, run `contextor index` from the project root (or `/contextor:index` in Pi), then query the affected identifiers to check that they point to the intended locations.
