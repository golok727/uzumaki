---
title: Props
description: Layout, styling, and event props for Uzumaki elements.
---

:::caution
Uzumaki is in alpha. This API is unstable and may change between releases.
:::

## Value formats

Props accept numbers, strings, or specific keywords:

- **Numbers** are treated as pixels: `w={100}`
- **Strings**: `"100"` (px), `"2rem"`, `"50%"`, `"auto"`
- **`"full"`** is shorthand for `100%`
- **Colors**: hex strings like `"#FF5733"` or with alpha `"#FF573380"`, or `"transparent"`

## Layout

| Prop                   | Description                  |
| ---------------------- | ---------------------------- |
| `w`                    | Width                        |
| `h`                    | Height                       |
| `minH`                 | Minimum height               |
| `p`                    | Padding (all sides)          |
| `px`                   | Horizontal padding           |
| `py`                   | Vertical padding             |
| `pt`, `pb`, `pl`, `pr` | Padding per side             |
| `m`                    | Margin (all sides)           |
| `mx`, `my`             | Margin horizontal / vertical |
| `mt`, `mb`, `ml`, `mr` | Margin per side              |
| `gap`                  | Gap between flex children    |

## Flexbox

| Prop         | Values                                          | Description        |
| ------------ | ----------------------------------------------- | ------------------ |
| `display`    | `"flex"`, `"none"`, `"block"`                   | Display mode       |
| `flex`       | number                                          | Flex shorthand     |
| `flexDir`    | `"row"`, `"col"`                                | Flex direction     |
| `flexGrow`   | number / string                                 | Flex grow factor   |
| `flexShrink` | number / string                                 | Flex shrink factor |
| `items`      | `"center"`, `"start"`, `"end"`, `"stretch"`     | Align items        |
| `justify`    | `"center"`, `"between"`, `"around"`, `"evenly"` | Justify content    |

## Styling

| Prop                                                     | Description                 |
| -------------------------------------------------------- | --------------------------- |
| `bg`                                                     | Background color            |
| `color`                                                  | Text color                  |
| `fontSize`                                               | Font size                   |
| `fontWeight`                                             | Font weight (numeric)       |
| `rounded`                                                | Border radius (all corners) |
| `roundedTL`, `roundedTR`, `roundedBR`, `roundedBL`       | Per-corner radius           |
| `border`                                                 | Border width (all sides)    |
| `borderTop`, `borderRight`, `borderBottom`, `borderLeft` | Per-side border             |
| `borderColor`                                            | Border color                |
| `opacity`                                                | Opacity (0-1)               |
| `cursor`                                                 | Cursor style                |
| `visible`                                                | Visibility                  |

## Transforms

| Prop         | Description                                            |
| ------------ | ------------------------------------------------------ |
| `translate`  | Move element (accepts number, `[x, y]`, or `{ x, y }`) |
| `translateX` | Horizontal offset                                      |
| `translateY` | Vertical offset                                        |
| `rotate`     | Rotation angle in degrees                              |
| `scale`      | Scale factor (accepts number, `[x, y]`, or `{ x, y }`) |
| `scaleX`     | Horizontal scale                                       |
| `scaleY`     | Vertical scale                                         |

Transform props also support state variants with `hover:` and `active:` prefixes (e.g., `hover:translateX`, `active:scale`).

```tsx
<view
  translate={[10, 5]}
  rotate={45}
  scale={1.2}
  hover:scale={1.3}
>
  <text>Transformed</text>
</view>

<button
  translateX={0}
  active:translateX={2}
  active:translateY={2}
>
  Press me
</button>
```

## State variants

Style props that apply on hover or active (press) states:

| Prop                 | Description           |
| -------------------- | --------------------- |
| `hover:bg`           | Background on hover   |
| `hover:color`        | Text color on hover   |
| `hover:opacity`      | Opacity on hover      |
| `hover:borderColor`  | Border color on hover |
| `active:bg`          | Background on press   |
| `active:color`       | Text color on press   |
| `active:opacity`     | Opacity on press      |
| `active:borderColor` | Border color on press |

```tsx
<view
  bg="#2d2d30"
  hover:bg="#3e3e42"
  active:bg="#4e4e52"
  rounded={6}
  p={10}
  cursor="pointer"
>
  <text color="#e4e4e7" hover:color="#ffffff">
    Hover me
  </text>
</view>
```

## Events

| Prop          | Type         | Description         |
| ------------- | ------------ | ------------------- |
| `onClick`     | `() => void` | Click / tap handler |
| `onMouseDown` | `() => void` | Mouse down handler  |
| `onMouseUp`   | `() => void` | Mouse up handler    |
