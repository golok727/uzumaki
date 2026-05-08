---
title: Installation
description: Install the Uzumaki CLI.
---

:::note
Docs are a work in progress.
:::

Windows:

```powershell
irm https://uzumaki.run/install.ps1 | iex
```

macOS / Linux:

```sh
curl -fsSL https://uzumaki.run/install.sh | sh
```

Verify:

```sh
uzumaki --version
```

Upgrade:

```sh
uzumaki upgrade
```

## Commands

- `uzumaki init <name>` — scaffold a project
- `uzumaki <entry>` — run an app
- `uzumaki build` — package for distribution

Next: [Quick Start](/guides/quick-start/).
