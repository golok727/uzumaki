# Uzumaki

A native desktop UI framework for JavaScript/TypeScript. Uses a runtime built on **Deno**, **wgpu** for GPU rendering, and **Vello** for 2D vector graphics. The React reconciler lets you write native desktop apps with JSX.

## JSX

Intrinsic elements: `<view>`, `<text>`, `<p>`, `<button>`

### Props

**Layout:** `w`, `h`, `p`, `px`, `py`, `pt`, `pb`, `pl`, `pr`, `m`, `mx`, `my`, `mt`, `mb`, `ml`, `mr`, `gap`

**Flexbox:** `flex`, `flexDir` (`"row"` | `"col"`), `flexGrow`, `flexShrink`, `items` (`"center"` | `"start"` | `"end"` | `"stretch"`), `justify` (`"center"` | `"between"` | `"around"` | `"evenly"`)

**Styling:** `bg`, `color`, `fontSize`, `fontWeight`, `rounded`, `roundedTL/TR/BR/BL`, `border`, `borderTop/Right/Bottom/Left`, `borderColor`, `opacity`, `display` (`"flex"` | `"none"` | `"block"`), `cursor`, `visible`

**Transforms:** `translate`, `translateX`, `translateY`, `rotate`, `scale`, `scaleX`, `scaleY`

**State variants:** `hover:bg`, `hover:color`, `hover:opacity`, `hover:borderColor`, `active:bg`, `active:color`, `active:opacity`, `active:borderColor`, `hover:translate`, `hover:translateX`, `hover:translateY`, `hover:rotate`, `hover:scale`, `hover:scaleX`, `hover:scaleY`, `active:translate`, `active:translateX`, `active:translateY`, `active:rotate`, `active:scale`, `active:scaleX`, `active:scaleY`

**Events:** `onClick`, `onMouseDown`, `onMouseUp`

### Value formats

- Numbers: treated as px (`w={100}`)
- Strings: `"100"` (px), `"2rem"`, `"50%"`, `"auto"`, `"full"` (= 100%)
- Colors: hex strings (`"#FF5733"`, `"#FF573380"`), `"transparent"`

### Example

```tsx
import { Window } from 'uzumaki';
import { render } from 'uzumaki/react';

const window = new Window('main', { width: 800, height: 600, title: 'My App' });

function App() {
  const [count, setCount] = useState(0);
  return (
    <view
      display="flex"
      flexDir="col"
      w="full"
      h="full"
      items="center"
      justify="center"
      bg="#0f0f0f"
    >
      <text fontSize={24} color="#d4d4d4">
        Count: {count}
      </text>
      <view
        onClick={() => setCount((c) => c + 1)}
        p="8"
        px="16"
        bg="#2d2d30"
        rounded="6"
        hover:bg="#373738"
      >
        <text fontSize={16} color="#569cd6">
          Increment
        </text>
      </view>
    </view>
  );
}

render(window, <App />);
```

## Commands

```sh
pnpm start              # Build core + run playground
# or
pnpm build:core         # Build the Rust NAPI addon
pnpm --filter playground start  # Run playground only
```
