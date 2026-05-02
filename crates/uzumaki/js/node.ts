import core, { type CoreNode } from './core';
import { getNode, registerNode, unregisterNode } from './registry';
// registry is keyed by (windowId, nodeId) since node ids can collide across windows.
import type { NodeId } from './types';
import type { Window } from './window';

export class UzNode {
  readonly _native: CoreNode;
  readonly _window: Window;
  interactionType?: 'button' | 'input' | 'container' | 'text';

  constructor(window: Window, native: CoreNode) {
    this._window = window;
    this._native = native;
    this.interactionType = inferInteractionType(native.nodeName);
    registerNode(this);
  }

  static fromNative(window: Window, native: CoreNode | null): UzNode | null {
    if (!native) return null;
    return getNode(window.id, native.id) ?? null;
  }

  get nodeId(): NodeId {
    return this._native.id;
  }

  get windowId(): number {
    return this._native.windowId;
  }

  get nodeType(): number {
    return this._native.nodeType;
  }

  get nodeName(): string {
    return this._native.nodeName;
  }

  get parentNode(): UzNode | null {
    return UzNode.fromNative(this._window, this._native.parentNode);
  }

  get firstChild(): UzNode | null {
    return UzNode.fromNative(this._window, this._native.firstChild);
  }

  get lastChild(): UzNode | null {
    return UzNode.fromNative(this._window, this._native.lastChild);
  }

  get nextSibling(): UzNode | null {
    return UzNode.fromNative(this._window, this._native.nextSibling);
  }

  get previousSibling(): UzNode | null {
    return UzNode.fromNative(this._window, this._native.previousSibling);
  }

  get textContent(): string | null {
    return this._native.textContent;
  }

  set textContent(text: string | null) {
    this._native.textContent = text ?? '';
  }

  appendChild<T extends UzNode>(child: T): T {
    if (!this._window.isDisposed) {
      this._native.appendChild(child._native);
    }
    return child;
  }

  insertBefore<T extends UzNode>(child: T, before: UzNode | null): T {
    if (!this._window.isDisposed) {
      this._native.insertBefore(child._native, before?._native ?? null);
    }
    return child;
  }

  removeChild<T extends UzNode>(child: T): T {
    if (!this._window.isDisposed) {
      this._native.removeChild(child._native);
    }
    return child;
  }

  destroy(): void {
    let child = this.firstChild;
    while (child) {
      const next = child.nextSibling;
      child.destroy();
      child = next;
    }
    unregisterNode(this.windowId, this.nodeId);
  }
}

export class UzTextNode extends UzNode {
  constructor(window: Window, text: string) {
    super(window, core.createTextNode(window.id, text));
  }
}

function inferInteractionType(
  nodeName: string,
): 'button' | 'input' | 'container' | 'text' {
  switch (nodeName.toLowerCase()) {
    case 'button':
      return 'button';

    case 'input':
    case 'textarea':
      return 'input';

    case 'span':
    case 'p':
    case 'text':
      return 'text';

    default:
      return 'container';
  }
}
