---
title: Quick Start
description: Create your first Uzumaki app.
---

## Create a project

```sh
uzumaki init my-app
```

This scaffolds a new project with the following structure:

```
my-app/
  src/
    index.tsx
  package.json
  tsconfig.json
  uzumaki.config.json
```

The init command prompts for a project name and bundle identifier, then generates everything you need.

## Install dependencies

Uzumaki does not ship a bundler. Use whichever package manager and bundler you prefer — Bun, npm, pnpm, yarn, etc.

The default template is set up for Bun:

```sh
cd my-app
bun install
```

Using npm or pnpm works the same way:

```sh
# npm
npm install

# pnpm
pnpm install
```

## Run in dev mode

```sh
bun dev
```

This runs `uzumaki src/index.tsx`, which starts the Deno-based runtime, compiles your TypeScript/JSX, and opens your app window.

You can also run any file directly:

```sh
uzumaki src/index.tsx
```

## Project template

The generated `src/index.tsx` looks like this:

```tsx
import { useState } from 'react';
import { Window } from 'uzumaki-ui';
import { render } from 'uzumaki-ui/react';

const window = new Window('main', {
  width: 800,
  height: 600,
  title: 'my-app',
});

function App() {
  const [count, setCount] = useState(0);

  return (
    <view
      display="flex"
      flexDir="col"
      w="full"
      h="full"
      items="center"
      justify="center"
      bg="#0f0f0f"
      gap={16}
    >
      <text fontSize={32} fontWeight={700} color="#e4e4e7">
        Welcome to Uzumaki
      </text>
      <text fontSize={18} color="#a1a1aa">
        Count: {count}
      </text>
      <view
        onClick={() => setCount((c) => c + 1)}
        p={10}
        px={24}
        bg="#2d2d30"
        rounded={8}
        hover:bg="#3e3e42"
        cursor="pointer"
      >
        <text fontSize={16} color="#60a5fa">
          Increment
        </text>
      </view>
    </view>
  );
}

render(window, <App />);
```

## Bundler note

Uzumaki does not include a bundler. The default template uses Bun for both package management and bundling, but you can swap in any tool you like:

- **Bun** — `bun build src/index.tsx --target node --outdir dist --minify`
- **esbuild** — `esbuild src/index.tsx --bundle --platform=node --outdir=dist`
- **Rollup**, **Webpack**, **Vite** — configure as usual for a Node target

Update the `build.command` field in `uzumaki.config.json` to match your setup.
