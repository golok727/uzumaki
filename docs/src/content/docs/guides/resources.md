---
title: Load Images and Resources
description: Bundle assets and resolve them at runtime with Uz.path.
---

Desktop apps need predictable asset paths in development and packaged builds. Uzumaki solves this with `bundle.resources` and `Uz.path.resource(...)`.

## Add Resources to the Config

```json
{
  "bundle": {
    "resources": ["assets/**/*", "data/**/*.json"]
  }
}
```

Files matched by these globs are copied into the packaged app.

## Resolve a Bundled File

```tsx
const logo = Uz.path.resource('assets/logo.svg');

<image src={logo} w={96} h={96} />;
```

`resource(rel)` returns an absolute path. The same call works in development and in packaged output.

## Load Remote Images

```tsx
<image
  src="https://example.com/hero.png"
  w={420}
  h={240}
  rounded={16}
  onLoadStart={() => setStatus('loading')}
  onLoad={() => setStatus('ready')}
  onError={(event) => setStatus(event.message)}
/>
```

`<image src>` accepts bundled paths, absolute paths, `file://` URLs, and `https://` URLs.

## Read App Directories

Use `Uz.path` for platform directories:

```ts
const dataDir = Uz.path.dataDir() ?? Uz.path.tempDir();
const cacheDir = Uz.path.cacheDir();
const configDir = Uz.path.configDir();
```

Some platform directories can be `null`, so keep a fallback for writable paths.

## Store Per-App Data

```ts
import { join } from 'node:path';

const appData = join(
  Uz.path.dataDir() ?? Uz.path.tempDir(),
  Uz.path.identifier,
);
```

`identifier` comes from your app config and is useful for namespacing app files.
