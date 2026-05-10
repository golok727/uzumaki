---
title: Style Native UI
description: Use Uzumaki layout, typography, color, variants, and scrolling props.
---

Uzumaki styles are element props. That keeps styling close to the native tree and makes examples easy to copy while the framework is still young.

## Create a Layout Shell

```tsx
<view
  display="flex"
  flexDir="col"
  w="full"
  h="full"
  bg="#0b0b0f"
  color="#f8fafc"
>
  <view p={20} borderBottom={1} borderColor="#27272a">
    <text fontSize={20} fontWeight={800}>
      Settings
    </text>
  </view>
  <view flex={1} p={20}>
    {children}
  </view>
</view>
```

Use `w="full"` and `h="full"` for full-window surfaces. Use `flex={1}` for panels that should fill remaining space.

## Use Flex Props

```tsx
<view display="flex" flexDir="row" items="center" justify="between" gap={12}>
  <text>Notifications</text>
  <checkbox checked={enabled} onValueChange={setEnabled} />
</view>
```

The most common flex props are `display`, `flexDir`, `items`, `justify`, `gap`, `flex`, `flexGrow`, and `flexShrink`.

## Add Visual States

Prefix style props with `hover:`, `active:`, or `focus:`:

```tsx
<button
  px={14}
  py={9}
  rounded={10}
  bg="#18181b"
  hover:bg="#27272a"
  active:scale={0.98}
  focus:outline={2}
  focus:outlineColor="#f59e0b"
>
  <text>Open</text>
</button>
```

State variants work best for visual props such as color, opacity, border, outline, and transforms.

## Build a Card

```tsx
<view
  p={18}
  rounded={18}
  bg="#111113"
  border={1}
  borderColor="#27272a"
  display="flex"
  flexDir="col"
  gap={8}
>
  <text fontSize={13} color="#a1a1aa">
    Runtime
  </text>
  <text fontSize={22} fontWeight={800}>
    Native window ready
  </text>
  <text color="#d4d4d8" textWrap="wrap">
    This card is styled entirely with Uzumaki props.
  </text>
</view>
```

## Make Content Scroll

```tsx
<view
  h={320}
  scrollY
  scrollbarWidth={6}
  scrollbarRadius={999}
  scrollbarColor="#3f3f46"
  scrollbarHoverColor="#71717a"
>
  {rows}
</view>
```

Use `scroll` for both axes, or `scrollX` and `scrollY` for one direction.

## Use `rem`

`"1rem"` resolves from the window's `remBase`.

```ts
window.remBase = 16;
```

```tsx
<text fontSize="1.25rem">Readable text</text>
```

Use this when you want a whole app to scale from one setting.
