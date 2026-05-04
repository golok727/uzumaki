import { CHECKBOX_ATTR_NAMES, INPUT_ATTR_NAMES } from '../constants';
import { Element } from '../elements/element';
import { UzImageElement } from '../elements/image';
import { UzNode } from '../node';
import type { ListenerEntry } from '../types';
import {
  assignNativeStyle,
  isEventProp,
  listenerKey,
  parseEventProp,
} from '../utils';
import type { Window } from '../window';

const RESERVED_PROPS = new Set(['children', 'key', 'ref', 'id']);
const IMAGE_RESERVED_PROPS = new Set([...RESERVED_PROPS, 'src']);

export interface HostInstance {
  /** The DOM node — Element for intrinsics, UzNode for #text instances. */
  node: UzNode;
  type: string;
}

export function createHostInstance(
  window: Window,
  type: string,
  props: Record<string, any>,
  textContent?: string,
): HostInstance {
  if (type === '#text') {
    const node = window.createTextNode(textContent ?? '');
    return { node, type };
  }

  const node = window.createElement(type);
  const instance: HostInstance = { node, type };
  applyReactProps(instance, props, {});
  return instance;
}

export function appendChild(parent: HostInstance, child: HostInstance): void {
  parent.node.appendChild(child.node);
}

export function insertBefore(
  parent: HostInstance,
  child: HostInstance,
  before: HostInstance,
): void {
  parent.node.insertBefore(child.node, before.node);
}

export function removeChild(parent: HostInstance, child: HostInstance): void {
  disposeHostInstance(child);
  parent.node.removeChild(child.node);
}

export function appendChildToContainer(
  container: UzNode,
  child: HostInstance,
): void {
  container.appendChild(child.node);
}

export function insertInContainerBefore(
  container: UzNode,
  child: HostInstance,
  before: HostInstance,
): void {
  container.insertBefore(child.node, before.node);
}

export function removeChildFromContainer(
  container: UzNode,
  child: HostInstance,
): void {
  disposeHostInstance(child);
  container.removeChild(child.node);
}

export function commitTextUpdate(instance: HostInstance, text: string): void {
  instance.node.textContent = text;
}

export function resetTextContent(instance: HostInstance): void {
  instance.node.textContent = '';
}

export function hideInstance(instance: HostInstance): void {
  if (instance.node instanceof Element) {
    instance.node.setAttribute('visibility', false);
  }
}

export function unhideInstance(instance: HostInstance): void {
  if (instance.node instanceof Element) {
    instance.node.setAttribute('visibility', true);
  }
}

export function disposeHostInstance(instance: HostInstance): void {
  instance.node.destroy();
}

export function applyReactProps(
  instance: HostInstance,
  newProps: Record<string, any>,
  oldProps: Record<string, any>,
): void {
  if (!(instance.node instanceof Element)) return;
  const node = instance.node;

  const oldBuckets = collectProps(instance.type, oldProps);
  const newBuckets = collectProps(instance.type, newProps);

  if (newProps.id !== oldProps.id) {
    node.id = newProps.id ?? null;
  }
  updateAttributes(node, oldBuckets.styles, newBuckets.styles);
  updateAttributes(node, oldBuckets.attrs, newBuckets.attrs);
  updateEvents(instance, oldBuckets.events, newBuckets.events);
  syncInteractive(instance, newBuckets.events.size > 0);
  if (instance.type === 'text') {
    node.textContent = String(newProps.children ?? '');
  }
  if (instance.type === 'image' && node instanceof UzImageElement) {
    node.src = newProps.src;
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
  node: Element,
  oldAttrs: Record<string, any>,
  newAttrs: Record<string, any>,
): void {
  for (const [key, value] of Object.entries(newAttrs)) {
    if (oldAttrs[key] !== value) {
      node.setAttribute(key, value);
    }
  }
  for (const key of Object.keys(oldAttrs)) {
    if (!(key in newAttrs)) {
      node.removeAttribute(key);
    }
  }
}

function updateEvents(
  instance: HostInstance,
  oldListeners: Map<string, ListenerEntry>,
  newListeners: Map<string, ListenerEntry>,
): void {
  if (!(instance.node instanceof Element)) return;
  const el = instance.node as Element<any>;
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

function syncInteractive(
  instance: HostInstance,
  hasReactEvents: boolean,
): void {
  if (!(instance.node instanceof Element)) return;
  instance.node.setAttribute('interactive', hasReactEvents);
}
