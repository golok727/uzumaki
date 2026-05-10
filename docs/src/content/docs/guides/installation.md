---
title: Installation
description: Install the Uzumaki CLI and verify your local environment.
---

Uzumaki apps run through the Uzumaki CLI. Install it first, then use the CLI to scaffold, run, and package apps.

:::caution[Prerelease docs]
These docs target `0.0.1-alpha.2`, which is not released yet. If your installed CLI behaves differently, you may be on an earlier alpha.
:::

## Requirements

The Uzumaki CLI is the only requirement. Uzumaki is built on Deno, so it ships a full JavaScript and TypeScript runtime. You do not need Node.js, Deno, or Bun installed to run apps. To install app dependencies like React you need any package manager (`pnpm`, `npm`, `yarn`, or `bun` all work — examples below use `pnpm`).

## Install the CLI

### Windows

```sh
powershell irm https://uzumaki.run/install.ps1 | iex
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

The dev command starts your app in the native runtime. There is no browser tab to open. Swap `pnpm` for `npm`, `yarn`, or `bun` if you prefer.

## Common Commands

| Command               | Use it for                                         |
| --------------------- | -------------------------------------------------- |
| `uzumaki init <name>` | Scaffold a new project.                            |
| `uzumaki <entry>`     | Run a TypeScript or bundled JavaScript entry file. |
| `uzumaki build`       | Build and package the configured app.              |
| `uzumaki upgrade`     | Upgrade the installed CLI.                         |

Next, build your first screen in [Quick Start](/guides/quick-start/).
