---
title: Paths & Resources
description: The global Uz.path API for resolving bundled resources and platform directories.
---

:::caution
Uzumaki is in alpha. This API is unstable and may change between releases.
:::

Use `Uz.path` to resolve paths to bundled resources and well-known platform directories.

## Bundled resources

List the files you want shipped with your app under `bundle.resources` in `uzumaki.config.json`. Look them up at runtime with `Uz.path.resource(...)`.

### Declaring resources

```json
{
  "bundle": {
    "resources": ["assets/**/*", "data/config.json"]
  }
}
```

Each entry is a path or glob, relative to the directory containing `uzumaki.config.json`. Matched files are shipped with your app, preserving their layout under the project root.

While developing (`uzumaki run ...`), `Uz.path.resource(...)` reads straight from your project tree, so edits show up without rebuilding.

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

`Uz.path.resource(rel)` returns an absolute filesystem path. Use forward slashes in `rel` — they're normalized to the platform separator.

`<image src>` accepts the returned path directly.

## API

`Uz.path` provides:

| Member          | Returns          | Description                                                                 |
| --------------- | ---------------- | --------------------------------------------------------------------------- |
| `resource(rel)` | `string`         | Absolute path to a bundled resource.                                        |
| `resourceDir`   | `string`         | Root directory holding your bundled resources.                              |
| `identifier`    | `string`         | The app's `identifier` from `uzumaki.config.json` (e.g. `com.example.app`). |
| `cacheDir()`    | `string \| null` | Platform user cache directory.                                              |
| `dataDir()`     | `string \| null` | Platform user data directory.                                               |
| `configDir()`   | `string \| null` | Platform user config directory.                                             |
| `tempDir()`     | `string`         | OS temporary directory.                                                     |
| `exeDir()`      | `string \| null` | Directory containing the running executable.                                |
| `homeDir()`     | `string \| null` | Current user's home directory.                                              |

The platform directories are not namespaced by `identifier` — join it yourself if you want a per-app folder:

```ts
import { join } from 'node:path';

const appCache = join(
  Uz.path.cacheDir() ?? Uz.path.tempDir(),
  Uz.path.identifier,
);
```

## Where resources live

| Mode                 | Resource root                                   |
| -------------------- | ----------------------------------------------- |
| `uzumaki run` (dev)  | Your project directory.                         |
| Packed app (Windows) | A `resources` folder next to the executable.    |
| Packed app (Linux)   | A `resources` folder next to the executable.    |
| Packed app (macOS)   | `Contents/Resources/` inside the `.app` bundle. |

You don't need to think about this in app code — `Uz.path.resource(...)` resolves correctly in every mode.

:::note
Proper installer bundling for Windows (`.msi` / `.exe` installer) and Linux (`.deb` / `.rpm` / AppImage) is planned. Contributions welcome! :D
:::

## Example: icons from a folder

```tsx
export function Icon({ name, size = 16 }: { name: string; size?: number }) {
  const src = Uz.path.resource(`assets/icons/${name}.svg`);
  return <image src={src} w={size} h={size} />;
}
```

With `"resources": ["assets/**/*"]` in `uzumaki.config.json`, the entire `assets/` tree ships with your app, and the same code works in dev and in production.
