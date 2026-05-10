---
title: Elements
description: Built-in JSX elements supported by uzumaki-react.
---

Uzumaki elements are native runtime elements, not HTML tags. Use them with `jsxImportSource: "uzumaki-react"`.

| Element      | Use it for                                                   |
| ------------ | ------------------------------------------------------------ |
| `<view>`     | Layout, grouping, backgrounds, borders, scrolling.           |
| `<text>`     | Text content.                                                |
| `<button>`   | Pressable content.                                           |
| `<input>`    | Text input.                                                  |
| `<checkbox>` | Boolean input.                                               |
| `<image>`    | Raster or SVG images from local, bundled, or remote sources. |

## `<view>`

`<view>` is the main layout primitive.

```tsx
<view display="flex" flexDir="col" gap={16} p={20}>
  <text fontSize={20} fontWeight={800}>
    Title
  </text>
  <text color="#a1a1aa">Body</text>
</view>
```

Use it for shells, cards, rows, columns, scroll containers, and absolute-positioned surfaces.

```tsx
<view scrollY h={280} scrollbarWidth={6} scrollbarRadius={999}>
  {items}
</view>
```

Set `selectable` when text inside the view should be selectable.

## `<text>`

Use `<text>` for explicit text styling.

```tsx
<text fontSize={18} fontWeight={700} color="#f4f4f5" textWrap="wrap">
  Hello from Uzumaki
</text>
```

Plain strings inside `<view>` are supported, but `<text>` is clearer when you need typography props.

## `<button>`

`<button>` is a pressable container. Put `<text>` or other elements inside it.

```tsx
<button
  onClick={save}
  px={16}
  py={10}
  rounded={10}
  bg="#27272a"
  hover:bg="#3f3f46"
  active:scale={0.98}
  cursor="pointer"
>
  <text fontWeight={700}>Save</text>
</button>
```

Buttons support pointer, keyboard, focus, and blur handlers.

## `<input>`

```tsx
<input
  value={name}
  onValueChange={setName}
  placeholder="Name"
  w={280}
  px={12}
  py={10}
  rounded={10}
/>
```

Input-specific props:

| Prop            | Description                                          |
| --------------- | ---------------------------------------------------- |
| `value`         | Controlled string value.                             |
| `onValueChange` | Receives the latest string value.                    |
| `onInput`       | Receives an input event with `inputType` and `data`. |
| `placeholder`   | Placeholder text.                                    |
| `disabled`      | Disable editing.                                     |
| `maxLength`     | Maximum text length.                                 |
| `multiline`     | Allow multiple lines.                                |
| `secure`        | Hide entered text for password-like fields.          |

## `<checkbox>`

```tsx
<checkbox checked={enabled} onValueChange={setEnabled} />
```

Checkbox-specific props:

| Prop            | Description                        |
| --------------- | ---------------------------------- |
| `checked`       | Controlled boolean value.          |
| `onValueChange` | Receives the latest boolean value. |
| `onInput`       | Receives the input event.          |

## `<image>`

```tsx
<image src={Uz.path.resource('assets/logo.svg')} w={96} h={96} />
<image src="https://example.com/hero.png" w={420} h={240} rounded={12} />
```

`src` accepts bundled paths, absolute paths, `file://` URLs, and `https://` URLs. Raster images and SVG are supported.

```tsx
<image
  src={src}
  onLoadStart={() => setStatus('loading')}
  onLoad={() => setStatus('loaded')}
  onError={(event) => setError(event.message)}
/>
```

## Refs

Refs point to runtime element instances:

```tsx
const inputRef = useRef<UzInputElement>(null);

<input ref={inputRef} />;
<button onClick={() => inputRef.current?.focus()}>
  <text>Focus input</text>
</button>;
```

Use element refs when you need imperative APIs such as `focus()`.
