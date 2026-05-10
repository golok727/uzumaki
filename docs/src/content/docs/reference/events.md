---
title: Events
description: Event handlers, event objects, propagation, and lifecycle events.
---

Uzumaki events are runtime events. They are modeled after DOM event flow, but they move through the Uzumaki element tree.

## Handler Props

Most elements accept these handlers:

| Handler                    | Event                               |
| -------------------------- | ----------------------------------- |
| `onClick`                  | Mouse click.                        |
| `onMouseDown`, `onMouseUp` | Mouse button press and release.     |
| `onKeyDown`, `onKeyUp`     | Keyboard events.                    |
| `onFocus`, `onBlur`        | Focus events on focusable elements. |

Pointer and keyboard handlers also support capture variants such as `onClickCapture` and `onKeyDownCapture`.

## Event Flow

Events can pass through capture, target, and bubble phases. Each event exposes:

```ts
event.target;
event.currentTarget;
event.eventPhase;
event.bubbles;
event.defaultPrevented;
```

Use flow-control methods when needed:

```ts
event.preventDefault();
event.stopPropagation();
event.stopImmediatePropagation();
```

## Mouse Events

```ts
event.x;
event.y;
event.screenX;
event.screenY;
event.button;
event.buttons;
```

Example:

```tsx
<button onClick={(event) => console.log(event.x, event.y)}>
  <text>Inspect click</text>
</button>
```

## Keyboard Events

```ts
event.key;
event.code;
event.keyCode;
event.repeat;
event.ctrlKey;
event.altKey;
event.shiftKey;
event.metaKey;
```

Example:

```tsx
<view
  focusable
  onKeyDown={(event) => {
    if (event.key === 'Escape') closePanel();
  }}
/>
```

## Input Events

Use `onValueChange` for controlled input state:

```tsx
<input value={query} onValueChange={setQuery} placeholder="Search" />
```

Use `onInput` for lower-level event data:

```tsx
<input
  value={query}
  onValueChange={setQuery}
  onInput={(event) => {
    console.log(event.inputType, event.data);
  }}
/>
```

Checkboxes use the same value-change pattern:

```tsx
<checkbox checked={done} onValueChange={setDone} />
```

## Clipboard Events and API

Clipboard events expose selected and clipboard text:

```ts
event.selectionText;
event.clipboardText;
```

Use the clipboard API directly when you need imperative access:

```ts
import { Clipboard } from 'uzumaki';

const text = Clipboard.readText();
Clipboard.writeText('hi');
```

## Window Lifecycle

```ts
window.on('load', () => {});
window.on('resize', (event) => console.log(event.width, event.height));
window.on('close', () => {});
```

UI events bubble to the window too, so a `Window` can handle global shortcuts.

## Image Events

```tsx
<image
  src={src}
  onLoadStart={() => setStatus('loading')}
  onLoad={() => setStatus('loaded')}
  onError={(event) => setError(event.message)}
/>
```
