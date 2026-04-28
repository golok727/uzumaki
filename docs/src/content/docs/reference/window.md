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

| Option                   | Type                                                          | Description                                         |
| ------------------------ | ------------------------------------------------------------- | --------------------------------------------------- |
| `width`                  | `number`                                                      | Initial window width in logical pixels              |
| `height`                 | `number`                                                      | Initial window height in logical pixels             |
| `title`                  | `string`                                                      | Window title bar text                               |
| `visible`                | `boolean`                                                     | Whether the window starts visible                   |
| `resizable`              | `boolean`                                                     | Whether the window can be resized                   |
| `decorations`            | `boolean`                                                     | Whether native titlebar and borders are shown       |
| `transparent`            | `boolean`                                                     | Whether the window background supports transparency |
| `maximized`              | `boolean`                                                     | Whether the window starts maximized                 |
| `minimized`              | `boolean`                                                     | Whether the window is minimized after creation      |
| `fullscreen`             | `boolean`                                                     | Whether the window starts in borderless fullscreen  |
| `alwaysOnTop`            | `boolean`                                                     | Convenience alias for `windowLevel: 'alwaysOnTop'`  |
| `windowLevel`            | `'normal' \| 'alwaysOnTop' \| 'alwaysOnBottom'`               | Requested z-level hint                              |
| `minWidth` / `minHeight` | `number`                                                      | Minimum size when both are provided                 |
| `maxWidth` / `maxHeight` | `number`                                                      | Maximum size when both are provided                 |
| `position`               | `{ x: number; y: number }`                                    | Initial window position                             |
| `theme`                  | `'light' \| 'dark' \| 'system'`                               | Preferred native window theme                       |
| `active`                 | `boolean`                                                     | Whether the window should request initial focus     |
| `contentProtected`       | `boolean`                                                     | Requests screen-capture protection where supported  |
| `enabledButtons`         | `{ close?: boolean; minimize?: boolean; maximize?: boolean }` | Enabled native titlebar buttons                     |

The first argument to `new Window()` is a window identifier string (e.g. `'main'`).
Use `getWindow(label)` to retrieve an existing window when you want to reuse or focus it instead of creating a duplicate.

## Runtime APIs

You can update several window properties after creation:

```ts
window.setTitle('Renamed');
window.setVisible(true);
window.setResizable(false);
window.setDecorations(false);
window.setTransparent(true);
window.setMaximized(true);
window.setMinimized(false);
window.setFullscreen(false);
window.setAlwaysOnTop(true);
window.setWindowLevel('normal');
window.setMinSize(640, 480);
window.setMaxSize(1440, 900);
window.setPosition(120, 80);
window.setTheme('dark');
window.focus();
window.setContentProtected(true);
window.setEnabledButtons({ close: true, minimize: true, maximize: false });
```

Read back common state through getters:

```ts
window.title;
window.visible;
window.transparent;
window.resizable;
window.decorated;
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
window.enabledButtons;
```

## Notes

- Sizes and positions use logical pixels.
- `fullscreen: true` uses borderless fullscreen.
- `windowLevel`, `alwaysOnTop`, `contentProtected`, `enabledButtons`, `active`, and runtime transparency are best-effort platform hints. The underlying OS or window manager may ignore them.
