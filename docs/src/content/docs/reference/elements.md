---
title: Elements
description: Intrinsic JSX elements available in Uzumaki.
---

:::caution
Uzumaki is in alpha. This API is unstable and may change between releases.
:::

Uzumaki provides a small set of intrinsic JSX elements for building native UIs. These are not HTML elements — they are rendered directly on the GPU.

## `<view>`

The primary layout container. Works like a `div` with flexbox.

```tsx
<view display="flex" flexDir="col" gap={16} p={20} bg="#1a1a1a" rounded={8}>
  {/* children */}
</view>
```

Views support all layout, styling, and event props.

Adding the `scrollable` prop enables vertical scrolling:

```tsx
<view scrollable h={300}>
  {/* overflowing content */}
</view>
```

Adding the `selectable` prop allows users to select text inside the view:

```tsx
<view selectable>This text can be selected.</view>
```

## `<text>`

Renders text content.

```tsx
<text fontSize={18} color="#e4e4e7" fontWeight={700}>
  Hello, world
</text>
```

Text can also be placed directly inside a `<view>`:

```tsx
<view>Some text content here</view>
```

## `<p>`

Alias for `<text>`. Behaves identically.

## `<button>`

A pressable element. Supports the same props as `<view>`.

```tsx
<button onClick={() => console.log('pressed')} p={10} bg="#2d2d30" rounded={6}>
  <text color="#60a5fa">Click me</text>
</button>
```

## `<input>`

Text input field.

```tsx
<input
  w={300}
  placeholder="Type something..."
  fontSize={16}
  color="#e4e4e7"
  value={value}
  onChangeText={setValue}
/>
```

### Input props

| Prop           | Type                     | Description                 |
| -------------- | ------------------------ | --------------------------- |
| `value`        | `string`                 | Controlled value            |
| `onChangeText` | `(text: string) => void` | Called when text changes    |
| `placeholder`  | `string`                 | Placeholder text            |
| `secure`       | `boolean`                | Mask input (password field) |
| `multiline`    | `boolean`                | Allow multiple lines        |

```tsx
// Password input
<input secure value={password} onChangeText={setPassword} />

// Multiline
<input multiline h={120} value={text} onChangeText={setText} />
```
