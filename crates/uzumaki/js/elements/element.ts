import core, { type CoreNode } from '../core';
import { eventManager } from '../events';
import {
  getElement,
  registerElement,
  unregisterElement,
} from '../registry';
import type { NodeId } from '../types';
import type { Window } from '../window';

export class Element {
  readonly _node: CoreNode;
  readonly _window: Window;
  readonly type: string;

  constructor(
    nodeName: string,
    window: Window,
    native = createNativeElement(window, nodeName),
  ) {
    this._window = window;
    this._node = native;
    this.type = nodeName; // this is wrong
    registerElement(this);
  }

  static fromNative(window: Window, native: CoreNode | null): Element | null {
    if (!native) return null;
    return (
      getElement(native.id) ?? new Element(native.nodeName, window, native)
    );
  }

  get nodeId(): NodeId {
    return this._node.id;
  }

  get windowId(): number {
    return this._node.windowId;
  }

  get nodeType(): number {
    return this._node.nodeType;
  }

  get nodeName(): string {
    return this._node.nodeName;
  }

  get parentNode(): Element | null {
    return Element.fromNative(this._window, this._node.parentNode);
  }

  get firstChild(): Element | null {
    return Element.fromNative(this._window, this._node.firstChild);
  }

  get lastChild(): Element | null {
    return Element.fromNative(this._window, this._node.lastChild);
  }

  get nextSibling(): Element | null {
    return Element.fromNative(this._window, this._node.nextSibling);
  }

  get previousSibling(): Element | null {
    return Element.fromNative(this._window, this._node.previousSibling);
  }

  get textContent(): string | null {
    return this._node.textContent;
  }

  set textContent(text: string | null) {
    this._node.textContent = text ?? '';
  }

  appendChild<T extends Element>(child: T): T {
    if (!this._window.isDisposed) {
      this._node.appendChild(child._node);
    }
    return child;
  }

  insertBefore<T extends Element>(child: T, before: Element | null): T {
    if (!this._window.isDisposed) {
      this._node.insertBefore(child._node, before?._node ?? null);
    }
    return child;
  }

  removeChild<T extends Element>(child: T): T {
    if (!this._window.isDisposed) {
      this._node.removeChild(child._node);
    }
    return child;
  }

  setAttribute(name: string, value: unknown): void {
    if (value == null) {
      this.removeAttribute(name);
      return;
    }
    if (typeof value === 'boolean') {
      this._node.setBoolAttribute(name, value);
    } else if (typeof value === 'number') {
      this._node.setNumberAttribute(name, value);
    } else {
      console.log(this._node);
      this._node.setStrAttribute(name, String(value));
    }
  }

  setAttributes(attributes: Record<string, unknown>): void {
    for (const [key, value] of Object.entries(attributes)) {
      this.setAttribute(key, value);
    }
  }

  removeAttribute(name: string): void {
    this._node.removeAttribute(name);
  }

  getAttribute(name: string): unknown {
    return this._node.getAttribute(name);
  }

  destroy(): void {
    let child = this.firstChild;
    while (child) {
      const next = child.nextSibling;
      child.destroy();
      child = next;
    }
    eventManager.clearNode(this.nodeId);
    unregisterElement(this.nodeId);
  }
}

export function createNativeElement(window: Window, type: string): CoreNode {
  return core.createCoreElement(window.id, type);
}

export function createNativeTextNode(window: Window, text: string): CoreNode {
  return core.createCoreTextNode(window.id, text);
}

export function getNativeRootElement(window: Window): CoreNode {
  return core.getRootElement(window.id);
}
