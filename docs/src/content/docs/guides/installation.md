---
title: Installation
description: Install the Uzumaki CLI.
---

## Requirements

- **macOS** (Intel or Apple Silicon) or **Windows** (x64 or ARM64)
- Linux support is coming soon

## Install

### macOS / Linux

```sh
curl -fsSL https://uzumaki.run/install.sh | sh
```

This downloads the latest release and installs it to `~/.uzumaki/bin`. Follow the instructions to add it to your PATH.

You can pin a specific version:

```sh
UZUMAKI_VERSION=0.1.0 curl -fsSL https://uzumaki.run/install.sh | sh
```

Or change the install location:

```sh
UZUMAKI_INSTALL=/usr/local/bin curl -fsSL https://uzumaki.run/install.sh | sh
```

### Windows

Open PowerShell and run:

```powershell
irm https://uzumaki.run/install.ps1 | iex
```

This installs to `%USERPROFILE%\.uzumaki\bin` and adds it to your PATH automatically. You may need to restart your terminal.

Pin a version:

```powershell
$env:UZUMAKI_VERSION="0.1.0"; irm https://uzumaki.run/install.ps1 | iex
```

### Verify

```sh
uzumaki --version
```

## Upgrade

```sh
uzumaki upgrade
```

Or upgrade to a specific version:

```sh
uzumaki upgrade --version 0.2.0
```
