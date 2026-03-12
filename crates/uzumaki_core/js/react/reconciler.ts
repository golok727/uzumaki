import ReactReconciler, { type EventPriority } from 'react-reconciler';
import { DefaultEventPriority } from 'react-reconciler/constants';
import type { JSX } from './jsx/runtime';
import type { Window } from '..';
import * as napi from '../bindings';

// ── Known style props ────────────────────────────────────────────────

const STYLE_PROPS = new Set([
  'h',
  'w',
  'p',
  'px',
  'py',
  'pt',
  'pb',
  'pl',
  'pr',
  'm',
  'mx',
  'my',
  'mt',
  'mb',
  'ml',
  'mr',
  'flex',
  'flexDir',
  'flexGrow',
  'flexShrink',
  'items',
  'justify',
  'gap',
  'bg',
  'color',
  'fontSize',
  'fontWeight',
  'rounded',
  'roundedTL',
  'roundedTR',
  'roundedBR',
  'roundedBL',
  'border',
  'borderTop',
  'borderRight',
  'borderBottom',
  'borderLeft',
  'borderColor',
  'opacity',
  'cursor',
  'display',
]);

// ── Event registry ───────────────────────────────────────────────────

const eventRegistry = new Map<string, Map<string, Function>>();

export function registerEvent(nodeId: string, eventType: string, cb: Function) {
  if (!eventRegistry.has(nodeId)) eventRegistry.set(nodeId, new Map());
  eventRegistry.get(nodeId)!.set(eventType, cb);
}

export function unregisterEvents(nodeId: string) {
  eventRegistry.delete(nodeId);
}

export function dispatchEvent(
  nodeId: string,
  eventType: string,
  payload?: unknown,
) {
  eventRegistry.get(nodeId)?.get(eventType)?.(payload);
}

// ── UElement ─────────────────────────────────────────────────────────

class UElement {
  id: string;
  type: string;
  label: string;
  styles: Record<string, string> = {};
  hoverStyles: Record<string, string> = {};
  eventListeners: Map<string, Function> = new Map();
  children: UElement[] = [];

  constructor(
    id: string,
    type: string,
    label: string,
    props: Record<string, any>,
  ) {
    this.id = id;
    this.type = type;
    this.label = label;
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
      } else if (key.startsWith('hover:')) {
        this.hoverStyles[key.slice(6)] = String(value);
      } else if (key.startsWith('active:')) {
        // Store active styles as hover: prefix in styles map
        this.styles['active:' + key.slice(7)] = String(value);
      } else if (STYLE_PROPS.has(key)) {
        this.styles[key] = String(value);
      }
      // else: ignore
    }
  }
}

// ── Container ────────────────────────────────────────────────────────

type Container = {
  window: Window;
  rootNodeId: string;
};

// ── Helpers ──────────────────────────────────────────────────────────

function getLabel(container: Container): string {
  return container.window.label;
}

function getTextContent(children: any): string {
  if (children == null) return '';
  if (Array.isArray(children)) return children.join('');
  return String(children);
}

