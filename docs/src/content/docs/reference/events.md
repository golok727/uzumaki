---
title: Events
description: Event handlers and event objects.
---

Most elements accept `onClick`, `onMouseDown`, `onMouseUp`, `onKeyDown`, `onKeyUp`, `onFocus`, `onBlur`. Pointer and keyboard events also have `Capture` variants.

## Mouse event

```ts
event.x;
event.y;
event.screenX;
event.screenY;
event.button;
event.buttons;
```

## Keyboard event

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

## Flow control

```ts
event.preventDefault();
event.stopPropagation();
event.stopImmediatePropagation();
```

## `<input>`

```tsx
<input
  value={query}
  onValueChange={setQuery}
  onInput={(e) => console.log(e.inputType, e.data)}
/>
```

## `<checkbox>`

```tsx
<checkbox checked={done} onValueChange={setDone} />
```

## Clipboard

`copy`, `cut`, `paste` events:

```ts
event.selectionText;
event.clipboardText;
```

Or use the API directly:

```ts
import { Clipboard } from 'uzumaki';
Clipboard.readText();
Clipboard.writeText('hi');
```

## Window lifecycle

```ts
window.on('load', () => {});
window.on('resize', (e) => console.log(e.width, e.height));
window.on('close', () => {});
```

## `<image>`

```tsx
<image
  src={src}
  onLoadStart={() => setStatus('loading')}
  onLoad={() => setStatus('loaded')}
  onError={(e) => setError(e.message)}
/>
```
