---
title: Paths and Resources
description: Resolve bundled resources and platform directories.
---

`Uz.path` is a global runtime API for files and platform directories.

```ts
const logo = Uz.path.resource('assets/logo.svg');
```

## Bundled Resources

Declare files in `uzumaki.config.json`:

```json
{
  "bundle": {
    "resources": ["assets/**/*", "data/app.json"]
  }
}
```

Resolve them at runtime:

```tsx
const logo = Uz.path.resource('assets/logo.svg');
const config = Uz.path.resource('data/app.json');

<image src={logo} w={96} h={96} />;
```

`resource(rel)` returns an absolute path. The same call works in development and packaged builds.

## API

| API             | Description                                     |
| --------------- | ----------------------------------------------- |
| `resource(rel)` | Resolve a bundled resource to an absolute path. |
| `resourceDir`   | Bundled resource root.                          |
| `identifier`    | App id from config.                             |
| `cacheDir()`    | Platform cache directory, or `null`.            |
| `dataDir()`     | Platform data directory, or `null`.             |
| `configDir()`   | Platform config directory, or `null`.           |
| `tempDir()`     | Writable temp directory.                        |
| `exeDir()`      | Directory of the running executable, or `null`. |
| `homeDir()`     | User home directory, or `null`.                 |

## Per-App Data Folder

```ts
import { join } from 'node:path';

const appData = join(
  Uz.path.dataDir() ?? Uz.path.tempDir(),
  Uz.path.identifier,
);
```

Use `tempDir()` as a fallback because some platform directories may be unavailable.

## Image Sources

`<image src>` accepts:

- Paths returned by `Uz.path.resource(...)`
- Absolute file paths
- `file://` URLs
- `https://` URLs

Prefer bundled resources for app-owned assets such as icons, illustrations, and seed data.
