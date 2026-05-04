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
  resizable: true,
  decorations: true,
  theme: 'system',

  // applied to root element directly
  rootStyles: {
    bg: '#0f0f0f',
  },
});

function App() {
  return (
    <view w="full" h="full">
      <text color="#e4e4e7">Hello</text>
    </view>
  );
}

render(window, <App />);
```

---

## Window options

| Option                   | Type                                            | Description                        |
| ------------------------ | ----------------------------------------------- | ---------------------------------- |
| `width`                  | `number`                                        | Initial window width (logical px)  |
| `height`                 | `number`                                        | Initial window height (logical px) |
| `title`                  | `string`                                        | Window title                       |
| `visible`                | `boolean`                                       | Start visible                      |
| `resizable`              | `boolean`                                       | Allow resizing                     |
| `decorations`            | `boolean`                                       | Native titlebar + borders          |
| `transparent`            | `boolean`                                       | Enable transparency                |
| `maximized`              | `boolean`                                       | Start maximized                    |
| `minimized`              | `boolean`                                       | Start minimized                    |
| `fullscreen`             | `boolean`                                       | Borderless fullscreen              |
| `windowLevel`            | `'normal' \| 'alwaysOnTop' \| 'alwaysOnBottom'` | Z-order hint                       |
| `minWidth` / `minHeight` | `number`                                        | Minimum size                       |
| `maxWidth` / `maxHeight` | `number`                                        | Maximum size                       |
| `position`               | `{ x: number; y: number }`                      | Initial position                   |
| `theme`                  | `'light' \| 'dark' \| 'system'`                 | Native theme                       |
| `active`                 | `boolean`                                       | Request focus on start             |
| `contentProtected`       | `boolean`                                       | Screen capture protection          |
| `closable`               | `boolean`                                       | Enable close button                |
| `minimizable`            | `boolean`                                       | Enable minimize button             |
| `maximizable`            | `boolean`                                       | Enable maximize button             |
| `rootStyles`             | `Record<string, unknown>`                       | Applied directly to root element   |

---

## Runtime API

### Property-based updates

All major window attributes are updated via direct property assignment:

```ts
window.title = 'Renamed';
window.visible = true;
window.resizable = false;
window.decorations = false;
window.transparent = true;
window.maximized = true;
window.minimized = false;
window.fullscreen = false;
window.windowLevel = 'normal';
window.theme = 'dark';
window.contentProtected = true;
window.closable = true;
window.minimizable = true;
window.maximizable = false;
```

### Methods (only where needed)

```ts
window.focus();

window.setMinSize(640, 480);
window.setMaxSize(1440, 900);
window.setPosition(120, 80);
```

---

## Getters

```ts
window.title;
window.visible;
window.transparent;
window.resizable;
window.decorations;
window.maximized;
window.minimized;
window.fullscreen;
window.alwaysOnTop;
window.windowLevel;
window.innerSize;
window.outerSize;
window.position;
window.scaleFactor;
window.theme;
window.active;
window.contentProtected;
window.closable;
window.minimizable;
window.maximizable;
```

---

## Window lifecycle

```ts
window.on('load', () => {});
window.on('resize', (e) => {});
window.on('close', (e) => {});
```

---

## Multiple windows

```ts
import { getWindow } from 'uzumaki-ui';

const existing = getWindow('main');
```

Creating a window with the same label will throw.
