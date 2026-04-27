import { isValidElement as isReactElement } from 'react';
import ReactReconciler, { type EventPriority } from 'react-reconciler';
import { DefaultEventPriority } from 'react-reconciler/constants.js'; // fixme our runtime doesnt do probing for imports

import type { JSX } from './jsx/runtime';

import core from '../core';
import { eventManager } from '../events';
import type { NodeId } from '../types';
import { Window } from '../window';

const STYLE_ATTRIBUTE_NAMES = new Set([
  'w',
  'h',
  'minW',
  'minH',
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
  'display',
  'cursor',
  'hover:bg',
  'hover:color',
  'hover:opacity',
  'hover:borderColor',
  'active:bg',
  'active:color',
  'active:opacity',
  'active:borderColor',
  'scrollable',
  'selectable',
  'visibility',
  'overflowWrap',
  'wordBreak',
  'position',
  'top',
  'right',
  'bottom',
  'left',
  'translate',
  'translateX',
  'translateY',
  'rotate',
  'scale',
  'scaleX',
  'scaleY',
  'hover:translateX',
  'hover:translate',
  'hover:translateY',
  'hover:rotate',
  'hover:scale',
  'hover:scaleX',
  'hover:scaleY',
  'active:translateX',
  'active:translate',
  'active:translateY',
  'active:rotate',
  'active:scale',
  'active:scaleX',
  'active:scaleY',
]);

const INTRINSIC_ELEMENTS = new Set([
  'view',
  'text',
  'input',
  'checkbox',
  'button',
  /* 'canvas' */ // todo
]);

function setNativeProp(
  windowId: number,
  nodeId: any,
  propName: string,
  value: any,
): void {
  if (typeof value === 'boolean') {
    core.setBoolAttribute(windowId, nodeId, propName, value);
  } else if (typeof value === 'number') {
    core.setNumberAttribute(windowId, nodeId, propName, value);
  } else {
    core.setStrAttribute(windowId, nodeId, propName, String(value));
  }
}

function clearNativeProp(
  windowId: number,
  nodeId: any,
  propName: string,
): void {
  core.clearAttribute(windowId, nodeId, propName);
}

function splitVariantProp(key: string): { prefix: string; name: string } {
  const idx = key.indexOf(':');
  if (idx === -1) return { prefix: '', name: key };
  return { prefix: key.slice(0, idx + 1), name: key.slice(idx + 1) };
}

function readPair(value: any): [number, number] {
  if (Array.isArray(value)) {
    const x = Number(value[0] ?? 0);
    const y = Number(value[1] ?? x);
    return [x, y];
  }
  if (typeof value === 'object' && value !== null) {
    const x = Number(value.x ?? 0);
    const y = Number(value.y ?? 0);
    return [x, y];
  }
  const n = Number(value ?? 0);
  return [n, n];
}

function assignNativeStyle(
  styles: Record<string, any>,
  key: string,
  value: any,
): void {
  const { prefix, name } = splitVariantProp(key);
  if (name === 'translate') {
    const [x, y] = readPair(value);
    styles[`${prefix}translateX`] = x;
    styles[`${prefix}translateY`] = y;
    return;
  }
  if (name === 'scale') {
    const [x, y] = readPair(value);
    styles[`${prefix}scaleX`] = x;
    styles[`${prefix}scaleY`] = y;
    return;
  }
  styles[key] = value;
}

function isEventProp(key: string): boolean {
  const thirdCharacterCode = key.codePointAt(2);
  return (
    key.length >= 3 &&
    key[0] === 'o' &&
    key[1] === 'n' &&
    thirdCharacterCode !== undefined &&
    thirdCharacterCode >= 65 &&
    thirdCharacterCode <= 90
  );
}

interface ListenerEntry {
  name: string;
  handler: Function;
  capture: boolean;
}

function listenerKey(name: string, capture: boolean): string {
  return `${name}:${capture ? 'capture' : 'bubble'}`;
}

function parseEventProp(key: string): { name: string; capture: boolean } {
  const raw = key.slice(2); // strip "on"
  if (raw.endsWith('Capture')) {
    return { name: raw.slice(0, -7).toLowerCase(), capture: true };
  }
  return { name: raw.toLowerCase(), capture: false };
}

