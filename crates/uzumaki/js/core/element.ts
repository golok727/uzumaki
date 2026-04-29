import core, { type NativeElement } from '../core';
import { eventManager } from '../events';
import { getNode, registerNode, unregisterNode } from '../registry';
import type { NodeId } from '../types';
import type { Window } from '../window';

export class CoreElement {
  readonly native: NativeElement;
  readonly window: Window;
  readonly type: string;

  constructor(window: Window, native: NativeElement, type = native.nodeName) {
    this.window = window;
    this.native = native;
    this.type = type;
    registerNode(this);
  }

  static fromNative(
    window: Window,
    native: NativeElement | null,
  ): CoreElement | null {
    if (!native) return null;
    return getNode(native.id) ?? new CoreElement(window, native);
  }

  get id(): NodeId {
    return this.native.id;
  }

  get windowId(): number {
    return this.native.windowId;
  }

  get nodeType(): number {
    return this.native.nodeType;
  }

  get nodeName(): string {
    return this.native.nodeName;
  }

  get parentNode(): CoreElement | null {
    return CoreElement.fromNative(this.window, this.native.parentNode);
  }

  get firstChild(): CoreElement | null {
    return CoreElement.fromNative(this.window, this.native.firstChild);
  }

  get lastChild(): CoreElement | null {
    return CoreElement.fromNative(this.window, this.native.lastChild);
  }

  get nextSibling(): CoreElement | null {
    return CoreElement.fromNative(this.window, this.native.nextSibling);
  }

  get previousSibling(): CoreElement | null {
    return CoreElement.fromNative(this.window, this.native.previousSibling);
  }

  get textContent(): string | null {
    return this.native.textContent;
  }

  set textContent(text: string | null) {
    this.native.textContent = text ?? '';
  }

  appendChild<T extends CoreElement>(child: T): T {
    if (!this.window.isDisposed) {
      this.native.appendChild(child.native);
    }
    return child;
  }

  insertBefore<T extends CoreElement>(child: T, before: CoreElement | null): T {
    if (!this.window.isDisposed) {
      this.native.insertBefore(child.native, before?.native ?? null);
    }
    return child;
  }

  removeChild<T extends CoreElement>(child: T): T {
    if (!this.window.isDisposed) {
      this.native.removeChild(child.native);
    }
    return child;
  }

  setAttribute(name: string, value: unknown): void {
    if (value == null) {
      this.removeAttribute(name);
      return;
    }
    if (typeof value === 'boolean') {
      this.native.setBoolAttribute(name, value);
    } else if (typeof value === 'number') {
      this.native.setNumberAttribute(name, value);
    } else {
      this.native.setStrAttribute(name, String(value));
    }
  }

  setAttributes(attributes: Record<string, unknown>): void {
    for (const [key, value] of Object.entries(attributes)) {
      this.setAttribute(key, value);
    }
  }

  removeAttribute(name: string): void {
    this.native.removeAttribute(name);
  }

  getAttribute(name: string): unknown {
    return this.native.getAttribute(name);
  }

  destroy(): void {
    let child = this.firstChild;
    while (child) {
      const next = child.nextSibling;
      child.destroy();
      child = next;
    }
    eventManager.clearNode(this.id);
    unregisterNode(this.id);
  }
}

export function createNativeElement(
  window: Window,
  type: string,
): NativeElement {
  return core.createCoreElement(window.id, type);
}

export function createNativeTextNode(
  window: Window,
  text: string,
): NativeElement {
  return core.createCoreTextNode(window.id, text);
}

export function getNativeRootElement(window: Window): NativeElement {
  return core.getRootElement(window.id);
}
