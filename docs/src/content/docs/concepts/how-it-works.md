---
title: How Uzumaki Works
description: The mental model for windows, native elements, React, and the runtime.
---

Uzumaki has three layers:

1. The native runtime creates windows, owns the app event loop, and draws UI.
2. The built-in `uzumaki` module exposes runtime APIs to JavaScript.
3. `uzumaki-react` turns React updates into Uzumaki element operations.

That makes Uzumaki feel familiar if you know React, but the output is not a web page.

## Runtime, Not Browser Shell

Electron apps usually render into Chromium. Uzumaki renders a native tree owned by the runtime.

```tsx
import { Window } from 'uzumaki';
import { render } from 'uzumaki-react';

const window = new Window('main', { width: 900, height: 620 });
render(window, <App />);
```

`Window` is not `window` from the browser. It is a runtime object that controls a native application window.

## Elements Are Native Runtime Nodes

Uzumaki JSX uses intrinsic elements:

```tsx
<view display="flex" flexDir="col" gap={12}>
  <text>Hello</text>
  <button onClick={save}>
    <text>Save</text>
  </button>
</view>
```

The renderer creates nodes such as `UzViewElement`, `UzTextElement`, and `UzButtonElement`. These elements expose tree operations, attributes, focus, and events through the low-level runtime API.

## Styling Is Prop-Based

Uzumaki does not use browser CSS. Styles are attributes on runtime elements:

```tsx
<view p={20} rounded={16} bg="#18181b" border={1} borderColor="#27272a" />
```

Numbers are logical pixels. Strings can represent special or relative values such as `"full"`, `"50%"`, `"auto"`, and `"2rem"`.

State variants use prop prefixes:

```tsx
<button bg="#27272a" hover:bg="#3f3f46" active:scale={0.98} />
```

## Events Bubble Through the Uzumaki Tree

Events are modeled after DOM flow, but they are runtime events. Handlers receive Uzumaki event objects with `target`, `currentTarget`, `preventDefault`, and propagation controls.

```tsx
<button
  onClick={(event) => {
    event.preventDefault();
    save();
  }}
>
  <text>Save</text>
</button>
```

Window lifecycle events live on the `Window` instance:

```ts
window.on('resize', (event) => {
  console.log(event.width, event.height);
});
```

## The Built-In Module

Apps import from `uzumaki`, but they do not install it like a normal dependency. The runtime provides it at execution time:

```tsx
import { Window, Clipboard, RUNTIME_VERSION } from 'uzumaki';
```

When bundling, mark `uzumaki` as external so the runtime can provide the real module.

## Where to Go Next

- Use [React Without the Browser](/concepts/react-runtime/) to understand renderer expectations.
- Use [Elements](/reference/elements/) and [Props](/reference/props/) while building screens.
- Use [Runtime API](/reference/runtime-api/) when you need imperative control.