abstract class BaseElement<
  TProps extends Record<string, any> = Record<string, any>,
> {
  readonly id: any;
  readonly type: string;
  readonly window: Window;
  readonly windowId: number;
  styles: Record<string, any> = {};
  /** Keyed by stable event identity (name + phase). */
  eventListeners: Map<string, ListenerEntry> = new Map();
  children: BaseElement[] = [];
  parent: BaseElement | null = null;

  constructor(id: any, type: string, window: Window) {
    this.id = id;
    this.type = type;
    this.window = window;
    this.windowId = window.id;
  }

  abstract commitUpdate(newProps: TProps, oldProps: TProps): void;

  applyStyles(): void {
    for (const [key, val] of Object.entries(this.styles)) {
      setNativeProp(this.windowId, this.id, key, val);
    }
  }

  applyEvents(): void {
    if (this.eventListeners.size > 0) {
      core.setBoolAttribute(this.windowId, this.id, 'interactive', true);
      for (const entry of this.eventListeners.values()) {
        eventManager.addHandlerByName(
          this.id,
          entry.name,
          entry.handler,
          entry.capture,
        );
      }
    }
  }

  updateStyles(newStyles: Record<string, any>): void {
    for (const [key, val] of Object.entries(newStyles)) {
      if (this.styles[key] !== val) {
        setNativeProp(this.windowId, this.id, key, val);
      }
    }
    for (const key of Object.keys(this.styles)) {
      if (!(key in newStyles)) {
        clearNativeProp(this.windowId, this.id, key);
      }
    }
    this.styles = newStyles;
  }

  updateEvents(newListeners: Map<string, ListenerEntry>): void {
    for (const [key, newEntry] of newListeners) {
      const old = this.eventListeners.get(key);
      if (
        !old ||
        old.handler !== newEntry.handler ||
        old.capture !== newEntry.capture
      ) {
        if (old)
          eventManager.removeHandlerByName(
            this.id,
            old.name,
            old.handler,
            old.capture,
          );
        eventManager.addHandlerByName(
          this.id,
          newEntry.name,
          newEntry.handler,
          newEntry.capture,
        );
      }
    }
    for (const [key, old] of this.eventListeners) {
      if (!newListeners.has(key)) {
        eventManager.removeHandlerByName(
          this.id,
          old.name,
          old.handler,
          old.capture,
        );
      }
    }

    if (newListeners.size > 0 && this.eventListeners.size === 0) {
      core.setBoolAttribute(this.windowId, this.id, 'interactive', true);
    } else if (newListeners.size === 0 && this.eventListeners.size > 0) {
      core.setBoolAttribute(this.windowId, this.id, 'interactive', false);
    }
    this.eventListeners = newListeners;
  }

  destroy(): void {
    for (const child of this.children) {
      child.destroy();
    }
    eventManager.clearNode(this.id);
    this.eventListeners.clear();
    this.children = [];
  }
}

class ViewElement extends BaseElement<Record<string, any>> {
  constructor(window: Window, type: string, props: Record<string, any>) {
    const id = core.createElement(window.id, type);
    super(id, type, window);
    this.parseProps(props);
    this.applyStyles();
    this.applyEvents();
  }

  private parseProps(props: Record<string, any>): void {
    for (const key in props) {
      if (key === 'children' || key === 'key' || key === 'ref') continue;
      const value = props[key];
      if (value == null) continue;
      if (isEventProp(key)) {
        const { name, capture } = parseEventProp(key);
        this.eventListeners.set(listenerKey(name, capture), {
          name,
          handler: value,
          capture,
        });
      } else if (isNativeAttribute(key)) {
        assignNativeStyle(this.styles, key, value);
      }
    }
  }

