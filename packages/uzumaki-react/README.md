<p align="center">
  <a href="https://uzumaki.run"><img src="https://uzumaki.run/logo_256.svg" alt="Uzumaki"></a>
</p>

# uzumaki-react

React renderer for [Uzumaki](https://uzumaki.run). Mount a React tree into an Uzumaki `Window` using the same JSX you already know.

> Status: experimental. APIs may change.

## Install

```sh
npm install uzumaki-react react
```

`react` is a peer dependency.

## Usage

```tsx
import { Window } from 'uzumaki';
import { createRoot } from 'uzumaki-react';

const window = new Window('main', {
  width: 800,
  height: 600,
  title: 'Hello',
});

function App() {
  return (
    <view display="flex" items="center" justify="center" w="full" h="full">
      <text fontSize={24} color="#e2a52e">
        Hello Uzumaki
      </text>
    </view>
  );
}

createRoot(window).render(<App />);
```

## JSX setup

In `tsconfig.json`:

```json
{
  "compilerOptions": {
    "jsx": "react-jsx",
    "jsxImportSource": "uzumaki-react"
  }
}
```

This wires the intrinsic elements (`<view>`, `<text>`, `<image>`, ...) and their props.

## API

### `createRoot(window)`

Creates a root bound to `window`. Returns `{ render, unmount }`:

```ts
const root = createRoot(window);
root.render(<App />);
// later — re-render preserves React state:
root.render(<App />);
root.unmount();
```

The root registers itself with the window, so it unmounts automatically when the window is disposed.

## Docs

Full guide and element reference: [uzumaki.run](https://uzumaki.run).
