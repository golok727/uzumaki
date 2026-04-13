---
title: Window
description: Creating and configuring application windows.
---

:::caution
Uzumaki is in alpha. This API is unstable and may change between releases.
:::

## Creating a window

Every Uzumaki app starts by creating a `Window` and passing it to `render`:

```tsx
import { Window } from 'uzumaki-ui';
import { render } from 'uzumaki-ui/react';

const window = new Window('main', {
  width: 800,
  height: 600,
  title: 'My App',
});

function App() {
  return (
    <view w="full" h="full" bg="#0f0f0f">
      <text color="#e4e4e7">Hello</text>
    </view>
  );
}

render(window, <App />);
```

## Window options

| Option   | Type     | Description                     |
| -------- | -------- | ------------------------------- |
| `width`  | `number` | Initial window width in pixels  |
| `height` | `number` | Initial window height in pixels |
| `title`  | `string` | Window title bar text           |

The first argument to `new Window()` is a window identifier string (e.g. `'main'`).
