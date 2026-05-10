# Agent Rules

## Package Manager

Use pnpm for all package management commands (not npm or yarn).

Exception: end-user install instructions should use `npm install -g` (global) or `npm install -D` (project dev dependency) since npm is universal.

## Dependencies

Always check for the latest npm version when adding dependencies. Use `pnpm add <package>` (without version) to get the latest, or verify with `npm view <package> version` first.

## No Emojis

Do not use emojis anywhere in this repository — code, comments, output, or docs.

## Dashes

Never use `--` as a dash in prose, comments, or user-facing output. Use an em dash (—) when a dash is needed, but prefer rephrasing to avoid dashes entirely. The only exception is CLI flags (e.g. `--port`).

## Boolean Environment Variables

Document boolean env vars using only `0` and `1` in CLI help, docs, and README. Code may accept `true`/`false` as well, but these alternatives are not documented.

## Coding Style

Rust follows standard `rustfmt` conventions: 4-space indentation, `snake_case` modules/functions, `PascalCase` types. TypeScript and TSX use Prettier with semicolons and single quotes; 2-space indentation.

### Rust 2024 idioms

Write idiomatic Rust 2024. In particular:

- Use `if let` chains instead of nested `if let` blocks. Rust 2024 stabilized `if let` chains — prefer `if let Some(x) = foo && condition` over nesting.
- Prefer `let else` for early returns over nested `if let` when the happy path should continue flat.
- Do not write nested `if` blocks where a single `if let ... && ...` chain would do.

When implementing changes:

- Ask when you have real doubt instead of making risky assumptions
- Keep code easy to test and easy to change later
- Avoid overcomplicated abstractions
- If there is a genuinely better and simpler fix, do that instead of patching around the problem
- Use comments sparingly and only when they add real value

## Validation

**Prefer `cargo check` over `cargo build`** for verifying Rust changes — it is faster and sufficient for catching type and borrow errors.

When you need to check Rust code, run against the runtime crate:

```
cargo check -p uzumaki_runtime
```

The `uzumaki` crate is the CLI. Only check or build it when the change touches CLI code specifically. Do not default to checking the whole workspace or building unless there is a concrete reason.

Add focused tests near the code you change when it makes sense. Rust unit tests live inline in `mod tests` blocks near the relevant module.

## Documentation

After implementing a notable feature, update the docs when needed — especially for user-facing API changes: new JS APIs, new JSX props, new events, or changes in expected playground usage.
