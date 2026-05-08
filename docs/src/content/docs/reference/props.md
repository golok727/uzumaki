---
title: Props
description: Layout, style, and interaction props.
---

Numbers are logical pixels. Strings can be `"50%"`, `"2rem"`, `"auto"`, `"full"`. Colors are hex strings.

## Layout

`w`, `h`, `minW`, `minH`, `maxW`, `maxH`. Padding: `p`, `px`, `py`, `pt`, `pr`, `pb`, `pl`. Margin: `m`, `mx`, `my`, `mt`, `mr`, `mb`, `ml`. `display`, `position` (`relative` / `absolute`), `top`, `right`, `bottom`, `left`.

## Flex

- `display="flex"`
- `flexDir` — `row` | `col` | `column`
- `flexWrap` — `nowrap` | `wrap` | `wrap-reverse`
- `flex`, `flexGrow`, `flexShrink`
- `items` — `start` | `end` | `center` | `stretch` | `baseline`
- `justify` — `start` | `end` | `center` | `between` | `around` | `evenly`
- `gap`

## Color & text

- `bg`, `color`, `opacity`, `visibility`
- `fontSize`, `fontWeight`, `fontFamily`
- `textWrap`, `wordBreak`

## Borders & corners

- `rounded`, `roundedTL`, `roundedTR`, `roundedBR`, `roundedBL`
- `border`, `borderTop`, `borderRight`, `borderBottom`, `borderLeft`, `borderColor`
- `outline`, `outlineColor`, `outlineOffset`

## Transforms

```tsx
<view translate={[8, 0]} rotate={-3} scale={1.05} hover:scale={1.08} />
```

`translate`, `translateX`, `translateY`, `rotate` (deg), `scale`, `scaleX`, `scaleY`.

## Interaction

`cursor`, `focusable`, `selectable`.

## Scrolling

`scroll`, `scrollX`, `scrollY`, `scrollbarWidth`, `scrollbarRadius`, `scrollbarColor`, `scrollbarHoverColor`, `scrollbarActiveColor`.

## State variants

`hover:`, `active:`, `focus:` prefixes work on most visual props.

```tsx
<button
  bg="#18181b"
  hover:bg="#27272a"
  active:bg="#3f3f46"
  focus:outline={2}
  focus:outlineColor="#60a5fa"
>
  <text>Open</text>
</button>
```

## Events

`onClick`, `onMouseDown`, `onMouseUp`, `onKeyDown`, `onKeyUp`, `onFocus`, `onBlur`, plus `Capture` variants. See [Events](/reference/events/).
