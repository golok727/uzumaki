---
title: Installation
description: Install the Uzumaki CLI and verify your local environment.
---

Uzumaki apps run through the Uzumaki CLI. Install it first, then use the CLI to scaffold, run, and package apps.

:::caution[Prerelease docs]
These docs target `0.0.1-alpha.2`, which is not released yet. If your installed CLI behaves differently, you may be on an earlier alpha.
:::

## Requirements

- Node.js with `pnpm`
- A working Rust toolchain when developing the runtime from this repository
- The Uzumaki CLI from the install script below

:::note
The generated app uses `pnpm`. Use `bun` only when an existing script in this repository already uses it.
:::

## Install the CLI

### Windows

```powershell
irm https://uzumaki.run/install.ps1 | iex
```

### macOS and Linux

```sh
curl -fsSL https://uzumaki.run/install.sh | sh
```

## Verify the Install

```sh
uzumaki --version
```

If the command is not found, restart your terminal and make sure the installer directory was added to your `PATH`.

## Create an App

```sh
uzumaki init my-app
cd my-app
pnpm install
pnpm dev
```

The dev command starts your app in the native runtime. There is no browser tab to open.

## Common Commands

| Command               | Use it for                                         |
| --------------------- | -------------------------------------------------- |
| `uzumaki init <name>` | Scaffold a new project.                            |
| `uzumaki <entry>`     | Run a TypeScript or bundled JavaScript entry file. |
| `uzumaki build`       | Build and package the configured app.              |
| `uzumaki upgrade`     | Upgrade the installed CLI.                         |

Next, build your first screen in [Quick Start](/guides/quick-start/).
