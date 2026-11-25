# Contextor

Install the CLI from this checkout with a recent stable Rust toolchain and a C compiler:

```sh
cargo install --locked --path indexer
```

Run it from the directory you want to search:

```sh
contextor index
contextor features
contextor features --format json
contextor query billing
contextor query billing --format json --limit 10
```

`contextor index` creates or incrementally updates the current directory's index and exits. Queries are read-only and look up an exact, case-sensitive identifier in the last completed index. Run `contextor index` again after changing source files. Each project has a database at `~/.contextor/<slug>-<path-hash>/index.sqlite`. The slug and hash come from the absolute project path, so different directories with the same name get separate databases. The scan respects ignore rules and skips hidden paths and `node_modules`.

`contextor features` lists all distinct identifiers in sorted order from the existing index. It prints one identifier per line by default, or a JSON array of strings with `--format json`.

Query results are capped at 30 by default. Text uses one line per result:

```text
docs/billing.md
src/billing.ts:12-24 charge
src/refunds.ts:31-36
```

JSON returns an array with `path` and, where applicable, `symbolName`, `startLine`, and `endLine`. Paths are relative to the queried directory; line ranges are one-based and inclusive.

## Mark features

In JavaScript, JSX, TypeScript, and TSX, a leading header comment can tag the whole file, and a doc comment can tag the next named declaration:

```ts
/** @file-feature billing */

/**
 * @feature billing
 * @feature refunds
 */
export function issueRefund() {
  /*
   * @feature-start
   * @feature billing
   * @feature refunds
   */
  validate();
  record();
  /* @feature-end */
}
```

Member matches include their owner in the name (for example, `Invoice.refund` or `PaymentGateway.provider`). Their line ranges still point to the member itself.

Use one identifier per tag. For a single-feature block, `@feature-start billing` and `@feature-end billing` also work. Ranges exclude the marker comments. A file tag does not implicitly tag every symbol in that file.

In Markdown, use YAML front matter for the whole file and HTML comments for sections or blocks:

```markdown
---
feature: [billing, refunds]
---

<!-- @feature billing -->

## Invoices

<!-- @feature-start refunds -->

Refund details go here.
<!-- @feature-end refunds -->
```

Front matter also accepts a single value, such as `feature: billing`. Repeat `@feature` on separate lines in an HTML comment to tag the same section with multiple identifiers. A tag comment immediately before a heading applies to that section; elsewhere it matches the comment's own range. Fenced examples and inline code are ignored.
