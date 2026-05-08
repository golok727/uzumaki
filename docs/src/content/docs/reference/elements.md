---
title: Elements
description: Built-in JSX elements.
---

`<view>`, `<text>`, `<button>`, `<input>`, `<checkbox>`, `<image>`. Native, not HTML.

## `<view>`

Layout container.

```tsx
<view display="flex" flexDir="col" gap={16} p={20}>
  <text>Title</text>
  <text color="#a1a1aa">Body</text>
</view>
```

Scrolling:

```tsx
<view scroll h={280} scrollbarWidth={6} scrollbarRadius={4}>
  {items}
</view>
```

Selectable text:

```tsx
<view selectable>Selectable</view>
```

## `<text>`

```tsx
<text fontSize={18} fontWeight={700} color="#f4f4f5">
  Hello
</text>
```

Plain strings inside `<view>` work too.

## `<button>`

A pressable `<view>`.

```tsx
<button
  onClick={() => {}}
  px={16}
  py={10}
  rounded={8}
  bg="#27272a"
  hover:bg="#3f3f46"
>
  <text>Save</text>
</button>
```

## `<input>`

```tsx
<input value={name} onValueChange={setName} placeholder="Name" w={280} />
```

Props: `value`, `onValueChange`, `onInput`, `placeholder`, `multiline`, `secure`.

```tsx
<input secure value={password} onValueChange={setPassword} />
<input multiline h={120} value={notes} onValueChange={setNotes} />
```

## `<checkbox>`

```tsx
<checkbox checked={enabled} onValueChange={setEnabled} />
```

## `<image>`

```tsx
<image src={Uz.path.resource('assets/logo.svg')} w={96} h={96} color="#fff" />
<image src="https://example.com/hero.png" w={420} h={240} rounded={12} />
```

`src` accepts bundled paths, absolute paths, `file://`, `https://`. Raster + SVG.

Loading events:

```tsx
<image
  src={src}
  onLoadStart={() => setStatus('loading')}
  onLoad={() => setStatus('loaded')}
  onError={(e) => setError(e.message)}
/>
```
