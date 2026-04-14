# Development Guide

This document explains how to set up a development environment and build Uzumaki from source.

## Architecture Overview

Uzumaki is a monorepo with three Rust crates and TypeScript packages:

```
crates/
  uzumaki/      # Core runtime — Deno runtime, wgpu rendering, Vello 2D graphics, layout (Taffy)
  cli/          # CLI binary — project scaffolding, dev server, build tooling
  refineable/   # Derive macro utility crate
packages/
  playground/   # Example app used for development and testing
```

The runtime embeds a Deno-based JavaScript engine and renders the UI with **wgpu** + **Vello**.

## Prerequisites

### Rust

Uzumaki requires a specific Rust version defined in `rust-toolchain.toml` (currently **1.92.0**). Install Rust via [rustup](https://rustup.rs/) — the correct toolchain will be selected automatically.

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustc -V
cargo -V
```

### Bun (for bundling you can you anything)

- **[Bun](https://bun.sh/)** >= 1.0
- **pnpm** (install via `bun install -g pnpm`, or `npm i -g pnpm`)

### Platform-specific dependencies

The runtime depends on `deno_runtime` and `deno_core`, so you need the native toolchain required to compile those crates.

#### Windows

1. Install [Visual Studio 2019+](https://visualstudio.microsoft.com/downloads/) (Community edition is fine) with the **"Desktop development with C++"** workload:
   - Visual C++ tools for CMake
   - Windows 10/11 SDK
   - C++/CLI support
2. Install the [Protocol Buffers compiler](https://github.com/protocolbuffers/protobuf/releases/latest) and add `protoc` to your `PATH`.
3. Install LLVM/Clang:
   ```powershell
   winget install LLVM.LLVM
   ```
   Make sure `clang` is on your `PATH`.
4. Enable **Developer Mode** in Windows Settings (required for symlink support).

#### macOS

```sh
xcode-select --install    # XCode Command Line Tools
brew install cmake protobuf
```

On Apple Silicon (M1/M2+), also install `llvm` and `lld`:

```sh
brew install llvm lld
# Add /opt/homebrew/opt/llvm/bin/ to $PATH
```

#### Linux (Debian/Ubuntu)

```sh
# LLVM and build tools
wget https://apt.llvm.org/llvm.sh
chmod +x llvm.sh
./llvm.sh 17
apt install --install-recommends -y cmake libglib2.0-dev

# Protobuf
apt install -y protobuf-compiler
```

## Building

### Install JS dependencies

```sh
pnpm install
```

### Build the Rust core

```sh
pnpm build:core
# or directly:
cargo build --release -p uzumaki
```

### Run the playground

```sh
pnpm start
# or separately:
pnpm build:core
pnpm --filter playground dev
```

### Quick iteration

If you're only changing TypeScript/JSX code in the playground:

```sh
pnpm --filter playground dev
```

If you're changing Rust code, rebuild the core first:

```sh
cargo build --release -p uzumaki && pnpm --filter playground dev
```

### Compile checks (no binary output)

```sh
cargo check                   # Full workspace
cargo check -p uzumaki_runtime  # Just the runtime crate
```

## Code Quality

### Formatting

```sh
pnpm format
cargo fmt
```

### Linting

```sh
pnpm lint                 # oxlint for JS/TS
pnpm lint:fix             # oxlint with auto-fix
cargo clippy --workspace  # clippy for Rust
```

Formatting and linting also run automatically on commit via `husky` + `lint-staged`.

## Project Structure

| Path                   | Description                                                      |
| ---------------------- | ---------------------------------------------------------------- |
| `crates/uzumaki/src/`  | Rust runtime — window management, rendering pipeline, event loop |
| `crates/uzumaki/core/` | Core abstractions (elements, layout, styling)                    |
| `crates/uzumaki/js/`   | JavaScript modules loaded into the Deno runtime                  |
| `crates/cli/src/`      | CLI entry point — `init`, `dev`, `build` commands                |
| `crates/cli/template/` | Project template used by `uzumaki init`                          |
| `crates/refineable/`   | Proc-macro crate for the `Refineable` derive macro               |
| `packages/playground/` | Development playground app                                       |
| `docs/`                | Documentation site                                               |

## Debugging

### Rust

```sh
# Full backtraces
RUST_BACKTRACE=1 cargo run -p uzumaki --release -- src/index.tsx

# Debug build (faster compile, slower runtime)
cargo run -p uzumaki -- src/index.tsx
```

## Troubleshooting

### `aws-lc-rs` / `rustls` build failures

This usually means LLVM/Clang is not found. Make sure `clang` is on your PATH. On Windows, install LLVM via `winget install LLVM.LLVM`.

### Linker errors on Windows

Ensure Visual Studio with the C++ workload is installed. The Windows SDK and MSVC build tools must be available.

### Slow builds

- Use `cargo check` instead of `cargo build` when you only need to verify compilation.
- Consider installing [sccache](https://github.com/mozilla/sccache) for caching compiled artifacts.
- On Linux, [mold](https://github.com/rui314/mold) can significantly speed up linking.
