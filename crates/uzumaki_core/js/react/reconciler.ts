import ReactReconciler, { type EventPriority } from 'react-reconciler';
import { DefaultEventPriority } from 'react-reconciler/constants';
import type { JSX } from './jsx/runtime';
import type { Window } from '..';
import * as core from '../bindings';
import { PropKey } from '../bindings';
import { eventManager } from '../events';

// ── Input attribute keys ─────────────────────────────────────────────

const INPUT_ATTRS = new Set(['value', 'placeholder', 'disabled', 'maxLength', 'multiline', 'secure']);

// ── Prop key mapping ─────────────────────────────────────────────────

const PROP_NAME_TO_KEY: Record<string, number> = {
  w: PropKey.W, h: PropKey.H,
  p: PropKey.P, px: PropKey.Px, py: PropKey.Py, pt: PropKey.Pt, pb: PropKey.Pb, pl: PropKey.Pl, pr: PropKey.Pr,
  m: PropKey.M, mx: PropKey.Mx, my: PropKey.My, mt: PropKey.Mt, mb: PropKey.Mb, ml: PropKey.Ml, mr: PropKey.Mr,
  flex: PropKey.Flex, flexDir: PropKey.FlexDir, flexGrow: PropKey.FlexGrow, flexShrink: PropKey.FlexShrink,
  items: PropKey.Items, justify: PropKey.Justify, gap: PropKey.Gap,
  bg: PropKey.Bg, color: PropKey.Color, fontSize: PropKey.FontSize, fontWeight: PropKey.FontWeight,
  rounded: PropKey.Rounded, roundedTL: PropKey.RoundedTL, roundedTR: PropKey.RoundedTR, roundedBR: PropKey.RoundedBR, roundedBL: PropKey.RoundedBL,
  border: PropKey.Border, borderTop: PropKey.BorderTop, borderRight: PropKey.BorderRight, borderBottom: PropKey.BorderBottom, borderLeft: PropKey.BorderLeft,
  borderColor: PropKey.BorderColor, opacity: PropKey.Opacity,
  display: PropKey.Display, cursor: PropKey.Cursor,
  'hover:bg': PropKey.HoverBg, 'hover:color': PropKey.HoverColor, 'hover:opacity': PropKey.HoverOpacity, 'hover:borderColor': PropKey.HoverBorderColor,
  'active:bg': PropKey.ActiveBg, 'active:color': PropKey.ActiveColor, 'active:opacity': PropKey.ActiveOpacity, 'active:borderColor': PropKey.ActiveBorderColor,
};

// ── Prop type categorization ─────────────────────────────────────────

const LENGTH_KEYS = new Set([PropKey.W, PropKey.H]);
const COLOR_KEYS = new Set([
  PropKey.Bg, PropKey.Color, PropKey.BorderColor,
  PropKey.HoverBg, PropKey.HoverColor, PropKey.HoverBorderColor,
  PropKey.ActiveBg, PropKey.ActiveColor, PropKey.ActiveBorderColor,
]);
const ENUM_KEYS = new Set([PropKey.FlexDir, PropKey.Items, PropKey.Justify, PropKey.Display]);

// ── Value conversion helpers ─────────────────────────────────────────

function toJsLength(value: any): { value: number; unit: number } {
  if (typeof value === 'number') return { value, unit: 0 };
  const s = String(value);
  if (s === 'auto') return { value: 0, unit: 3 };
  if (s === 'full') return { value: 1.0, unit: 1 };
  if (s.endsWith('rem')) return { value: parseFloat(s), unit: 2 };
  if (s.endsWith('%')) return { value: parseFloat(s) / 100, unit: 1 };
  return { value: parseFloat(s) || 0, unit: 0 };
}

