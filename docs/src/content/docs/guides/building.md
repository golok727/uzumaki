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
    "output": "./my-app",
    "name": "my-app"
  }
}
```

### Fields

| Field           | Description                                 |
| --------------- | ------------------------------------------- |
| `productName`   | Display name for your app                   |
| `version`       | App version string                          |
| `identifier`    | Bundle identifier (e.g. `com.yourname.app`) |
| `build.command` | Shell command to bundle your JS/TS          |
| `pack.dist`     | Directory containing bundled output         |
| `pack.entry`    | Entry point file within dist                |
| `pack.output`   | Path for the final executable               |
| `pack.name`     | Name for the output binary                  |

## Skip the build step

If you've already bundled your code, skip the build command:

```sh
uzumaki build --no-build
```

## Custom config path

```sh
uzumaki build --config path/to/uzumaki.config.json
```
