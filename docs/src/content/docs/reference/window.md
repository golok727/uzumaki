---
title: Window
description: Create and control native windows.
---

```tsx
import { Window } from 'uzumaki';
import { render } from 'uzumaki-react';

const window = new Window('main', {
  width: 960,
  height: 680,
  title: 'My App',
  theme: 'system',
  rootStyles: { bg: '#09090b', color: '#f4f4f5', fontSize: 14 },
});

render(window, <App />);
```

The first arg is a unique label.

## Options

- `width`, `height`, `title`, `visible`, `resizable`
- `decorations`, `transparent`
- `maximized`, `minimized`, `fullscreen`
- `windowLevel` — `normal` | `alwaysOnTop` | `alwaysOnBottom`
- `minWidth`, `minHeight`, `maxWidth`, `maxHeight`
- `position` — `{ x, y }`
- `theme` — `light` | `dark` | `system`
- `closable`, `minimizable`, `maximizable`
- `active`, `contentProtected`
- `rootStyles` — element props applied to the window root before mount

## Mutable properties

```ts
window.title = 'Renamed';
window.visible = true;
window.resizable = false;
window.decorations = true;
window.transparent = false;
window.maximized = false;
window.minimized = false;
window.fullscreen = false;
window.windowLevel = 'alwaysOnTop';
window.theme = 'dark';
window.contentProtected = false;
```

## Methods

```ts
window.focus();
window.close();
window.requestRedraw();
window.setMinSize(720, 480);
window.setMaxSize(1600, 1200);
window.setPosition(120, 80);
```

## Getters

`id`, `label`, `innerWidth`, `innerHeight`, `innerSize`, `outerSize`, `position`, `scaleFactor`, `active`, `isDisposed`, `remBase`.

`remBase` controls what `"1rem"` resolves to.

## Events

```ts
window.on('load', () => {});
window.on('resize', (e) => console.log(e.width, e.height));
window.on('close', () => {});
```

UI events bubble to the window — useful for global shortcuts.

## Multiple windows

```ts
import { Window, getWindow } from 'uzumaki';

const existing = getWindow('main');
if (!existing) new Window('main', { width: 800, height: 600 });
```