function toJsColor(value: any): { r: number; g: number; b: number; a: number } {
  if (typeof value === 'string') {
    if (value.startsWith('#')) {
      const hex = value.slice(1);
      if (hex.length === 6) {
        return {
          r: parseInt(hex.slice(0, 2), 16),
          g: parseInt(hex.slice(2, 4), 16),
          b: parseInt(hex.slice(4, 6), 16),
          a: 255,
        };
      }
      if (hex.length === 8) {
        return {
          r: parseInt(hex.slice(0, 2), 16),
          g: parseInt(hex.slice(2, 4), 16),
          b: parseInt(hex.slice(4, 6), 16),
          a: parseInt(hex.slice(6, 8), 16),
        };
      }
    }
    if (value === 'transparent') return { r: 0, g: 0, b: 0, a: 0 };
  }
  return { r: 255, g: 255, b: 255, a: 255 };
}

const FLEX_DIR_MAP: Record<string, number> = {
  row: 0, col: 1, column: 1, 'row-reverse': 2, 'col-reverse': 3, 'column-reverse': 3,
};

function toEnumValue(key: number, value: any): number {
  if (typeof value === 'number') return value;
  const s = String(value);
  switch (key) {
    case PropKey.FlexDir:
      return FLEX_DIR_MAP[s] ?? 0;
    case PropKey.Items:
      return ({ 'flex-start': 0, start: 0, 'flex-end': 1, end: 1, center: 2, stretch: 3, baseline: 4 } as any)[s] ?? 3;
    case PropKey.Justify:
      return ({ 'flex-start': 0, start: 0, 'flex-end': 1, end: 1, center: 2, 'space-between': 3, between: 3, 'space-around': 4, around: 4, 'space-evenly': 5, evenly: 5 } as any)[s] ?? 0;
    case PropKey.Display:
      return ({ none: 0, flex: 1, block: 2 } as any)[s] ?? 1;
    default:
      return 0;
  }
}

// ── Typed prop setters ───────────────────────────────────────────────

function setProp(windowId: number, nodeId: any, propName: string, value: any): void {
  // Special: flex with string direction value
  if (propName === 'flex' && typeof value === 'string') {
    const dir = FLEX_DIR_MAP[value];
    if (dir !== undefined) {
      core.setEnumProp(windowId, nodeId, PropKey.Display, 1); // Flex
      core.setEnumProp(windowId, nodeId, PropKey.FlexDir, dir);
      return;
    }
  }

  const key = PROP_NAME_TO_KEY[propName];
  if (key === undefined) return;

  if (LENGTH_KEYS.has(key)) {
    core.setLengthProp(windowId, nodeId, key, toJsLength(value));
  } else if (COLOR_KEYS.has(key)) {
    core.setColorProp(windowId, nodeId, key, toJsColor(value));
  } else if (ENUM_KEYS.has(key)) {
    core.setEnumProp(windowId, nodeId, key, toEnumValue(key, value));
  } else {
    core.setF32Prop(windowId, nodeId, key, typeof value === 'number' ? value : parseFloat(String(value)) || 0);
  }
}

function setInputAttr(windowId: number, nodeId: any, key: string, value: any): void {
  switch (key) {
    case 'value':
      core.setInputValue(windowId, nodeId, String(value ?? ''));
      break;
    case 'placeholder':
      core.setInputPlaceholder(windowId, nodeId, String(value ?? ''));
      break;
    case 'disabled':
      core.setInputDisabled(windowId, nodeId, !!value);
      break;
    case 'maxLength':
      core.setInputMaxLength(windowId, nodeId, typeof value === 'number' ? value : -1);
      break;
    case 'multiline':
      core.setInputMultiline(windowId, nodeId, !!value);
      break;
    case 'secure':
      core.setInputSecure(windowId, nodeId, !!value);
      break;
  }
}

function clearProp(windowId: number, nodeId: any, propName: string): void {
  // Special: flex clear doesn't need special handling
  const key = PROP_NAME_TO_KEY[propName];
  if (key === undefined) return;

  if (LENGTH_KEYS.has(key)) {
    core.setLengthProp(windowId, nodeId, key, { value: 0, unit: 3 }); // Auto
  } else if (COLOR_KEYS.has(key)) {
    core.setColorProp(windowId, nodeId, key, { r: 255, g: 255, b: 255, a: 255 });
  } else if (ENUM_KEYS.has(key)) {
    core.setEnumProp(windowId, nodeId, key, 0);
  } else {
    core.setF32Prop(windowId, nodeId, key, 0);
  }
}

