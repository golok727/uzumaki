---
title: Quick Start
description: Create a native counter app and learn the core Uzumaki loop.
---

This tutorial takes you from a fresh project to a working native window. You will create a window, render React into it, style native elements, and handle an event.

## 1. Scaffold a Project

```sh
uzumaki init my-app
cd my-app
pnpm install
pnpm dev
```

The app opens in the Uzumaki runtime. If you came from Electron, the key difference is that there is no hidden Chromium page. The runtime owns the window and the native UI tree.

## 2. Open the Entry File

Open `src/index.tsx`. The shape should feel familiar:

```tsx
import { Window } from 'uzumaki';
import { render } from 'uzumaki-react';

const window = new Window('main', {
  width: 900,
  height: 620,
  title: 'My App',
});

render(window, <App />);
```

`Window` comes from the built-in `uzumaki` module. `render` comes from `uzumaki-react`, the React renderer.

## 3. Render Native JSX

Replace the app component with a small counter:

```tsx
import { useState } from 'react';
import { Window } from 'uzumaki';
import { render } from 'uzumaki-react';

const window = new Window('main', {
  width: 900,
  height: 620,
  title: 'Counter',
  rootStyles: {
    bg: '#0b0b0f',
    color: '#f8fafc',
    fontFamily: 'Inter',
  },
});

function App() {
  const [count, setCount] = useState(0);

  return (
    <view
      display="flex"
      flexDir="col"
      items="center"
      justify="center"
      h="full"
      gap={18}
    >
      <text fontSize={20} color="#94a3b8">
        Native React state
      </text>
      <text fontSize={56} fontWeight={800}>
        {count}
      </text>
      <button
        px={18}
        py={10}
        rounded={12}
        bg="#f59e0b"
        color="#111827"
        hover:bg="#fbbf24"
        active:scale={0.98}
        cursor="pointer"
        onClick={() => setCount((value) => value + 1)}
      >
        <text fontWeight={800}>Increment</text>
      </button>
    </view>
  );
}

render(window, <App />);
```

These tags are Uzumaki elements:

- `<view>` is the general layout container.
- `<text>` draws text.
- `<button>` is a pressable element.

They are not DOM nodes, so use Uzumaki props like `flexDir`, `items`, `rounded`, `bg`, and `hover:bg` instead of DOM attributes or CSS class names.

## 4. Add an Input

React state works the same way. Add a name field:

```tsx
const [name, setName] = useState('Uzumaki');

<input
  value={name}
  onValueChange={setName}
  placeholder="Project name"
  w={280}
  px={12}
  py={10}
  rounded={10}
  bg="#18181b"
/>;
```

Use `onValueChange` when you want the current value. Use `onInput` when you need event details such as `inputType` or `data`.

## 5. Next Steps

You now know the runtime loop:

1. Create a `Window`.
2. Render a React tree into it.
3. Compose native elements.
4. Style with element props.
5. Handle native events with React handlers.

Keep going with [How Uzumaki Works](/concepts/how-it-works/) or jump to [Style Native UI](/guides/styling/).
