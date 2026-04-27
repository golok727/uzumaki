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

## `<button>`

A pressable element. Supports the same props as `<view>`.

```tsx
<button onClick={() => console.log('pressed')} p={10} bg="#2d2d30" rounded={6}>
  <text color="#60a5fa">Click me</text>
</button>
```

## `<input>`

Text input field.

This element is specifically for text entry. Checkbox controls use a separate
`<checkbox>` intrinsic instead of `type="checkbox"`.

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

## `<checkbox>`

A boolean form control for checked and unchecked state.

```tsx
<checkbox
  checked={done}
  onChange={setDone}
  bg="#3b82f6"
  borderColor="#93c5fd"
  color="#ffffff"
  rounded={6}
  w={20}
  h={20}
/>
```

### Checkbox props

| Prop       | Type                         | Description                   |
| ---------- | ---------------------------- | ----------------------------- |
| `checked`  | `boolean`                    | Controlled checked state      |
| `onChange` | `(checked: boolean) => void` | Called when the value toggles |

Checkboxes also support the normal element props from `<view>`, which makes
them easy to customize with `bg`, `borderColor`, `color`, `rounded`, `border`,
`w`, `h`, `opacity`, `hover:*`, and `active:*`.

```tsx
<view display="flex" items="center" gap={12}>
  <checkbox
    checked={marketing}
    onChange={setMarketing}
    bg="#22c55e"
    borderColor={marketing ? '#22c55e' : '#3f3f46'}
    color="#08110a"
    rounded={4}
  />
  <text color="#e4e4e7">Email marketing</text>
</view>
```

## `<image>`

Renders a native raster image decoded by the runtime and painted through Vello.

```tsx
const hero = new URL('./hero.png', import.meta.url).href;

<image src={hero} w={320} rounded={12} />;
```

### Image props

| Prop  | Type     | Description          |
| ----- | -------- | -------------------- |
| `src` | `string` | Image source to load |

Sizing behavior in v1:

- No `w` and `h`: uses the image's natural size
- Only one of `w` or `h`: preserves aspect ratio
- Both `w` and `h`: stretches to the given box

Source behavior in v1:

- Preferred bundled asset form: `new URL('./asset.png', import.meta.url).href`
- Also supports remote URLs such as `https://...`
- Also supports explicit local file paths and `file://` URLs

Supported formats follow Deno's current image pipeline and cover common raster
formats such as PNG, JPEG, GIF, BMP, ICO, and WebP. SVG is not supported by
this element.
