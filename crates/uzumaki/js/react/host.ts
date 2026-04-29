import { CHECKBOX_ATTR_NAMES, INPUT_ATTR_NAMES } from '../constants';
import core from '../core';
import { Element } from '../elements/element';
import { eventManager } from '../events';
import type { ListenerEntry } from '../types';
import {
  assignNativeStyle,
  isEventProp,
  listenerKey,
  parseEventProp,
} from '../utils';
import type { Window } from '../window';

const WINDOWS_DRIVE_PATH = /^[A-Za-z]:[\\/]/;
const URL_SCHEME = /^[A-Za-z][A-Za-z\d+\-.]*:/;
const RESERVED_PROPS = new Set(['children', 'key', 'ref', 'id']);
const IMAGE_LIFECYCLE_PROPS = new Set([
  'children',
  'key',
  'ref',
  'id',
  'src',
  'onLoad',
  'onLoadStart',
  'onError',
]);

interface ImageState {
  generation: number;
  disposed: boolean;
}

export interface HostInstance {
  node: Element;
  type: string;
  onChangeTextListener?: (ev: any) => void;
  onChangeListener?: (ev: any) => void;
  image?: ImageState;
}

export function createHostInstance(
  window: Window,
  type: string,
  props: Record<string, any>,
  textContent?: string,
): HostInstance {
  const node =
    type === '#text'
      ? window.createTextNode(textContent ?? '')
      : window.createElement(type);
  const instance: HostInstance = {
    node,
    type,
  };

  if (type === 'image') {
    instance.image = {
      generation: 0,
      disposed: false,
    };
  }

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
  container: Element,
  child: HostInstance,
): void {
  container.appendChild(child.node);
}

export function insertInContainerBefore(
  container: Element,
  child: HostInstance,
  before: HostInstance,
): void {
  container.insertBefore(child.node, before.node);
}

export function removeChildFromContainer(
  container: Element,
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
  instance.node.setAttribute('visibility', false);
}

export function unhideInstance(instance: HostInstance): void {
  instance.node.setAttribute('visibility', true);
}

export function disposeHostInstance(instance: HostInstance): void {
  if (instance.image) {
    instance.image.generation++;
    instance.image.disposed = true;
  }
  unbindSpecialEvents(instance);
  instance.node.destroy();
}

export function applyReactProps(
  instance: HostInstance,
  newProps: Record<string, any>,
  oldProps: Record<string, any>,
): void {
  const oldBuckets = collectProps(instance.type, oldProps);
  const newBuckets = collectProps(instance.type, newProps);

  updateAttributes(instance.node, oldBuckets.styles, newBuckets.styles);
  updateAttributes(instance.node, oldBuckets.attrs, newBuckets.attrs);
  updateEvents(instance, oldBuckets.events, newBuckets.events);
  updateSpecialEvents(instance, newProps, oldProps);
  syncInteractive(instance, newBuckets.events.size > 0);
  if (instance.type === 'text') {
    instance.node.textContent = String(newProps.children ?? '');
  }
  if (instance.type === 'image') {
    void updateImageSource(instance, newProps.src, newProps, oldProps);
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
  if (type === 'image') return IMAGE_LIFECYCLE_PROPS;
  if (type === 'input') {
    return new Set([...RESERVED_PROPS, 'onChangeText']);
  }
  if (type === 'checkbox') {
    return new Set([...RESERVED_PROPS, 'onChange']);
  }
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
  for (const [key, newEntry] of newListeners) {
    const old = oldListeners.get(key);
    if (
      !old ||
      old.handler !== newEntry.handler ||
      old.capture !== newEntry.capture
    ) {
      if (old) {
        eventManager.removeHandlerByName(
          instance.node.nodeId,
          old.name,
          old.handler,
          old.capture,
        );
      }
      eventManager.addHandlerByName(
        instance.node.nodeId,
        newEntry.name,
        newEntry.handler,
        newEntry.capture,
      );
    }
  }

  for (const [key, old] of oldListeners) {
    if (!newListeners.has(key)) {
      eventManager.removeHandlerByName(
        instance.node.nodeId,
        old.name,
        old.handler,
        old.capture,
      );
    }
  }
}

function updateSpecialEvents(
  instance: HostInstance,
  newProps: Record<string, any>,
  oldProps: Record<string, any>,
): void {
  if (
    instance.type === 'input' &&
    newProps.onChangeText !== oldProps.onChangeText
  ) {
    unbindOnChangeText(instance);
    if (typeof newProps.onChangeText === 'function') {
      const onChangeText = newProps.onChangeText;
      instance.onChangeTextListener = (ev: any) => {
        onChangeText(ev.value);
      };
      eventManager.addHandlerByName(
        instance.node.nodeId,
        'input',
        instance.onChangeTextListener,
      );
    }
  }

  if (instance.type === 'checkbox' && newProps.onChange !== oldProps.onChange) {
    unbindOnChange(instance);
    if (typeof newProps.onChange === 'function') {
      const onChange = newProps.onChange;
      instance.onChangeListener = (ev: any) => {
        onChange(ev.value === 'true');
      };
      eventManager.addHandlerByName(
        instance.node.nodeId,
        'input',
        instance.onChangeListener,
      );
    }
  }
}

function unbindSpecialEvents(instance: HostInstance): void {
  unbindOnChangeText(instance);
  unbindOnChange(instance);
}

function unbindOnChangeText(instance: HostInstance): void {
  if (instance.onChangeTextListener) {
    eventManager.removeHandlerByName(
      instance.node.nodeId,
      'input',
      instance.onChangeTextListener,
    );
    instance.onChangeTextListener = undefined;
  }
}

function unbindOnChange(instance: HostInstance): void {
  if (instance.onChangeListener) {
    eventManager.removeHandlerByName(
      instance.node.nodeId,
      'input',
      instance.onChangeListener,
    );
    instance.onChangeListener = undefined;
  }
}

function syncInteractive(
  instance: HostInstance,
  hasReactEvents: boolean,
): void {
  const interactive =
    hasReactEvents ||
    Boolean(instance.onChangeTextListener) ||
    Boolean(instance.onChangeListener);
  instance.node.setAttribute('interactive', interactive);
}

function isFilePath(source: string) {
  return (
    WINDOWS_DRIVE_PATH.test(source) ||
    source.startsWith('/') ||
    source.startsWith('./') ||
    source.startsWith('../') ||
    source.startsWith(String.raw`\\`)
  );
}

async function fetchImageBytes(source: string): Promise<Uint8Array> {
  if (isFilePath(source)) {
    return Deno.readFile(source);
  }

  if (URL_SCHEME.test(source)) {
    const url = new URL(source);
    if (url.protocol === 'file:') {
      return Deno.readFile(url);
    }
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(`HTTP ${response.status} while loading ${source}`);
    }
    return new Uint8Array(await response.arrayBuffer());
  }

  return Deno.readFile(source);
}

