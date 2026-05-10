---
title: React Without the Browser
description: How React maps onto Uzumaki's native renderer.
---

`uzumaki-react` is a React renderer. It lets React manage components, state, effects, refs, and reconciliation while Uzumaki owns the native nodes.

This is the same broad idea as React Native: React is the programming model, but the host elements are not HTML.

## JSX Setup

Projects use React JSX with Uzumaki's JSX runtime:

```json
{
  "compilerOptions": {
    "jsx": "react-jsx",
    "jsxImportSource": "uzumaki-react",
    "types": ["uzumaki-types"]
  }
}
```

`uzumaki-types` teaches TypeScript about the built-in `uzumaki` module and Uzumaki's JSX intrinsic elements.

## Host Elements

Use the elements that the renderer knows about:

| Element      | Purpose                                            |
| ------------ | -------------------------------------------------- |
| `<view>`     | Layout, grouping, backgrounds, borders, scrolling. |
| `<text>`     | Text rendering.                                    |
| `<button>`   | Pressable content.                                 |
| `<input>`    | Text input.                                        |
| `<checkbox>` | Boolean input.                                     |
| `<image>`    | Local, bundled, or remote image.                   |

Do not use DOM tags such as `<div>`, `<span>`, or `<img>`.

## Props Instead of CSS

Uzumaki props are intentionally compact:

```tsx
<view display="flex" flexDir="row" items="center" gap={10} p={12}>
  <text fontWeight={700}>Inbox</text>
</view>
```

There are no CSS selectors, no browser layout engine, and no DOM attributes. If a prop is not in the Uzumaki JSX types, it is not supported yet.

## Refs Point to Uzumaki Elements

Refs receive runtime element instances:

```tsx
import { useRef } from 'react';
import type { UzInputElement } from 'uzumaki';

function SearchBox() {
  const inputRef = useRef<UzInputElement>(null);

  return (
    <button onClick={() => inputRef.current?.focus()}>
      <text>Focus search</text>
      <input ref={inputRef} placeholder="Search" />
    </button>
  );
}
```

That ref is an Uzumaki element, so methods like `focus()` come from the runtime API.

## A Good Rule of Thumb

Write components like React. Choose elements like React Native. Think about platform APIs like a desktop runtime.

If you are unsure whether something is supported, check the in-tree JSX types at `packages/uzumaki-react/src/jsx/types.ts`.
