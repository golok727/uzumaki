---
title: Configuration
description: uzumaki.config.json reference.
---

`uzumaki.config.json`:

```json
{
  "productName": "my-app",
  "version": "0.1.0",
  "identifier": "com.example.my_app",
  "build": {
    "command": "bun build src/index.tsx --target node --outdir dist --minify --external uzumaki"
  },
  "pack": {
    "jsDist": "./dist",
    "entry": "index.js",
    "outputDir": "./target",
    "binName": "my-app"
  },
  "bundle": {
    "resources": ["assets/**/*"]
  }
}
```

## Fields

- `productName` — display name
- `version` — app version
- `identifier` — reverse-DNS id (e.g. `com.example.my_app`)
- `build.command` — bundler command
- `pack.jsDist` — bundler output folder
- `pack.entry` — entry file inside `jsDist`
- `pack.outputDir` — packaged output location
- `pack.binName` — executable name
- `bundle.resources` — globs of files shipped with the app

## Always external

`uzumaki` is provided by the runtime. Mark it external in your bundler:

```sh
bun build src/index.tsx --target node --outdir dist --minify --external uzumaki
```

> Vite support coming soon.

## Resources

```json
{ "bundle": { "resources": ["assets/**/*", "data/**/*.json"] } }
```

```tsx
const icon = Uz.path.resource('assets/icons/search.svg');
const config = Uz.path.resource('data/app.json');
```

`<image src>` accepts bundled paths, absolute paths, `file://`, and `https://`.

## CLI flags

```sh
uzumaki build --config path/to/uzumaki.config.json
uzumaki build --no-build
```
