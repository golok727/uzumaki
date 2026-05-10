---
title: Package for Distribution
description: Build JavaScript, copy resources, and package your app.
---

`uzumaki build` packages the app described by `uzumaki.config.json`.

```sh
uzumaki build
```

The command runs `build.command`, copies declared resources, and writes packaged output to `pack.outputDir`.

## Build Flow

<div class="uz-flow" aria-label="Uzumaki build flow">
  <div class="uz-flow__node"><code>uzumaki.config.json</code></div>
  <div class="uz-flow__split">
    <div class="uz-flow__path">
      <span class="uz-flow__arrow">→</span>
      <div class="uz-flow__node"><code>build.command</code></div>
      <span class="uz-flow__arrow">→</span>
      <div class="uz-flow__node"><code>pack.jsDist</code></div>
    </div>
    <div class="uz-flow__path">
      <span class="uz-flow__arrow">→</span>
      <div class="uz-flow__node"><code>bundle.resources</code></div>
    </div>
  </div>
  <span class="uz-flow__arrow">→</span>
  <div class="uz-flow__node uz-flow__node--accent"><code>uzumaki build</code></div>
  <span class="uz-flow__arrow">→</span>
  <div class="uz-flow__node"><code>pack.outputDir</code></div>
</div>

## Local Build

```sh
pnpm install
uzumaki build
```

Use this when you want the CLI to run your configured JavaScript build before packaging.

## CI Build

Bundle JavaScript yourself, then package without rerunning the build step:

```sh
pnpm install
pnpm build
uzumaki build --no-build
```

This keeps CI explicit and avoids running the same bundler twice.

## Resources in Packaged Apps

Files matched by `bundle.resources` are copied into the packaged app:

```json
{
  "bundle": {
    "resources": ["assets/**/*"]
  }
}
```

Resolve them with `Uz.path.resource(...)`:

```tsx
const logo = Uz.path.resource('assets/logo.svg');
```

The same call should work in development and packaged builds.

## Current Limitations

Installer formats such as DMG, MSI, and AppImage are not part of the CLI yet. Today, `uzumaki build` focuses on producing the packaged runtime output.
