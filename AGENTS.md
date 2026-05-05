# Repository Guidelines

## Project Structure & Module Organization

This repository is a mixed Rust and TypeScript workspace for the Uzumaki UI runtime.

- `crates/uzumaki/`: main runtime, renderer, event system, TypeScript bridge, and React JSX runtime
- `crates/refineable/`: shared Rust support crate plus `derive_refineable/` proc-macro code
- `packages/playground/`: the main TypeScript playground app
- `scripts/`: small repo utilities

Build artifacts land in `target/`. Top-level config lives in `Cargo.toml`, `package.json`, `pnpm-workspace.yaml`, and `tsconfig.json`.

## Framework Notes

For the playground TypeScript app, use the current `uzumaki-ui` crate and do not assume any built-in module has landed yet.

- Import `Window` and runtime APIs from `uzumaki-ui`
- Import `render` from `uzumaki-ui/react`
- Follow the JSX setup already used by the repo: `jsx: react-jsx` with `jsxImportSource: uzumaki-ui/react`
- Use the Uzumaki intrinsic elements and prop names that already exist in the repo instead of inventing DOM-style props

Check the docs and in-tree types before writing playground code:

- `crates/uzumaki/README.md`
- `crates/uzumaki/js/react/jsx/types.ts`
- existing examples in `packages/playground/src/`

Keep this file focused on repo-specific syntax and behavior. For broader framework usage, read the docs instead of guessing.

## Build, Test, and Development Commands

- `pnpm start`: runs the playground through the native runtime
- `cargo build --release -p uzumaki`: builds the desktop runtime
- `cargo check`: required after Rust changes
- `cargo test -p uzumaki`: runs the Rust unit tests
- `pnpm format`: formats Markdown, TS, JS, and JSON with Prettier

Use `pnpm` for workspace scripts, `cargo` for Rust work, and `bun` only where an existing package script already uses it.

## Coding Style & Engineering Expectations

Rust follows standard `rustfmt` conventions: 4-space indentation, `snake_case` modules/functions, and `PascalCase` types. TypeScript and TSX use Prettier with semicolons and single quotes; current files use 2-space indentation.

When implementing changes:

- Ask questions when you have real doubt instead of making risky assumptions
- Keep code easy to test and easy to change later
- Avoid overcomplicated abstractions
- If there is a genuinely better and simpler fix, do that instead of adding fake fixes or workarounds
- Use comments sparingly and only when they add real value

## Validation Workflow

After writing or modifying Rust code, always run:

- `cargo check`

Add focused tests near the code you change when it makes sense. Rust unit tests should live inline in `mod tests` blocks near the relevant module.

There is no root JS test runner configured yet, so playground changes are usually manual-test territory unless the change introduces or updates coverage.

## Documentation Expectations

After implementing a notable feature, update the docs when needed. This is especially important for user-facing API changes such as new JS APIs, new JSX props, new events, or changes in expected playground usage.

## Commit & Pull Request Guidelines

Keep commits brief, specific, and scoped to one change. PRs should explain the behavior change, note affected crates or packages, and include screenshots or recordings for UI-facing playground changes.