  commitUpdate(
    newProps: Record<string, any>,
    _oldProps: Record<string, any>,
  ): void {
    const newStyles: Record<string, any> = {};
    const newEvents: Map<string, ListenerEntry> = new Map();

    for (const key in newProps) {
      if (key === 'children' || key === 'key' || key === 'ref') continue;
      const value = newProps[key];
      if (value == null) continue;
      if (isEventProp(key)) {
        const { name, capture } = parseEventProp(key);
        newEvents.set(listenerKey(name, capture), {
          name,
          handler: value,
          capture,
        });
      } else if (isNativeAttribute(key)) {
        assignNativeStyle(newStyles, key, value);
      }
    }

    this.updateStyles(newStyles);
    this.updateEvents(newEvents);
  }
}

import { __DEV__ } from '../constants';

const INPUT_ATTR_NAMES = new Set([
  'value',
  'placeholder',
  'disabled',
  'maxLength',
  'multiline',
  'secure',
]);
const CHECKBOX_ATTR_NAMES = new Set(['checked']);

function isNativeAttribute(key: string): boolean {
  return (
    STYLE_ATTRIBUTE_NAMES.has(key) ||
    INPUT_ATTR_NAMES.has(key) ||
    CHECKBOX_ATTR_NAMES.has(key)
  );
}

class InputElement extends BaseElement<Record<string, any>> {
  inputAttrs: Record<string, any> = {};
  private onChangeText: ((value: string) => void) | undefined;
  private onChangeTextListener: ((ev: any) => void) | null = null;

  constructor(window: Window, props: Record<string, any>) {
    const id = core.createElement(window.id, 'input');
    super(id, 'input', window);
    this.parseProps(props);
    this.applyStyles();
    this.applyInputAttrs();
    this.applyEvents();
    this.bindOnChangeText(props.onChangeText);
  }

  private parseProps(props: Record<string, any>): void {
    for (const key in props) {
      if (
        key === 'children' ||
        key === 'key' ||
        key === 'ref' ||
        key === 'onChangeText'
      )
        continue;
      const value = props[key];
      if (value == null) continue;
      if (isEventProp(key)) {
        const { name, capture } = parseEventProp(key);
        this.eventListeners.set(listenerKey(name, capture), {
          name,
          handler: value,
          capture,
        });
      } else if (INPUT_ATTR_NAMES.has(key)) {
        this.inputAttrs[key] = value;
      } else if (isNativeAttribute(key)) {
        assignNativeStyle(this.styles, key, value);
      }
    }
  }

  private applyInputAttrs(): void {
    for (const [key, val] of Object.entries(this.inputAttrs)) {
      setNativeProp(this.windowId, this.id, key, val);
    }
  }

  private bindOnChangeText(
    onChangeText: ((value: string) => void) | undefined,
  ): void {
    if (!onChangeText) return;
    this.onChangeText = onChangeText;
    this.onChangeTextListener = (ev: any) => {
      this.onChangeText?.(ev.value);
    };
    eventManager.addHandlerByName(this.id, 'input', this.onChangeTextListener);
    core.setBoolAttribute(this.windowId, this.id, 'interactive', true);
  }

  private unbindOnChangeText(): void {
    if (this.onChangeTextListener) {
      eventManager.removeHandlerByName(
        this.id,
        'input',
        this.onChangeTextListener,
      );
      this.onChangeTextListener = null;
    }
    this.onChangeText = undefined;
  }

  commitUpdate(
    newProps: Record<string, any>,
    _oldProps: Record<string, any>,
  ): void {
    const newStyles: Record<string, any> = {};
    const newInputAttrs: Record<string, any> = {};
    const newEvents: Map<string, ListenerEntry> = new Map();

    for (const key in newProps) {
      if (
        key === 'children' ||
        key === 'key' ||
        key === 'ref' ||
        key === 'onChangeText'
      )
        continue;
      const value = newProps[key];
      if (value == null) continue;
      if (isEventProp(key)) {
        const { name, capture } = parseEventProp(key);
        newEvents.set(listenerKey(name, capture), {
          name,
          handler: value,
          capture,
        });
      } else if (INPUT_ATTR_NAMES.has(key)) {
        newInputAttrs[key] = value;
      } else if (isNativeAttribute(key)) {
        assignNativeStyle(newStyles, key, value);
      }
    }

    this.updateStyles(newStyles);
    this.updateEvents(newEvents);

    const newOnChangeText = newProps.onChangeText;
    if (newOnChangeText !== this.onChangeText) {
      this.unbindOnChangeText();
      this.bindOnChangeText(newOnChangeText);
    }

    for (const [key, val] of Object.entries(newInputAttrs)) {
      if (this.inputAttrs[key] !== val) {
        setNativeProp(this.windowId, this.id, key, val);
      }
    }
    for (const key of Object.keys(this.inputAttrs)) {
      if (!(key in newInputAttrs)) {
        clearNativeProp(this.windowId, this.id, key);
      }
    }
    this.inputAttrs = newInputAttrs;
  }