const inflightBytes = new Map<string, Promise<Uint8Array>>();

function loadImageBytes(source: string): Promise<Uint8Array> {
  let promise = inflightBytes.get(source);
  if (promise) return promise;
  promise = fetchImageBytes(source).catch((error) => {
    inflightBytes.delete(source);
    throw error;
  });
  inflightBytes.set(source, promise);
  return promise;
}

async function updateImageSource(
  instance: HostInstance,
  src: string | undefined,
  newProps: Record<string, any>,
  oldProps: Record<string, any>,
): Promise<void> {
  const image = instance.image;
  if (!image) return;
  if (typeof src !== 'string' || src.length === 0) {
    src = undefined;
  }
  const oldSrc =
    typeof oldProps.src === 'string' && oldProps.src.length > 0
      ? oldProps.src
      : undefined;
  if (src === oldSrc) return;

  const generation = ++image.generation;
  core.clearImageData(instance.node.windowId, instance.node.nodeId);
  core.requestRedraw(instance.node.windowId);

  if (!src) {
    return;
  }

  callImageHandler(newProps, 'onLoadStart', { src });

  if (
    core.applyCachedImage(instance.node.windowId, instance.node.nodeId, src)
  ) {
    if (!isImageLoadCurrent(instance, generation)) return;
    core.requestRedraw(instance.node.windowId);
    callImageHandler(newProps, 'onLoad', { src });
    return;
  }

  try {
    const data = await loadImageBytes(src);
    if (!isImageLoadCurrent(instance, generation)) return;
    core.setEncodedImageData(
      instance.node.windowId,
      instance.node.nodeId,
      src,
      data,
    );
    core.requestRedraw(instance.node.windowId);
    callImageHandler(newProps, 'onLoad', { src });
  } catch (error) {
    if (!isImageLoadCurrent(instance, generation)) return;
    core.clearImageData(instance.node.windowId, instance.node.nodeId);
    core.requestRedraw(instance.node.windowId);
    const message = error instanceof Error ? error.message : String(error);
    if (typeof newProps.onError === 'function') {
      callImageHandler(newProps, 'onError', { src, message });
    } else {
      console.error(`[uzumaki] Failed to load image "${src}": ${message}`);
    }
  }
}

function isImageLoadCurrent(
  instance: HostInstance,
  generation: number,
): boolean {
  return (
    !instance.image?.disposed &&
    !instance.node._window.isDisposed &&
    generation === instance.image?.generation
  );
}

function callImageHandler(
  props: Record<string, any>,
  name: 'onLoadStart' | 'onLoad' | 'onError',
  event: Record<string, unknown>,
): void {
  const handler = props[name];
  if (typeof handler !== 'function') return;
  try {
    handler(event);
  } catch (error) {
    console.error(`[uzumaki] ${name} handler threw:`, error);
  }
}
