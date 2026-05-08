---
title: Paths
description: Resolve bundled resources and platform directories.
---

`Uz.path` resolves files — bundled resources or platform directories.

## Bundled resources

Declare them:

```json
{ "bundle": { "resources": ["assets/**/*", "data/app.json"] } }
```

Use them:

```tsx
const logo = Uz.path.resource('assets/logo.svg');
const config = Uz.path.resource('data/app.json');

<image src={logo} w={96} h={96} />;
```

`resource(rel)` returns an absolute path. Same call works in dev and packaged builds.

## API

- `resource(rel)` — bundled resource → absolute path
- `resourceDir` — bundled resource root
- `identifier` — app id from config
- `cacheDir()`, `dataDir()`, `configDir()` — platform dirs (may be `null`)
- `tempDir()` — writable temp dir
- `exeDir()` — directory of the running executable
- `homeDir()` — user home

## Per-app data folder

```ts
import { join } from 'node:path';

const appData = join(
  Uz.path.dataDir() ?? Uz.path.tempDir(),
  Uz.path.identifier,
);
```
