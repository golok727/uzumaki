---
title: Handle Events and State
description: Connect React state to Uzumaki events, inputs, checkboxes, and window lifecycle.
---

React state works normally. Uzumaki supplies the event objects and native host elements.

## Click Events

```tsx
function Counter() {
  const [count, setCount] = useState(0);

  return (
    <button onClick={() => setCount((value) => value + 1)}>
      <text>Clicked {count} times</text>
    </button>
  );
}
```

Pointer events include coordinates and button state:

```tsx
<view
  onMouseDown={(event) => {
    console.log(event.x, event.y, event.button);
  }}
/>
```

## Text Inputs

Use `onValueChange` for the current value:

```tsx
const [query, setQuery] = useState('');

<input
  value={query}
  onValueChange={setQuery}
  placeholder="Search"
  px={12}
  py={10}
  rounded={10}
/>;
```

Use `onInput` when you need event details:

```tsx
<input
  value={query}
  onValueChange={setQuery}
  onInput={(event) => {
    console.log(event.inputType, event.data);
  }}
/>
```

## Checkboxes

```tsx
const [enabled, setEnabled] = useState(false);

<checkbox checked={enabled} onValueChange={setEnabled} />;
```

## Keyboard Shortcuts

Elements can handle keyboard events when they can receive focus:

```tsx
<view
  focusable
  onKeyDown={(event) => {
    if (event.metaKey && event.key === 'k') {
      event.preventDefault();
      openCommandMenu();
    }
  }}
/>
```

UI events also bubble to the window, which is useful for global shortcuts:

```ts
window.on('keydown', (event) => {
  if (event.ctrlKey && event.key === ',') openSettings();
});
```

## Lifecycle Events

```ts
window.on('load', () => {
  console.log('window loaded');
});

window.on('resize', (event) => {
  console.log(event.width, event.height);
});

window.on('close', () => {
  console.log('window closed');
});
```

## Stop or Cancel an Event

```tsx
<button
  onClick={(event) => {
    event.stopPropagation();
    event.preventDefault();
  }}
>
  <text>Do not bubble</text>
</button>
```

Use `preventDefault()` for native default behavior. Use propagation controls when parent handlers should not run.
