---
title: Runtime API
description: Low-level imperative APIs from the built-in uzumaki module.
---

Most apps use React through `uzumaki-react`, but the built-in `uzumaki` module also exposes the runtime directly. Use this API for window control, custom renderers, clipboard access, resource paths, and imperative element work.

:::note[Framework support]
Uzumaki's architecture is framework-agnostic, but React is the first supported renderer today. React fits the current runtime well because JSX can be transformed as plain JavaScript. Frameworks like Solid and Vue have their own compilers, so supporting them cleanly needs a native transform/plugin system rather than a one-off workaround. That support is planned, but it will take a little time to land properly.
:::

## Importing

```ts
import {
  Window,
  Clipboard,
  Element,
  UzEvent,
  EventPhase,
  EventType,
  RUNTIME_VERSION,
} from 'uzumaki';
```

`uzumaki` is provided by the runtime. Do not bundle it into your app.

## Main Exports

| Export                                                                                                                        | Purpose                                      |
| ----------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------- |
| `Window`, `getWindow`                                                                                                         | Create and look up native windows.           |
| `UzNode`, `UzTextNode`                                                                                                        | Base tree node APIs.                         |
| `Element`, `UzElement`                                                                                                        | Runtime element APIs.                        |
| `UzRootElement`, `UzViewElement`, `UzTextElement`, `UzButtonElement`, `UzInputElement`, `UzCheckboxElement`, `UzImageElement` | Built-in element classes.                    |
| `Clipboard`                                                                                                                   | Read and write text clipboard contents.      |
| `EventEmitter`                                                                                                                | Local event emitter used by runtime objects. |
| `UzEvent`, `EventType`, `EventPhase`                                                                                          | Event objects and enums.                     |
| `RUNTIME_VERSION`                                                                                                             | Numeric runtime version.                     |

## Build a Tree Imperatively

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

This is the API that renderers build on top of. React users usually do not need to create trees manually.

## Tree Operations

```ts
node.appendChild(child);
node.insertBefore(child, beforeNode);
node.removeChild(child);
node.remove();
node.removeChildren();
node.destroy();

window.createElement('button');
window.createTextNode('Hello');
```

## Attributes

Use the same attribute names as JSX:

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
button.on('click', (event) => {
  event.preventDefault();
});

button.on('keydown', (event) => {
  if (event.key === 'Enter') submit();
});
```

Handlers receive Uzumaki event objects. See [Events](/reference/events/) for event fields.

## Window Control

```ts
window.title = 'Renamed';
window.focus();
window.requestRedraw();
window.setPosition(100, 80);
window.setMinSize(720, 480);
window.remBase = 18;
```

See [Window](/reference/window/) for all options and mutable properties.

## Clipboard

```ts
import { Clipboard } from 'uzumaki';

const text = Clipboard.readText();
Clipboard.writeText('Copied from Uzumaki');
```

## Paths and Version

```ts
const logo = Uz.path.resource('assets/logo.svg');
const dataDir = Uz.path.dataDir();

import { RUNTIME_VERSION } from 'uzumaki';
```

For generated signatures, see the Generated API section in the sidebar.
