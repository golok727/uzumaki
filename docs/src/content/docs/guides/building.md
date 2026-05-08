---
title: Building
description: Package your app for distribution.
---

```sh
uzumaki build
```

Reads `uzumaki.config.json`, runs `build.command`, packages the output plus declared resources into `pack.outputDir`.

## CI flow

Bundle yourself, then package:

```sh
pnpm install
pnpm build
uzumaki build --no-build
```

## Resources

Files matched by `bundle.resources` are copied into the packaged app and resolve through `Uz.path.resource(...)` the same way in dev and prod.

```json
{ "bundle": { "resources": ["assets/**/*"] } }
```

```tsx
const logo = Uz.path.resource('assets/logo.svg');
```

## Flags

```sh
uzumaki build --no-build
uzumaki build --config path/to/uzumaki.config.json
```

Installer formats (DMG, MSI, AppImage) aren't part of the CLI yet.