// ── Event registry (delegated to EventManager) ──────────────────────
export { eventManager } from '../events';

// ── UElement ─────────────────────────────────────────────────────────

class UElement {
  id: any;
  type: string;
  windowId: number;
  styles: Record<string, any> = {};
  inputAttrs: Record<string, any> = {};
  eventListeners: Map<string, Function> = new Map();
  children: UElement[] = [];
  parent: UElement | null = null;

  constructor(
    id: any,
    type: string,
    windowId: number,
    props: Record<string, any>,
  ) {
    this.id = id;
    this.type = type;
    this.windowId = windowId;
    this.parseProps(props);
  }

  private parseProps(props: Record<string, any>) {
    for (const key in props) {
      if (key === 'children' || key === 'key' || key === 'ref') continue;
      const value = props[key];
      if (value == null) continue;

      if (
        key.length >= 3 &&
        key[0] === 'o' &&
        key[1] === 'n' &&
        key.charCodeAt(2) >= 65 &&
        key.charCodeAt(2) <= 90
      ) {
        // Event listener: onClick → click
        const eventName = key.slice(2).toLowerCase();
        this.eventListeners.set(eventName, value);
      } else if (INPUT_ATTRS.has(key)) {
        this.inputAttrs[key] = value;
      } else if (PROP_NAME_TO_KEY[key] !== undefined) {
        this.styles[key] = value;
      }
    }
  }
}

// ── Reconciler ───────────────────────────────────────────────────────

type Container = {
  window: Window;
  rootNodeId: any;
};

function getWindowId(container: Container): number {
  return container.window.id;
}

function getTextContent(children: any): string {
  if (children == null) return '';
  if (Array.isArray(children)) return children.join('');
  return String(children);
}

type Type = string;
type Props = Record<string, any>;
type Instance = UElement;
type TextInstance = UElement;
type SuspenseInstance = any;
type HydratableInstance = any;
type FormInstance = any;
type PublicInstance = UElement;
type HostContext = {};
type ChildSet = any;
type TimeoutHandle = ReturnType<typeof setTimeout>;
type NoTimeout = undefined;
type TransitionStatus = any;

let currentPriority: EventPriority = DefaultEventPriority;
let currentContainer: Container | null = null;

const reconciler = ReactReconciler<
  Type,
  Props,
  Container,
  Instance,
  TextInstance,
  SuspenseInstance,
  HydratableInstance,
  FormInstance,
  PublicInstance,
  HostContext,
  ChildSet,
  TimeoutHandle,
  NoTimeout,
  TransitionStatus
