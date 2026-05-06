import { CHECKBOX_ATTR_NAMES, INPUT_ATTR_NAMES } from './constants';
import { UzElement } from '../elements/base';
import { UzImageElement } from '../elements/image';
import { UzTextNode } from '../node';
import {
  assignNativeStyle,
  isEventProp,
  listenerKey,
  parseEventProp,
} from './utils';
import type { Window } from '../window';

const RESERVED_PROPS = new Set(['children', 'key', 'ref', 'id']);
const IMAGE_RESERVED_PROPS = new Set([...RESERVED_PROPS, 'src']);

interface ListenerEntry {
  name: string;
  handler: Function;
  capture: boolean;
}

export function createElement(
  window: Window,
  type: string,
  props: Record<string, any>,
): UzElement {
  const element = window.createElement(type) as UzElement;
  applyProps(element, props, {});
  return element;
}

export function createText(window: Window, text: string): UzTextNode {
  return window.createTextNode(text);
}

export function commitText(node: UzTextNode, text: string): void {
  node.textContent = text;
}

export function resetText(element: UzElement): void {
  element.textContent = '';
}

export function hide(element: UzElement): void {
  element.setAttribute('visibility', false);
}

export function unhide(element: UzElement): void {
  element.setAttribute('visibility', true);
}

export function applyProps(
  element: UzElement,
  newProps: Record<string, any>,
  oldProps: Record<string, any>,
): void {
  const oldBuckets = collectProps(element.type, oldProps);
  const newBuckets = collectProps(element.type, newProps);

  if (newProps.id !== oldProps.id) {
    element.id = newProps.id ?? null;
  }
  updateAttributes(element, oldBuckets.styles, newBuckets.styles);
  updateAttributes(element, oldBuckets.attrs, newBuckets.attrs);
  updateEvents(element, oldBuckets.events, newBuckets.events);
  if (element.type === 'text') {
    element.textContent = String(newProps.children ?? '');
  }
  if (element instanceof UzImageElement) {
    element.src = newProps.src;
  }
}

function collectProps(
  type: string,
  props: Record<string, any>,
): {
  styles: Record<string, any>;
  attrs: Record<string, any>;
  events: Map<string, ListenerEntry>;
} {
  const styles: Record<string, any> = {};
  const attrs: Record<string, any> = {};
  const events: Map<string, ListenerEntry> = new Map();
  const skip = skippedPropsForType(type);

  for (const key in props) {
    if (skip.has(key)) continue;
    const value = props[key];
    if (value == null) continue;
    if (isEventProp(key)) {
      const { name, capture } = parseEventProp(key);
      events.set(listenerKey(name, capture), {
        name,
        handler: value,
        capture,
      });
    } else if (attrNamesForType(type).has(key)) {
      attrs[key] = value;
    } else {
      assignNativeStyle(styles, key, value);
    }
  }

  return { styles, attrs, events };
}

function skippedPropsForType(type: string): Set<string> {
  if (type === 'image') return IMAGE_RESERVED_PROPS;
  return RESERVED_PROPS;
}

function attrNamesForType(type: string): Set<string> {
  if (type === 'input') return INPUT_ATTR_NAMES;
  if (type === 'checkbox') return CHECKBOX_ATTR_NAMES;
  return new Set();
}

function updateAttributes(
  element: UzElement,
  oldAttrs: Record<string, any>,
  newAttrs: Record<string, any>,
): void {
  for (const [key, value] of Object.entries(newAttrs)) {
    if (oldAttrs[key] !== value) {
      element.setAttribute(key, value);
    }
  }
  for (const key of Object.keys(oldAttrs)) {
    if (!(key in newAttrs)) {
      element.removeAttribute(key);
    }
  }
}

function updateEvents(
  element: UzElement,
  oldListeners: Map<string, ListenerEntry>,
  newListeners: Map<string, ListenerEntry>,
): void {
  const el = element as UzElement<any>;
  for (const [key, newEntry] of newListeners) {
    const old = oldListeners.get(key);
    if (
      !old ||
      old.handler !== newEntry.handler ||
      old.capture !== newEntry.capture
    ) {
      if (old) {
        el.off(old.name, old.handler as any, { capture: old.capture });
      }
      el.on(newEntry.name, newEntry.handler as any, {
        capture: newEntry.capture,
      });
    }
  }

  for (const [key, old] of oldListeners) {
    if (!newListeners.has(key)) {
      el.off(old.name, old.handler as any, { capture: old.capture });
    }
  }
}
