---
title: Configure an App
description: Configure builds, packaging, resources, and app identity.
---

`uzumaki.config.json` describes how the CLI should build and package your app.

Most projects need three things:

- App identity: name, version, and identifier.
- Build command: how to turn TypeScript into JavaScript.
- Package settings: where the bundled JavaScript lives and what executable to create.

## Example Config

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

## App Identity

| Field         | Description                                       |
| ------------- | ------------------------------------------------- |
| `productName` | Human-readable app name.                          |
| `version`     | App version.                                      |
| `identifier`  | Reverse-DNS app id, such as `com.example.my_app`. |

Use a stable identifier early. It is also exposed at runtime as `Uz.path.identifier`.

## Build Settings

```json
{
  "build": {
    "command": "bun build src/index.tsx --target node --outdir dist --minify --external uzumaki"
  }
}
```

`build.command` runs before packaging. It should write JavaScript into `pack.jsDist`.

Always mark `uzumaki` as external:

```sh
bun build src/index.tsx --target node --outdir dist --minify --external uzumaki
```

The runtime provides the `uzumaki` module at execution time. Bundling it into your app would produce the wrong module.

:::note
Vite support is planned. For now, use the scaffolded build command or another bundler that can externalize `uzumaki`.
:::

## Package Settings

| Field            | Description                                 |
| ---------------- | ------------------------------------------- |
| `pack.jsDist`    | Directory containing built JavaScript.      |
| `pack.entry`     | Entry file inside `jsDist`.                 |
| `pack.outputDir` | Directory where packaged output is written. |
| `pack.binName`   | Executable name.                            |

## Resources

Declare resources:

```json
{
  "bundle": {
    "resources": ["assets/**/*", "data/**/*.json"]
  }
}
```

Resolve them:

```tsx
const icon = Uz.path.resource('assets/icons/search.svg');
const config = Uz.path.resource('data/app.json');
```

`<image src>` accepts bundled paths, absolute paths, `file://` URLs, and `https://` URLs.

## CLI Flags

```sh
uzumaki build --config path/to/uzumaki.config.json
uzumaki build --no-build
```

Use `--no-build` in CI when your JavaScript bundle has already been produced.