>({
  supportsMutation: true,
  supportsPersistence: false,

  createInstance(type, props, rootContainer) {
    const windowId = getWindowId(rootContainer);
    if (type === 'text' || type === 'p') {
      const text = getTextContent(props.children);
      const id = core.createTextNode(windowId, text);
      const el = new UElement(id, type, windowId, props);

      for (const [key, val] of Object.entries(el.styles)) {
        setProp(windowId, id, key, val);
      }

      if (el.eventListeners.size > 0) {
        core.setF32Prop(windowId, id, PropKey.Interactive, 1);
        for (const [event, cb] of el.eventListeners) {
          eventManager.addHandlerByName(id, event, cb);
        }
      }

      return el;
    }

    // View-like elements (including input)
    const id = core.createElement(windowId, type);
    const el = new UElement(id, type, windowId, props);

    for (const [key, val] of Object.entries(el.styles)) {
      setProp(windowId, id, key, val);
    }

    // Set input attributes
    for (const [key, val] of Object.entries(el.inputAttrs)) {
      setInputAttr(windowId, id, key, val);
    }

    if (el.eventListeners.size > 0) {
      core.setF32Prop(windowId, id, PropKey.Interactive, 1);
      for (const [event, cb] of el.eventListeners) {
        eventManager.addHandlerByName(id, event, cb);
      }
    }

    return el;
  },

  createTextInstance(text, rootContainer) {
    const windowId = getWindowId(rootContainer);
    const id = core.createTextNode(windowId, text);
    return new UElement(id, '#text', windowId, {});
  },

  shouldSetTextContent(type) {
    return type === 'text' || type === 'p';
  },

  appendInitialChild(parent, child) {
    parent.children.push(child);
    child.parent = parent;
    eventManager.setParent(child.id, parent.id);
    core.appendChild(parent.windowId, parent.id, child.id);
  },

  finalizeInitialChildren() {
    return false;
  },

  appendChildToContainer(container, child) {
    const windowId = getWindowId(container);
    child.parent = null;
    eventManager.setParent(child.id, container.rootNodeId);
    core.appendChild(windowId, container.rootNodeId, child.id);
  },

  appendChild(parent, child) {
    parent.children.push(child);
    child.parent = parent;
    eventManager.setParent(child.id, parent.id);
    core.appendChild(parent.windowId, parent.id, child.id);
  },

  insertBefore(parent, child, before) {
    const idx = parent.children.indexOf(before);
    if (idx >= 0) {
      parent.children.splice(idx, 0, child);
    } else {
      parent.children.push(child);
    }
    child.parent = parent;
    eventManager.setParent(child.id, parent.id);
    core.insertBefore(parent.windowId, parent.id, child.id, before.id);
  },

  insertInContainerBefore(container, child, before) {
    const windowId = getWindowId(container);
    child.parent = null;
    eventManager.setParent(child.id, container.rootNodeId);
    core.insertBefore(windowId, container.rootNodeId, child.id, before.id);
  },

  removeChild(parent, child) {
    const idx = parent.children.indexOf(child);
    if (idx >= 0) parent.children.splice(idx, 1);
    child.parent = null;
    core.removeChild(parent.windowId, parent.id, child.id);
    eventManager.clearNode(child.id);
  },

  removeChildFromContainer(container, child) {
    const windowId = getWindowId(container);
    child.parent = null;
    core.removeChild(windowId, container.rootNodeId, child.id);
    eventManager.clearNode(child.id);
  },

  commitUpdate(instance, type, oldProps, newProps, _internalHandle) {
    const windowId = instance.windowId;

    // Parse new props
    const newStyles: Record<string, any> = {};
    const newInputAttrs: Record<string, any> = {};
    const newEventListeners: Map<string, Function> = new Map();

    for (const key in newProps) {
      if (key === 'children' || key === 'key' || key === 'ref') continue;
      const value = newProps[key];
      if (value == null) continue;

      if (
        key.length >= 3 &&
        key[0] === 'o' &&
        key[1] === 'n' &&
        key.charCodeAt(2) >= 65 &&
        key.charCodeAt(2) <= 90
      ) {
        const eventName = key.slice(2).toLowerCase();
        newEventListeners.set(eventName, value);
      } else if (INPUT_ATTRS.has(key)) {
        newInputAttrs[key] = value;
      } else if (PROP_NAME_TO_KEY[key] !== undefined) {
        newStyles[key] = value;
      }
    }

    // Diff styles
    for (const [key, val] of Object.entries(newStyles)) {
      if (instance.styles[key] !== val) {
        setProp(windowId, instance.id, key, val);
      }
    }
    for (const key of Object.keys(instance.styles)) {
      if (!(key in newStyles)) {
        clearProp(windowId, instance.id, key);
      }
    }
    instance.styles = newStyles;

    // Diff input attributes
    for (const [key, val] of Object.entries(newInputAttrs)) {
      if (instance.inputAttrs[key] !== val) {
        setInputAttr(windowId, instance.id, key, val);
      }
    }
    instance.inputAttrs = newInputAttrs;

    // Diff event listeners
    for (const [event, newCb] of newEventListeners) {
      const oldCb = instance.eventListeners.get(event);
      if (oldCb !== newCb) {
        if (oldCb) eventManager.removeHandlerByName(instance.id, event, oldCb);
        eventManager.addHandlerByName(instance.id, event, newCb);
      }
    }
    for (const [event, cb] of instance.eventListeners) {
      if (!newEventListeners.has(event)) {
        eventManager.removeHandlerByName(instance.id, event, cb);
      }
    }
    if (newEventListeners.size > 0 && instance.eventListeners.size === 0) {
      core.setF32Prop(windowId, instance.id, PropKey.Interactive, 1);
    } else if (
      newEventListeners.size === 0 &&
      instance.eventListeners.size > 0
    ) {
      core.setF32Prop(windowId, instance.id, PropKey.Interactive, 0);
    }
    instance.eventListeners = newEventListeners;

    // Text content for text/p types
    if (type === 'text' || type === 'p') {
      const oldText = getTextContent(oldProps.children);
      const newText = getTextContent(newProps.children);
      if (oldText !== newText) {
        core.setText(windowId, instance.id, newText);
      }
    }
  },

  commitTextUpdate(instance, oldText, newText) {
    if (oldText !== newText) {
      core.setText(instance.windowId, instance.id, newText);
    }
  },

  detachDeletedInstance(instance) {
    eventManager.clearNode(instance.id);
    instance.eventListeners.clear();
  },

  hideInstance(instance) {
    core.setF32Prop(instance.windowId, instance.id, PropKey.Visible, 0);
  },

  unhideInstance(instance) {
    core.setF32Prop(instance.windowId, instance.id, PropKey.Visible, 1);
  },

  hideTextInstance(instance) {
    core.setF32Prop(instance.windowId, instance.id, PropKey.Visible, 0);
  },

  unhideTextInstance(instance) {
    core.setF32Prop(instance.windowId, instance.id, PropKey.Visible, 1);
  },

  resetTextContent(instance) {
    core.setText(instance.windowId, instance.id, '');
  },

  clearContainer(container) {
    console.log('[reconciler]: clear container');
  },

  getRootHostContext: () => ({}),
  getChildHostContext: (parentHostContext) => parentHostContext,
  getPublicInstance: (instance) => instance,

  prepareForCommit(container) {
    currentContainer = container;
    return null;
  },

  resetAfterCommit(container) {
    core.requestRedraw(container.window.id);
    currentContainer = null;
  },

  preparePortalMount: () => {},
  scheduleTimeout: (fn, delay) => setTimeout(fn, delay),
  cancelTimeout: (id) => clearTimeout(id),
  noTimeout: undefined,
  isPrimaryRenderer: true,
  getInstanceFromNode: () => null,
  beforeActiveInstanceBlur: () => {},
  afterActiveInstanceBlur: () => {},
  prepareScopeUpdate: () => {},
  getInstanceFromScope: () => null,
  supportsHydration: false,
  NotPendingTransition: undefined,
  HostTransitionContext: {
    $$typeof: Symbol.for('react.context'),
    _currentValue: null,
    _currentValue2: null,
  } as any,
  setCurrentUpdatePriority: (newPriority) => {
    currentPriority = newPriority;
  },
  getCurrentUpdatePriority: () => currentPriority,
  resolveUpdatePriority: () => DefaultEventPriority,
  resetFormInstance: () => {},
  requestPostPaintCallback: () => {},
  shouldAttemptEagerTransition: () => false,
  trackSchedulerEvent: () => {},
  resolveEventType: () => null,
  resolveEventTimeStamp: () => Date.now(),
  maySuspendCommit: () => false,
  preloadInstance: () => false,
  startSuspendingCommit: () => false,
  suspendInstance: () => {},
  waitForCommitToBeReady: () => null,
});

const roots = new Map<string, { root: any; container: Container }>();

export function render(window: Window, element: JSX.Element) {
  const rootNodeId = core.getRootNodeId(window.id);
  const container: Container = { window, rootNodeId };

  // Root node bubbles to the window
  eventManager.setParent(rootNodeId, window.eventId);

  const root = reconciler.createContainer(
    container,
    1,
    null,
    false,
    null,
    '',
    console.error,
    console.error,
    console.error,
    () => {},
  );

  roots.set(window.label, { root, container });
  reconciler.updateContainer(element, root, null, null);

  return {
    dispose: () => {
      reconciler.updateContainer(null, root, null, null);
      eventManager.removeParent(rootNodeId);
      roots.delete(window.label);
    },
  };
}

export function disposeAllRoots() {
  roots.clear();
}

export function clearEventRegistry() {
  eventManager.clear();
}
