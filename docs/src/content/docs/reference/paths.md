---
title: Paths & Resources
description: The global Uz.path API for resolving bundled resources and platform directories.
---

:::caution
Uzumaki is in alpha. This API is unstable and may change between releases.
:::

Uzumaki exposes a global `Uz` object inside your app's runtime. Today it carries a single namespace, `Uz.path`, which resolves paths to bundled resources and well-known platform directories.

## Bundled resources

Files declared under `bundle.resources` in `uzumaki.config.json` are staged next to the packed executable at build time and resolvable at runtime via `Uz.path.resource(...)`.

### Declaring resources

```json
{
  "bundle": {
    "resources": ["assets/**/*", "data/config.json"]
  }
}
```

Each entry is a path or glob, relative to the directory containing `uzumaki.config.json`. Matched files are copied to `<output_dir>/resources/<relative_path>`, preserving their layout under the project root.

In dev (`uzumaki run ...`) no copy happens — `Uz.path.resource(...)` reads straight from the project tree, so edits are picked up without rebuilding.

### Resolving at runtime

```tsx
import { Window } from 'uzumaki-ui';
import { render } from 'uzumaki-ui/react';

const logo = Uz.path.resource('assets/logo.svg');

function App() {
  return <image src={logo} w={64} h={64} />;
}

const win = new Window({ title: 'Hello' });
render(<App />, win);
```

`Uz.path.resource(rel)` returns an absolute filesystem path — it does no I/O and performs no traversal sanitation. Use forward slashes in `rel`; they're normalized to the platform separator.

`<image src>` accepts the returned path directly (drive paths and `file:` URLs both work).

## API

`Uz.path` provides:

| Member          | Returns          | Description                                                                              |
| --------------- | ---------------- | ---------------------------------------------------------------------------------------- |
| `resource(rel)` | `string`         | Absolute path under the resource root for the given relative path.                       |
| `resourceDir`   | `string`         | Absolute path of the resource root itself.                                               |
| `identifier`    | `string`         | The app's `identifier` from `uzumaki.config.json` (e.g. `com.example.app`).              |
| `cacheDir()`    | `string \| null` | Platform user cache dir (`%LOCALAPPDATA%`, `~/Library/Caches`, `$XDG_CACHE_HOME`).       |
| `dataDir()`     | `string \| null` | Platform user data dir (`%APPDATA%`, `~/Library/Application Support`, `$XDG_DATA_HOME`). |
| `configDir()`   | `string \| null` | Platform user config dir.                                                                |
| `tempDir()`     | `string`         | OS temporary directory.                                                                  |
| `exeDir()`      | `string \| null` | Directory containing the running executable.                                             |
| `homeDir()`     | `string \| null` | Current user's home directory.                                                           |

The platform directories are not namespaced by `identifier` — join it yourself if you want a per-app folder:

```ts
import { join } from 'node:path';

const appCache = join(
  Uz.path.cacheDir() ?? Uz.path.tempDir(),
  Uz.path.identifier,
);
```

## Where resources live

| Mode                    | Resource root                                              |
| ----------------------- | ---------------------------------------------------------- |
| `uzumaki run` (dev)     | The directory containing `uzumaki.config.json`.            |
| Packed binary (Windows) | `<exe_dir>\resources\`                                     |
| Packed binary (Linux)   | `<exe_dir>/resources/`                                     |
| Packed binary (macOS)   | `<exe_dir>/resources/` (proper `.app` bundling is planned) |

## Example: icons from a folder

```tsx
export function Icon({ name, size = 16 }: { name: string; size?: number }) {
  const src = Uz.path.resource(`assets/icons/${name}.svg`);
  return <image src={src} w={size} h={size} />;
}
```

With `"resources": ["assets/**/*"]` in `uzumaki.config.json`, the entire `assets/` tree is staged next to the exe, and the same code works in dev and in production.
