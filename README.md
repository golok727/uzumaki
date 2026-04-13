# Uzumaki

Native desktop UI framework for JavaScript/TypeScript. GPU-rendered with **wgpu** + **Vello**, powered by a **Deno** runtime. Write your UI in React.

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

**macOS**

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
- [X / Twitter](https://x.com/golok727)

## License

[Apache-2.0](LICENSE)
