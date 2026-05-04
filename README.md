<div align="center">
  <img src="etc/logo.svg" width="140" alt="Uzumaki logo" />

  <h1>Uzumaki</h1>

  <p>Native desktop UI framework for JavaScript/TypeScript.<br/>
  GPU-rendered with <strong>wgpu</strong> + <strong>Vello</strong>, powered by a <strong>Deno</strong> runtime. Write your UI in React.</p>

[![CI](https://github.com/golok727/uzumaki/actions/workflows/ci.yml/badge.svg)](https://github.com/golok727/uzumaki/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE-MIT)
[![License: Apache 2.0](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE-APACHE)
[![npm](https://img.shields.io/npm/v/uzumaki-ui?color=cb3837&logo=npm)](https://www.npmjs.com/package/uzumaki-ui)
[![GitHub stars](https://img.shields.io/github/stars/golok727/uzumaki?style=flat&logo=github)](https://github.com/golok727/uzumaki/stargazers)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-orange?logo=rust)](https://www.rust-lang.org/)
[![Powered by Deno](https://img.shields.io/badge/runtime-Deno-black?logo=deno)](https://deno.com/)
[![wgpu](https://img.shields.io/badge/renderer-wgpu%20%2B%20vello-purple)](#)

</div>

> [!WARNING]
> Uzumaki is in alpha. The API is unstable and will change. Do not use in production.

## Quick Example

```tsx
import { useState } from 'react';
import { Window } from 'uzumaki-ui';
import { render } from 'uzumaki-ui/react';

const window = new Window('main', {
  width: 800,
  height: 600,
  title: 'My App',
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

## Install

**macOS / Linux**

```sh
curl -fsSL https://uzumaki.run/install.sh | sh
```

**Windows**

```powershell
irm https://uzumaki.run/install.ps1 | iex
```

Then create a project:

```sh
uzumaki init my-app
cd my-app
bun install
bun dev
```

## Links

- [Docs](https://uzumaki.run)
- [GitHub](https://github.com/golok727/uzumaki)
- [Contributing](CONTRIBUTING.md)
- [Development](DEVELOPMENT.md)
- [X / Twitter](https://x.com/golok727)

## Acknowledgements

Uzumaki wouldn't be possible without the work of these projects:

- [Deno](https://github.com/denoland/deno) — JavaScript runtime powering the embedded engine
- [wgpu](https://github.com/gfx-rs/wgpu) — Cross-platform GPU abstraction layer
- [Vello](https://github.com/linebender/vello) — GPU-accelerated 2D vector renderer
- [Parley](https://github.com/linebender/parley) — Text layout
- [Blitz](https://github.com/DioxusLabs/blitz) — Architectural inspiration and reference for working with the Linebender stack
- [Zed](https://github.com/zed-industries/zed) — Some utility code borrowed from their codebase

...and many more 🙏

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.
