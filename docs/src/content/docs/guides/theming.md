---
title: Theme with Variables
description: Define theme tokens, reference them from props, and swap themes at runtime without rerendering React.
---

Define your colors and sizes once, reference them anywhere in your UI as `$name`, and swap the whole theme with a single call.

## Define Theme Tokens

Set up two maps with the same keys, one per theme. `defineVars` turns the keys into `$name` references for use in JSX.

```ts
// theme.ts
import { defineVars } from 'uzumaki';

const dark = {
  bg: '#0a0a0a',
  text: '#e4e4e7',
  accent: '#e2a52e',
};

const light: typeof dark = {
  bg: '#fafafa',
  text: '#18181b',
  accent: '#f59e0b',
};

const { vars: darkVars, theme } = defineVars(dark);
const { vars: lightVars } = defineVars(light);

export const C = theme;
export const themes = { dark: darkVars, light: lightVars };
export type ThemeName = keyof typeof themes;
```

`C.bg` is a reference like `"$bg"`, not a color directly. Use it anywhere a color value goes and the runtime fills in the value from your theme.

## Apply on Window Create

Pass the initial map to `vars` on the window options.

```ts
import { Window } from 'uzumaki';
import { C, themes } from './theme';

const window = new Window('main', {
  width: 800,
  height: 600,
  vars: themes.dark,
  rootStyles: { bg: C.bg, color: C.text },
});
```

## Reference Tokens in Props

Drop `C.token` into any style prop. Values that start with `$` are looked up from your theme; everything else passes through.

```tsx
<view bg={C.bg} color={C.text}>
  <button bg={C.accent} hover:bg={C.accent}>
    <text color={C.bg}>Click</text>
  </button>
</view>
```

If a token isn't defined yet, the prop just doesn't apply — define it later and everything bound to it picks up the value.

## Switch at Runtime

Call `setVars` with a new map. Every element that uses one of those tokens updates immediately. No re-render, no state loss.

```ts
window.setVars(themes.light);
```

To change a single token, use `setVar`. Pass `null` to remove a token entirely.

```ts
window.setVar('accent', '#22c55e');
window.setVar('accent', null);
```

## Share Across Windows

Each window has its own theme. If your app has more than one window, keep them in sync with a small store.

```ts
// themeStore.ts
import type { Window } from 'uzumaki';
import { themes, type ThemeName } from './theme';

const listeners = new Set<() => void>();
const windows = new Set<Window>();
let current: ThemeName = 'dark';

export const themeStore = {
  get: () => current,
  set(name: ThemeName) {
    if (name === current) return;
    current = name;
    for (const w of windows) w.setVars(themes[name]);
    for (const l of listeners) l();
  },
  subscribe(cb: () => void) {
    listeners.add(cb);
    return () => listeners.delete(cb);
  },
  attach(window: Window) {
    windows.add(window);
    window.setVars(themes[current]);
  },
};
```

Wrap it with React's `useSyncExternalStore` for components that need to read the current theme:

```ts
// useTheme.ts
import { useSyncExternalStore } from 'react';
import { themeStore } from './themeStore';

export function useTheme() {
  return useSyncExternalStore(themeStore.subscribe, themeStore.get);
}
```

Call `themeStore.attach(window)` after each `new Window(...)`. From any component:

```tsx
const theme = useTheme();
return (
  <button onClick={() => themeStore.set(theme === 'dark' ? 'light' : 'dark')}>
    <text>Toggle theme</text>
  </button>
);
```
