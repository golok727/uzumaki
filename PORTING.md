claude --resume 590d9f6b-6963-4f27-a562-ef68b2f82ff4

**Context**

We're porting Uzumaki from the old Bun + napi-rs architecture to an embedded deno_core runtime. The napi calls have been removed and the runtime is now directly embedded. Most of the framework code is decoupled from napi, so large portions can be copied over with minimal changes.

**Source → Destination**

- `crates/uzumaki-core` → `crates/uzumaki-core-exp` (Rust core)
- `packages/playground` → `packages/playground-exp` (test playground)

Dependencies, JSX, and build tooling are already configured in the `-exp` targets. You don't need to set any of that up.

---

**What to copy as-is**

Anything that doesn't touch napi or the runtime boundary — layout, rendering, reconciler logic, component definitions, etc. Copy these files directly and only modify imports if paths changed.

---

**What to change**

### 1. Window class — expose more ops with graceful fallbacks

Add deno ops for window properties like `width`, `height`, `title`, etc. But the window may not exist yet when JS code first accesses these properties (e.g. during initial component setup before the window is created).

Handle this with a fallback pattern:

- Accept initial/default values in the `Window` constructor (passed from Rust via bootstrap or config).
- Store them as `_width`, `_height`, etc.
- On property access, try the live op first, fall back to the cached default if the op returns null/undefined.

```js
class Window {
  constructor(initialConfig) {
    this._width = initialConfig.width ?? 800;
    this._height = initialConfig.height ?? 600;
  }

  get width() {
    this._width = core.ops.op_get_window_width() ?? this._width;
    return this._width;
  }

  get height() {
    this._height = core.ops.op_get_window_height() ?? this._height;
    return this._height;
  }
}
```

This way the property always returns something valid — the live value when the window exists, the initial config when it doesn't. No crashes, no undefined.

### 2. OpState-based state access

Store `Rc<RefCell<AppState>>` inside deno's `OpState` so that all ops (window, DOM, events, etc.) can access shared framework state without global statics or extra plumbing. Every op that needs window dimensions, the DOM tree, or event state should pull from `OpState`.

### 3. Serialization

Leverage deno_core's built-in serde support for op arguments and return values. Remove any manual serialization hacks left over from the napi layer. Things that were painful to pass across the napi boundary should be straightforward now.

### 4. Event system — full rebuild

This is the biggest change. The old event system was duct-taped and didn't handle bubbling/capture correctly.

- Use `#[op2]` to create a proper `Event` object on the Rust side. Use deno's op macro to expose it as a JS class (or class-like object).
- Implement proper event propagation phases:
  - **Capture phase**: root → target
  - **Target phase**: fire on target
  - **Bubble phase**: target → root
- Support `stopPropagation()`, `stopImmediatePropagation()`, `preventDefault()`.
- Support `event.target` and `event.currentTarget` (updating `currentTarget` as the event walks the tree).
- This should be a solid, spec-aligned implementation — not a patch job.

---

**Acceptance Criteria**

- `packages/playground-exp` runs and behaves identically to `packages/playground` — same rendering, same interactions, same visual output.
- Window properties (`width`, `height`, etc.) are always accessible from JS, even before the native window is created (returns defaults).
- Events bubble and capture correctly through the node tree.
- `stopPropagation()`, `preventDefault()`, and related methods work correctly.
- No napi-rs dependency remains in the `-exp` crates/packages.
- No regressions in functionality compared to the old runtime.

# Examples From the deno runtime core repository

C:\Users\Radha\dev\kimi\deno\ext\webgpu\device.rs -> This has some examples of using the `op` macro within an impleentation idk how it works take a llok if you want

# Note

-- Dont remove the old framwork. just keep it i will remove it after this is working