  override destroy(): void {
    this.unbindOnChangeText();
    super.destroy();
  }
}

class CheckboxElement extends BaseElement<Record<string, any>> {
  checkboxAttrs: Record<string, any> = {};
  private onChange: ((checked: boolean) => void) | undefined;
  private onChangeListener: ((ev: any) => void) | null = null;

  constructor(window: Window, props: Record<string, any>) {
    const id = core.createElement(window.id, 'checkbox');
    super(id, 'checkbox', window);
    this.parseProps(props);
    this.applyStyles();
    this.applyCheckboxAttrs();
    this.applyEvents();
    this.bindOnChange(props.onChange);
  }

  private parseProps(props: Record<string, any>): void {
    for (const key in props) {
      if (
        key === 'children' ||
        key === 'key' ||
        key === 'ref' ||
        key === 'onChange'
      )
        continue;
      const value = props[key];
      if (value == null) continue;
      if (isEventProp(key)) {
        const { name, capture } = parseEventProp(key);
        this.eventListeners.set(listenerKey(name, capture), {
          name,
          handler: value,
          capture,
        });
      } else if (CHECKBOX_ATTR_NAMES.has(key)) {
        this.checkboxAttrs[key] = value;
      } else if (isNativeAttribute(key)) {
        assignNativeStyle(this.styles, key, value);
      }
    }
  }

  private applyCheckboxAttrs(): void {
    for (const [key, val] of Object.entries(this.checkboxAttrs)) {
      setNativeProp(this.windowId, this.id, key, val);
    }
  }

  private bindOnChange(
    onChange: ((checked: boolean) => void) | undefined,
  ): void {
    if (!onChange) return;
    this.onChange = onChange;
    this.onChangeListener = (ev: any) => {
      this.onChange?.(ev.value === 'true');
    };
    eventManager.addHandlerByName(this.id, 'input', this.onChangeListener);
    core.setBoolAttribute(this.windowId, this.id, 'interactive', true);
  }

  private unbindOnChange(): void {
    if (this.onChangeListener) {
      eventManager.removeHandlerByName(this.id, 'input', this.onChangeListener);
      this.onChangeListener = null;
    }
    this.onChange = undefined;
  }

  commitUpdate(
    newProps: Record<string, any>,
    _oldProps: Record<string, any>,
  ): void {
    const newStyles: Record<string, any> = {};
    const newCheckboxAttrs: Record<string, any> = {};
    const newEvents: Map<string, ListenerEntry> = new Map();

    for (const key in newProps) {
      if (
        key === 'children' ||
        key === 'key' ||
        key === 'ref' ||
        key === 'onChange'
      )
        continue;
      const value = newProps[key];
      if (value == null) continue;
      if (isEventProp(key)) {
        const { name, capture } = parseEventProp(key);
        newEvents.set(listenerKey(name, capture), {
          name,
          handler: value,
          capture,
        });
      } else if (CHECKBOX_ATTR_NAMES.has(key)) {
        newCheckboxAttrs[key] = value;
      } else if (isNativeAttribute(key)) {
        assignNativeStyle(newStyles, key, value);
      }
    }

    this.updateStyles(newStyles);
    this.updateEvents(newEvents);

    const newOnChange = newProps.onChange;
    if (newOnChange !== this.onChange) {
      this.unbindOnChange();
      this.bindOnChange(newOnChange);
    }

    for (const [key, val] of Object.entries(newCheckboxAttrs)) {
      if (this.checkboxAttrs[key] !== val) {
        setNativeProp(this.windowId, this.id, key, val);
      }
    }
    for (const key of Object.keys(this.checkboxAttrs)) {
      if (!(key in newCheckboxAttrs)) {
        clearNativeProp(this.windowId, this.id, key);
      }
    }
    this.checkboxAttrs = newCheckboxAttrs;
  }

