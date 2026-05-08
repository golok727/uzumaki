---
title: Runtime API
description: Low-level imperative APIs from the uzumaki module.
---

Uzumaki isn't React-only. React is what's shipped today, but the runtime exposes a low-level API so other frameworks (Solid, etc.) can build custom renderers on top. First-party support for more is planned.

## Exports

From `uzumaki`:

- `Window`, `getWindow`
- `UzNode`, `UzTextNode`
- `Element`, `UzElement`, plus `UzRootElement`, `UzViewElement`, `UzTextElement`, `UzButtonElement`, `UzInputElement`, `UzCheckboxElement`, `UzImageElement`
- `Clipboard`
- `EventEmitter`
- `UzEvent`, `EventType`, `EventPhase`
- `RUNTIME_VERSION`

## Building a tree

```ts
import { Window } from 'uzumaki';

const window = new Window('main', { width: 640, height: 420 });

const view = window.createElement('view');
const label = window.createElement('text');

view.setAttributes({
  display: 'flex',
  items: 'center',
  justify: 'center',
  w: 'full',
  h: 'full',
  bg: '#0f0f0f',
});

label.textContent = 'Hello';
label.setAttribute('color', '#f4f4f5');
label.setAttribute('fontSize', 20);

view.appendChild(label);
window.root.appendChild(view);
```

## Tree ops

```ts
node.appendChild(child);
node.insertBefore(child, beforeNode);
node.removeChild(child);
node.remove();
node.removeChildren();
window.createTextNode('Hello');
```

## Attributes

Same names and values as JSX:

```ts
button.setAttributes({
  px: 16,
  py: 10,
  rounded: 8,
  bg: '#27272a',
  'hover:bg': '#3f3f46',
});
button.setAttribute('bg', '#18181b');
button.getAttribute('bg');
button.removeAttribute('bg');
button.focus();
```

## Events

```ts
button.on('click', () => {});
button.on('keydown', (e) => {
  if (e.key === 'Enter') e.preventDefault();
});
```

`EventEmitter` is exported for your own use.

## Window

```ts
window.requestRedraw();
window.focus();
window.setPosition(100, 80);
window.remBase = 18;
```

## Clipboard

```ts
import { Clipboard } from 'uzumaki';
Clipboard.readText();
Clipboard.writeText('hi');
```

## Paths & version

```ts
Uz.path.resource('assets/logo.svg');
Uz.path.dataDir();

import { RUNTIME_VERSION } from 'uzumaki';
```
