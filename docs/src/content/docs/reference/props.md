---
title: Props
description: Layout, style, interaction, and state props.
---

Uzumaki style props are runtime attributes. They are shared by most elements.

## Values

Numbers are logical pixels:

```tsx
<view p={16} rounded={12} />
```

Strings can express relative or special values:

```tsx
<view w="100%" h="full" m="auto" fontSize="1.25rem" />
```

Colors are hex strings:

```tsx
<text color="#f4f4f5" />
```

## Size and Spacing

| Prop                                    | Description               |
| --------------------------------------- | ------------------------- |
| `w`, `h`                                | Width and height.         |
| `minW`, `minH`                          | Minimum width and height. |
| `p`, `px`, `py`, `pt`, `pr`, `pb`, `pl` | Padding.                  |
| `m`, `mx`, `my`, `mt`, `mr`, `mb`, `ml` | Margin.                   |

## Layout

| Prop                             | Values                  |
| -------------------------------- | ----------------------- |
| `display`                        | `flex`, `block`, `none` |
| `position`                       | `relative`, `absolute`  |
| `top`, `right`, `bottom`, `left` | Number or string offset |

## Flex

| Prop                             | Values                                                  |
| -------------------------------- | ------------------------------------------------------- |
| `flexDir`                        | `row`, `col`, `column`                                  |
| `flexWrap`                       | `nowrap`, `wrap`, `wrap-reverse`                        |
| `flex`, `flexGrow`, `flexShrink` | Number, string, or `true` for `flex`                    |
| `items`                          | `start`, `end`, `center`, `stretch`, `baseline`         |
| `justify`                        | `start`, `end`, `center`, `between`, `around`, `evenly` |
| `gap`                            | Number or string                                        |

```tsx
<view display="flex" flexDir="row" items="center" justify="between" gap={12} />
```

## Color and Typography

| Prop                                   | Values                                               |
| -------------------------------------- | ---------------------------------------------------- |
| `bg`, `color`                          | Hex color                                            |
| `opacity`                              | Number or string                                     |
| `visibility`                           | `visible`, `hidden`                                  |
| `fontSize`, `fontWeight`, `fontFamily` | Number or string                                     |
| `textAlign`                            | `left`, `center`, `right`, `start`, `end`, `justify` |
| `textWrap`                             | `wrap`, `nowrap`, `anywhere`, `break-word`           |
| `wordBreak`                            | `normal`, `break-all`, `keep-all`                    |

## Borders, Corners, and Outlines

| Prop                                                               | Description                |
| ------------------------------------------------------------------ | -------------------------- |
| `rounded`, `roundedTL`, `roundedTR`, `roundedBR`, `roundedBL`      | Corner radius.             |
| `border`, `borderTop`, `borderRight`, `borderBottom`, `borderLeft` | Border width.              |
| `borderColor`                                                      | Border color.              |
| `outline`, `outlineColor`, `outlineOffset`                         | Focus or emphasis outline. |

## Transforms

```tsx
<view translate={[8, 0]} rotate={-3} scale={1.05} hover:scale={1.08} />
```

| Prop                       | Description                      |
| -------------------------- | -------------------------------- |
| `translate`                | Number, `[x, y]`, or `{ x, y }`. |
| `translateX`, `translateY` | Single-axis translate.           |
| `rotate`                   | Rotation in degrees.             |
| `scale`                    | Number, `[x, y]`, or `{ x, y }`. |
| `scaleX`, `scaleY`         | Single-axis scale.               |

## Interaction

| Prop         | Description                                         |
| ------------ | --------------------------------------------------- |
| `cursor`     | Cursor shown while hovering.                        |
| `focusable`  | Allows a view to receive focus and keyboard events. |
| `selectable` | Allows text inside a view to be selected.           |

## Scrolling

```tsx
<view scrollY h={320} scrollbarWidth={6} scrollbarRadius={999}>
  {rows}
</view>
```

| Prop                           | Description              |
| ------------------------------ | ------------------------ |
| `scroll`, `scrollX`, `scrollY` | Enable scrolling.        |
| `scrollbarWidth`               | Scrollbar thickness.     |
| `scrollbarColor`               | Default scrollbar color. |
| `scrollbarHoverColor`          | Hover color.             |
| `scrollbarActiveColor`         | Active drag color.       |
| `scrollbarRadius`              | Scrollbar radius.        |

## State Variants

Prefix most visual props with `hover:`, `active:`, or `focus:`.

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

See [Elements](/reference/elements/) for element-specific props and [Events](/reference/events/) for event handlers.