  override destroy(): void {
    this.unbindOnChange();
    super.destroy();
  }
}

class TextElement extends BaseElement<Record<string, any>> {
  textContent: string;

  constructor(
    window: Window,
    type: string,
    text: string,
    props: Record<string, any>,
  ) {
    const id = core.createTextNode(window.id, text);
    super(id, type, window);
    this.textContent = text;
    this.parseProps(props);
    this.applyStyles();
    this.applyEvents();
  }

  private parseProps(props: Record<string, any>): void {
    for (const key in props) {
      if (key === 'children' || key === 'key' || key === 'ref') continue;
      const value = props[key];
      if (value == null) continue;
      if (isEventProp(key)) {
        const { name, capture } = parseEventProp(key);
        this.eventListeners.set(listenerKey(name, capture), {
          name,
          handler: value,
          capture,
        });
      } else if (isNativeAttribute(key)) {
        assignNativeStyle(this.styles, key, value);
      }
    }
  }

  setText(text: string): void {
    if (this.textContent !== text) {
      this.textContent = text;
      core.setText(this.windowId, this.id, text);
    }
  }

  commitUpdate(
    newProps: Record<string, any>,
    _oldProps: Record<string, any>,
  ): void {
    const newStyles: Record<string, any> = {};
    const newEvents: Map<string, ListenerEntry> = new Map();

    for (const key in newProps) {
      if (key === 'children' || key === 'key' || key === 'ref') continue;
      const value = newProps[key];
      if (value == null) continue;
      if (isEventProp(key)) {
        const { name, capture } = parseEventProp(key);
        newEvents.set(listenerKey(name, capture), {
          name,
          handler: value,
          capture,
        });
      } else if (isNativeAttribute(key)) {
        assignNativeStyle(newStyles, key, value);
      }
    }

    this.updateStyles(newStyles);
    this.updateEvents(newEvents);

    const newText = getTextContent(newProps.children);
    this.setText(newText);
  }
}

type Container = {
  window: Window;
  rootNodeId: NodeId;
};

function getWindowId(container: Container): number {
  return container.window.id;
}

/**
 * Get text content of a <text> node. will throw an error if you nest a react element inside this
 */
function getTextContent(children: any): string {
  if (children == null) return '';
  if (Array.isArray(children)) {
    return children
      .map((child) => {
        if (__DEV__ && isReactElement(child)) {
          throw new Error(
            `[uzumaki] <text> received a React element as a child (<${child.type}>). ` +
              `Only strings and numbers are allowed inside <text>.`,
          );
        }
        return child == null ? '' : String(child);
      })
      .join('');
  }

  if (__DEV__ && isReactElement(children)) {
    throw new Error(
      `[uzumaki] <text> received a React element as a child (<${children.type}>). ` +
        `Only strings and numbers are allowed inside <text>.`,
    );
  }

  return String(children);
}

function isTextType(type: string): boolean {
  return type === 'text';
}

function createElementInstance(
  type: string,
  props: Record<string, any>,
  window: Window,
): BaseElement {
  if (!INTRINSIC_ELEMENTS.has(type)) {
    throw new Error(
      `[uzumaki] Unknown intrinsic element: <${type}>. Did you mean <view>?`,
    );
  }

  if (type === 'input') {
    return new InputElement(window, props);
  }

  if (type === 'checkbox') {
    return new CheckboxElement(window, props);
  }

  if (isTextType(type)) {
    return new TextElement(window, type, getTextContent(props.children), props);
  }
  return new ViewElement(window, type, props);
}

type Type = string;
type Props = Record<string, any>;
type Instance = BaseElement;
type TextInstance = TextElement;
type SuspenseInstance = any;
type HydratableInstance = any;
type FormInstance = any;
type PublicInstance = BaseElement;
type HostContext = {};
type ChildSet = any;
type TimeoutHandle = ReturnType<typeof setTimeout>;
type NoTimeout = undefined;
type TransitionStatus = any;

