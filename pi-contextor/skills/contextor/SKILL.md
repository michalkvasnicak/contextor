---
name: contextor
description: Use when a prompt includes a Contextor feature reference or when adding, updating, or looking up feature annotations in project code or documentation.
---

# Contextor

Contextor links a project's feature identifiers to the code and documentation that explain or implement them. In Pi, a reference like `#{billing}` attaches indexed paths, symbols, and line ranges to the prompt, not the source itself. Use those locations as starting points and read the relevant files before editing.

## Use only confirmed identifiers

**Never invent, rename, or guess a feature identifier.** Run `contextor features` from the target project to list indexed identifiers, then use `contextor query <identifier>` and inspect existing annotations to confirm what a candidate means. Reuse the exact spelling and case. The index can be stale, missing, or incomplete; check project guidance and existing annotations before concluding there is no match.

If no existing identifier clearly applies, multiple identifiers are plausible, or a new one is needed, ask the user which exact identifier to use before adding or changing tags. Leave the annotation unchanged or omit it while waiting; do not add a placeholder. `billing` and `refunds` below are syntax examples, not defaults.

## Annotate the smallest useful location

Tag the declarations, blocks, and documentation that a future agent should read to understand or change the feature. Reuse the same confirmed identifier across code and docs. Prefer a focused tag over a whole-file tag unless the entire file is about the feature.

- **Code:** Use the language's native comment syntax and follow existing project conventions. Put `@feature billing` in a doc comment immediately above a named declaration, or `@file-feature billing` in a leading comment if the whole file belongs to the feature. For a block, use comments containing `@feature-start billing` and `@feature-end billing` around the relevant lines.
- **Markdown:** Put `<!-- @feature billing -->` immediately before a heading to tag its section. For a whole file, use `feature: billing` in YAML front matter, or `feature: [billing, refunds]` for multiple features. Block markers go in HTML comments.
- **Multiple features:** Use one `@feature` per line in a declaration or section comment. For a block, use a bare `@feature-start` followed by one `@feature` per line in the opening comment, then a bare `@feature-end` in the closing comment.

These conventions apply even when Contextor has no parser for the language. A valid source comment can help readers without being indexed. When code isn't indexed, tag a relevant Markdown section and link to the source files and name the key symbols there so the feature is still discoverable.

After changing annotations, run `contextor index` from the project root (or `/contextor:index` in Pi), then query the affected identifiers to verify which locations were indexed.
