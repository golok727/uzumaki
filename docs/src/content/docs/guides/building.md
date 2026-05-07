---
title: Building Your App
description: Package your Uzumaki app into a standalone executable.
---

## Build for distribution

Once your app is ready, you can package it into a standalone executable:

```sh
uzumaki build
```

This reads `uzumaki.config.json` from your project root, runs the build command, and packs everything into a single binary.

## Configuration

The build is configured through `uzumaki.config.json`:

```json
{
  "productName": "my-app",
  "version": "0.1.0",
  "identifier": "com.example.my_app",
  "build": {
    "command": "bun build src/index.tsx --target node --outdir dist --minify"
  },
  "pack": {
    "dist": "./dist",
    "entry": "index.js",
    "output": "./target/my-app",
    "name": "my-app"
  },
  "bundle": {
    "resources": ["assets/**/*"]
  }
}
```

### Fields

| Field              | Description                                                                        |
| ------------------ | ---------------------------------------------------------------------------------- |
| `productName`      | Display name for your app                                                          |
| `version`          | App version string                                                                 |
| `identifier`       | Bundle identifier (e.g. `com.yourname.app`)                                        |
| `build.command`    | Shell command to bundle your JS/TS                                                 |
| `pack.dist`        | Directory containing bundled output                                                |
| `pack.entry`       | Entry point file within dist                                                       |
| `pack.output`      | Path for the final executable                                                      |
| `pack.name`        | Name for the output binary                                                         |
| `bundle.resources` | Paths or globs (relative to the config) to copy next to the packed exe (see below) |

## Bundling resources

Files listed in `bundle.resources` are staged at build time into `<pack.output_dir>/resources/`, preserving their relative path under the project root. At runtime, look them up with [`Uz.path.resource(...)`](/reference/paths/):

```ts
const logo = Uz.path.resource('assets/logo.svg');
```

In dev (`uzumaki run ...`) no copy happens — `Uz.path.resource(...)` reads straight from the project tree, so the same code works in both modes.

See [Paths & Resources](/reference/paths/) for the full API.

## Skip the build step

If you've already bundled your code, skip the build command:

```sh
uzumaki build --no-build
```

## Custom config path

```sh
uzumaki build --config path/to/uzumaki.config.json
```
