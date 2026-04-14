# Contributing to Uzumaki

Thanks for your interest in contributing to Uzumaki! This guide will help you get started.

## AI Usage

Using AI tools (Copilot, Claude, ChatGPT, Cursor, etc.) is totally fine. However, you are responsible for the code you submit. Low-effort, AI-generated slop PRs — fake outputs, fabricated fixes, PRs that clearly haven't been tested or reviewed by a human — will be closed and repeat offenders will be banned.

## Getting Started

1. Fork the repository and clone it:

   ```sh
   git clone https://github.com/<your-username>/uzumaki.git
   cd uzumaki
   ```

2. Set up your development environment by following [DEVELOPMENT.md](DEVELOPMENT.md).

3. Create a new branch for your work:
   ```sh
   git checkout -b feature/my-feature
   ```

## Making Changes

### Before You Start

- Check existing [issues](https://github.com/nicecapj/uzumaki/issues) and [pull requests](https://github.com/nicecapj/uzumaki/pulls) to avoid duplicating work.
- For larger changes, open an issue first to discuss the approach.

### Code Style

- **Rust:** Run `cargo fmt` and `cargo clippy --workspace` before committing. The project uses Rust 2024 edition.
- **TypeScript/JavaScript:** Run `pnpm format` and `pnpm lint:fix` before committing.
- Formatting and linting are enforced by pre-commit hooks (`husky` + `lint-staged`), so most of this happens automatically.

### Commit Messages

Write clear, concise commit messages. Use conventional commit prefixes:

- `feat:` — new feature
- `fix:` — bug fix
- `refactor:` — code restructuring without behavior change
- `docs:` — documentation changes
- `chore:` — tooling, CI, dependencies
- `test:` — adding or updating tests

Example: `feat: add border-radius support for text elements`

## Submitting a Pull Request

1. Make sure your code compiles and passes checks:

   ```sh
   cargo check --workspace
   cargo clippy --workspace
   cargo fmt --check
   pnpm lint
   ```

2. Push your branch and open a PR against `main`.

3. In the PR description:
   - Summarize what the PR does and why.
   - Include screenshots or recordings for UI changes.
   - Link related issues (e.g., "Closes #42").

4. Keep PRs focused — avoid unrelated drive-by changes. If you spot something else that needs fixing, open a separate PR.

5. Don't force-push after review has started. Add new commits instead — they get squashed on merge.

## Project Structure

See [DEVELOPMENT.md](DEVELOPMENT.md#project-structure) for a breakdown of the codebase.

### Where to Contribute

- **Layout/styling:** `crates/uzumaki/core/` — element styling, Taffy layout integration
- **Rendering:** `crates/uzumaki/src/` — wgpu/Vello rendering pipeline
- **JS runtime:** `crates/uzumaki/js/` — JavaScript modules and React reconciler bridge
- **CLI tooling:** `crates/cli/src/` — `uzumaki init`, `dev`, `build` commands
- **Documentation:** `docs/`
- **Examples:** `packages/playground/`

## Reporting Bugs

When filing a bug report, include:

- OS and version
- Uzumaki version (`uzumaki -V`)
- Rust version (`rustc -V`)
- Steps to reproduce
- Expected vs actual behavior
- Error output or screenshots if applicable

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