// ── Reconciler ───────────────────────────────────────────────────────

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
    const label = getLabel(rootContainer);
    if (type === 'text' || type === 'p') {
      // Text-like elements: create a native text node
      const text = getTextContent(props.children);
      const id = napi.createTextNode(label, text);
      const el = new UElement(id, type, label, props);

      // Apply styles
      for (const [key, val] of Object.entries(el.styles)) {
        napi.setProperty(label, id, key, val);
      }
      for (const [key, val] of Object.entries(el.hoverStyles)) {
        napi.setProperty(label, id, 'hover:' + key, val);
      }

      // Register events
      if (el.eventListeners.size > 0) {
        napi.setProperty(label, id, 'interactive', 'true');
        for (const [event, cb] of el.eventListeners) {
          registerEvent(id, event, cb);
        }
      }

      return el;
    }

    // View-like elements
    const id = napi.createElement(label, type);
    const el = new UElement(id, type, label, props);

    for (const [key, val] of Object.entries(el.styles)) {
      napi.setProperty(label, id, key, val);
    }
    for (const [key, val] of Object.entries(el.hoverStyles)) {
      napi.setProperty(label, id, 'hover:' + key, val);
    }

    if (el.eventListeners.size > 0) {
      napi.setProperty(label, id, 'interactive', 'true');
      for (const [event, cb] of el.eventListeners) {
        registerEvent(id, event, cb);
      }
    }

    return el;
  },

  createTextInstance(text, rootContainer) {
    const label = getLabel(rootContainer);
    const id = napi.createTextNode(label, text);
    return new UElement(id, '#text', label, {});
  },

  shouldSetTextContent(type) {
    return type === 'text' || type === 'p';
  },

  appendInitialChild(parent, child) {
    parent.children.push(child);
    napi.appendChild(parent.label, parent.id, child.id);
  },

  finalizeInitialChildren() {
    return false;
  },

  appendChildToContainer(container, child) {
    const label = getLabel(container);
    napi.appendChild(label, container.rootNodeId, child.id);
  },

  appendChild(parent, child) {
    parent.children.push(child);
    napi.appendChild(parent.label, parent.id, child.id);
  },

  insertBefore(parent, child, before) {
    const idx = parent.children.indexOf(before);
    if (idx >= 0) {
      parent.children.splice(idx, 0, child);
    } else {
      parent.children.push(child);
    }
    napi.insertBefore(parent.label, parent.id, child.id, before.id);
  },

  insertInContainerBefore(container, child, before) {
    const label = getLabel(container);
    napi.insertBefore(label, container.rootNodeId, child.id, before.id);
  },

  removeChild(parent, child) {
    const idx = parent.children.indexOf(child);
    if (idx >= 0) parent.children.splice(idx, 1);
    napi.removeChild(parent.label, parent.id, child.id);
    unregisterEvents(child.id);
  },

  removeChildFromContainer(container, child) {
    const label = getLabel(container);
    napi.removeChild(label, container.rootNodeId, child.id);
    unregisterEvents(child.id);
  },


  commitUpdate(
    instance,
    type,
    oldProps,
    newProps,
    _internalHandle,
  ) {
    const label = instance.label;

    // Parse new props
    const newStyles: Record<string, string> = {};
    const newHoverStyles: Record<string, string> = {};
    const newEventListeners: Map<string, Function> = new Map();

    for (const key in newProps) {
      if (key === 'children' || key === 'key' || key === 'ref') continue;
      const value = newProps[key];

      if (
        key.length >= 3 &&
        key[0] === 'o' &&
        key[1] === 'n' &&
        key.charCodeAt(2) >= 65 &&
        key.charCodeAt(2) <= 90
      ) {
        const eventName = key.slice(2).toLowerCase();
        newEventListeners.set(eventName, value);
      } else if (key.startsWith('hover:')) {
        newHoverStyles[key.slice(6)] = String(value);
      } else if (STYLE_PROPS.has(key)) {
        newStyles[key] = String(value);
      }
    }

    // Diff styles
    for (const [key, val] of Object.entries(newStyles)) {
      if (instance.styles[key] !== val) {
        napi.setProperty(label, instance.id, key, val);
      }
    }
    for (const key of Object.keys(instance.styles)) {
      if (!(key in newStyles) && !key.startsWith('active:')) {
        napi.setProperty(label, instance.id, key, '');
      }
    }
    instance.styles = newStyles;

    // Diff hover styles
    for (const [key, val] of Object.entries(newHoverStyles)) {
      if (instance.hoverStyles[key] !== val) {
        napi.setProperty(label, instance.id, 'hover:' + key, val);
      }
    }
    for (const key of Object.keys(instance.hoverStyles)) {
      if (!(key in newHoverStyles)) {
        napi.setProperty(label, instance.id, 'hover:' + key, '');
      }
    }
    instance.hoverStyles = newHoverStyles;

    // Diff event listeners
    for (const [event, cb] of newEventListeners) {
      registerEvent(instance.id, event, cb);
    }
    for (const [event] of instance.eventListeners) {
      if (!newEventListeners.has(event)) {
        eventRegistry.get(instance.id)?.delete(event);
      }
    }
    if (newEventListeners.size > 0 && instance.eventListeners.size === 0) {
      napi.setProperty(label, instance.id, 'interactive', 'true');
    } else if (
      newEventListeners.size === 0 &&
      instance.eventListeners.size > 0
    ) {
      napi.setProperty(label, instance.id, 'interactive', 'false');
    }
    instance.eventListeners = newEventListeners;

    // Text content for text/p types
    if (type === 'text' || type === 'p') {
      const oldText = getTextContent(oldProps.children);
      const newText = getTextContent(newProps.children);
      if (oldText !== newText) {
        napi.setText(label, instance.id, newText);
      }
    }
  },

  commitTextUpdate(instance, oldText, newText) {
    if (oldText !== newText) {
      napi.setText(instance.label, instance.id, newText);
    }
  },

  detachDeletedInstance(instance) {
    unregisterEvents(instance.id);
    instance.eventListeners.clear();
  },

  hideInstance(instance) {
    napi.setProperty(instance.label, instance.id, 'visible', 'false');
  },

  unhideInstance(instance) {
    napi.setProperty(instance.label, instance.id, 'visible', 'true');
  },

  hideTextInstance(instance) {
    napi.setProperty(instance.label, instance.id, 'visible', 'false');
  },

  unhideTextInstance(instance) {
    napi.setProperty(instance.label, instance.id, 'visible', 'true');
  },

  resetTextContent(instance) {
    napi.setText(instance.label, instance.id, '');
  },

  clearContainer(container) {
    // No-op: React handles removing children individually
  },

  getRootHostContext: () => ({}),
  getChildHostContext: (parentHostContext) => parentHostContext,
  getPublicInstance: (instance) => instance,

  prepareForCommit(container) {
    currentContainer = container;
    return null;
  },

  resetAfterCommit(container) {
    napi.requestRedraw(container.window.label);
    currentContainer = null;
  },

  preparePortalMount: () => { },
  scheduleTimeout: (fn, delay) => setTimeout(fn, delay),
  cancelTimeout: (id) => clearTimeout(id),
  noTimeout: undefined,
  isPrimaryRenderer: true,
  getInstanceFromNode: () => null,
  beforeActiveInstanceBlur: () => { },
  afterActiveInstanceBlur: () => { },
  prepareScopeUpdate: () => { },
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
  resetFormInstance: () => { },
  requestPostPaintCallback: () => { },
  shouldAttemptEagerTransition: () => false,
  trackSchedulerEvent: () => { },
  resolveEventType: () => null,
  resolveEventTimeStamp: () => Date.now(),
  maySuspendCommit: () => false,
  preloadInstance: () => false,
  startSuspendingCommit: () => false,
  suspendInstance: () => { },
  waitForCommitToBeReady: () => null,
});

// ── Render ───────────────────────────────────────────────────────────

export function render(window: Window, element: JSX.Element) {
  const rootNodeId = napi.getRootNodeId(window.label);
  const container: Container = { window, rootNodeId };

  const root = reconciler.createContainer(
    container,
    0,
    null,
    false,
    null,
    '',
    console.error,
    console.error,
    console.error,
    () => { },
  );

  reconciler.updateContainer(element, root, null, null);

  return {
    dispose: () => reconciler.updateContainer(null, root, null, null),
  };
}