let currentPriority: EventPriority = DefaultEventPriority;
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
    return createElementInstance(type, props, rootContainer.window);
  },

  createTextInstance(text, rootContainer) {
    return new TextElement(rootContainer.window, '#text', text, {});
  },

  shouldSetTextContent(type) {
    return isTextType(type);
  },

  appendInitialChild(parent, child) {
    parent.children.push(child);
    child.parent = parent;
    if (parent.window.isDisposed) return;
    core.appendChild(parent.windowId, parent.id, child.id);
  },

  finalizeInitialChildren() {
    return false;
  },

  appendChildToContainer(container, child) {
    child.parent = null;
    if (container.window.isDisposed) return;
    const windowId = getWindowId(container);
    core.appendChild(windowId, container.rootNodeId, child.id);
  },

  appendChild(parent, child) {
    parent.children.push(child);
    child.parent = parent;
    if (parent.window.isDisposed) return;
    core.appendChild(parent.windowId, parent.id, child.id);
  },

  insertBefore(parent, child, before) {
    const idx = parent.children.indexOf(before);
    if (idx === -1) {
      parent.children.push(child);
    } else {
      parent.children.splice(idx, 0, child);
    }
    child.parent = parent;
    if (parent.window.isDisposed) return;
    core.insertBefore(parent.windowId, parent.id, child.id, before.id);
  },

  insertInContainerBefore(container, child, before) {
    child.parent = null;
    if (container.window.isDisposed) return;
    const windowId = getWindowId(container);
    core.insertBefore(windowId, container.rootNodeId, child.id, before.id);
  },

  removeChild(parent, child) {
    const idx = parent.children.indexOf(child);
    if (idx !== -1) parent.children.splice(idx, 1);
    child.parent = null;
    if (!parent.window.isDisposed) {
      core.removeChild(parent.windowId, parent.id, child.id);
    }
    child.destroy();
  },

  removeChildFromContainer(container, child) {
    child.parent = null;
    if (!container.window.isDisposed) {
      const windowId = getWindowId(container);
      core.removeChild(windowId, container.rootNodeId, child.id);
    }
    child.destroy();
  },

  commitUpdate(instance, _type, oldProps, newProps, _internalHandle) {
    if (instance.window.isDisposed) return;
    instance.commitUpdate(newProps, oldProps);
  },

  commitTextUpdate(instance, _oldText, newText) {
    if (instance.window.isDisposed) return;
    instance.setText(newText);
  },

  detachDeletedInstance(instance) {
    instance.destroy();
  },

  hideInstance(instance) {
    core.setBoolAttribute(instance.windowId, instance.id, 'visibility', false);
  },

  unhideInstance(instance) {
    core.setBoolAttribute(instance.windowId, instance.id, 'visibility', true);
  },

  hideTextInstance(instance) {
    core.setBoolAttribute(instance.windowId, instance.id, 'visibility', false);
  },

  unhideTextInstance(instance) {
    core.setBoolAttribute(instance.windowId, instance.id, 'visibility', true);
  },

  resetTextContent(instance) {
    core.setText(instance.windowId, instance.id, '');
  },

  clearContainer(container) {
    const windowId = getWindowId(container);
    core.resetDom(windowId);
  },

  getRootHostContext: () => ({}),
  getChildHostContext: (parentHostContext) => parentHostContext,
  getPublicInstance: (instance) => instance,

  prepareForCommit(_container) {
    return null;
  },

  resetAfterCommit(container) {
    core.requestRedraw(container.window.id);
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

  function dispose() {
    reconciler.updateContainer(null, root, null, null);
    roots.delete(window.label);
  }

  window.addDisposable(dispose);

  return {
    dispose,
  };
}

export function disposeRoot(windowLabel: string) {
  const entry = roots.get(windowLabel);
  if (entry) {
    reconciler.updateContainer(null, entry.root, null, null);
    roots.delete(windowLabel);
  }
}

export function disposeAllRoots() {
  roots.clear();
}

export function clearEventRegistry() {
  eventManager.clear();
}
