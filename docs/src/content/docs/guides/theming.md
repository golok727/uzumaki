---
title: Theme with Variables
description: Define theme tokens, reference them from props, and swap themes at runtime without rerendering React.
---

Define your colors and sizes once, reference them anywhere in your UI as `$name`, and swap the whole theme with a single call.

## Define Theme Tokens

Set up a map of tokens and pass it to `defineVars`. The returned `theme` object exposes each key as a `$name` reference for use in JSX.

```ts
// theme.ts
import { defineVars } from 'uzumaki';

const dark = {
  bg: '#0a0a0a',
  text: '#e4e4e7',
  accent: '#e2a52e',
};

const { vars: darkVars, theme } = defineVars(dark);

export { theme, darkVars };
```

Any string prop value that starts with `$` is resolved against the active vars at runtime. `defineVars` builds `theme` so that `theme.bg` is just the string `"$bg"` — you can write it by hand or let the helper do it for you.

## Apply on Window Create

Pass the initial map to `vars` on the window options.

```ts
import { Window } from 'uzumaki';
import { theme, darkVars } from './theme';

const window = new Window('main', {
  width: 800,
  height: 600,
  vars: darkVars,
  rootStyles: { bg: theme.bg, color: theme.text },
});
```

## Reference Tokens in Props

Drop `theme.token` into any prop — or write the `$name` string directly. Both work the same.

```tsx
<view bg={theme.bg} color="$text">
  <button bg={theme.accent} hover:bg={theme.accent}>
    <text color={theme.bg}>Click</text>
  </button>
</view>
```

If a token isn't defined yet, the prop just doesn't apply — define it later and everything bound to it picks up the value.

## Switch at Runtime

Call `setVars` with a new map. Every element that uses one of those tokens updates immediately. No re-render, no state loss.

```ts
window.setVars(lightVars);
```

To change a single token, use `setVar`. Pass `null` to remove a token entirely.

```ts
window.setVar('accent', '#22c55e');
window.setVar('accent', null);
```

## Across the App

Each window owns its own vars. If you have a theme store or React context driving the active theme, call `window.setVars(...)` from wherever the theme changes — components keep their state and the UI updates in place.
