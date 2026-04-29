import core, { type CoreNode } from '../core';
import { UzNode } from '../node';
import type { Window } from '../window';

export class Element extends UzNode {
  constructor(window: Window, native: CoreNode) {
    super(window, native);
  }

  focus(): void {
    core.focusElement(this._window.id, this._native.id);
  }

  setAttribute(name: string, value: unknown): void {
    if (value == null) {
      this.removeAttribute(name);
      return;
    }
    if (typeof value === 'boolean') {
      this._native.setBoolAttribute(name, value);
    } else if (typeof value === 'number') {
      this._native.setNumberAttribute(name, value);
    } else {
      this._native.setStrAttribute(name, String(value));
    }
  }

  setAttributes(attributes: Record<string, unknown>): void {
    for (const [key, value] of Object.entries(attributes)) {
      this.setAttribute(key, value);
    }
  }

  removeAttribute(name: string): void {
    this._native.removeAttribute(name);
  }

  getAttribute(name: string): unknown {
    return this._native.getAttribute(name);
  }
}

export function createNativeElement(window: Window, type: string): CoreNode {
  return core.createCoreElementNode(window.id, type);
}

export function getNativeRootNode(window: Window): CoreNode {
  return core.getRootNode(window.id);
}
