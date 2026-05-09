import core, { type CoreNode } from 'ext:uzumaki/core.ts';
import { getNode, registerNode, unregisterNode } from 'ext:uzumaki/registry.ts';
import type { NodeId } from 'ext:uzumaki/types.ts';
import type { Window } from 'ext:uzumaki/window.ts';

export const NodeType = {
  Root: 1,
  Element: 2,
  Text: 3,
} as const;

export class UzNode {
  protected readonly _native: CoreNode;
  readonly window: Window;

  constructor(window: Window, native: CoreNode) {
    this.window = window;
    this._native = native;
    registerNode(this);
  }

  static fromNodeId(window: Window, nodeId: NodeId | null): UzNode | null {
    if (nodeId == null) return null;
    return getNode(window, nodeId) ?? null;
  }

  get nodeId(): NodeId {
    return this._native.id;
  }

  get windowId(): number {
    return this._native.windowId;
  }

  get nodeType(): number {
    // NodeData::Root => 1,
    // NodeData::Element(_) => 2,
    // NodeData::Text(_) => 3,
    return this._native.nodeType;
  }

  // get nodeName(): string {
  //   return this._native.nodeName;
  // }

  get parentNode(): UzNode | null {
    return UzNode.fromNodeId(this.window, this._native.parentNodeId);
  }

  get firstChild(): UzNode | null {
    return UzNode.fromNodeId(this.window, this._native.firstChildId);
  }

  get lastChild(): UzNode | null {
    return UzNode.fromNodeId(this.window, this._native.lastChildId);
  }

  get nextSibling(): UzNode | null {
    return UzNode.fromNodeId(this.window, this._native.nextSiblingId);
  }

  get previousSibling(): UzNode | null {
    return UzNode.fromNodeId(this.window, this._native.previousSiblingId);
  }

  get textContent(): string | null {
    return this._native.textContent;
  }

  set textContent(text: string | null) {
    // Fixme this should behave different for elements vs text nodes
    // text nodes should set node.textContent, for elements - clear the children and append the new text node
    this._native.textContent = text ?? '';
  }

  appendChild<T extends UzNode>(child: T): T {
    if (!this.window.isDisposed) {
      this._native.appendChild(child._native);
    }
    return child;
  }

  insertBefore<T extends UzNode>(child: T, before: UzNode | null): T {
    if (!this.window.isDisposed) {
      this._native.insertBefore(child._native, before?._native ?? null);
    }
    return child;
  }

  removeChild<T extends UzNode>(child: T): T {
    if (!this.window.isDisposed) {
      this._native.removeChild(child._native);
    }
    return child;
  }

  /**
   * Detach this node from its parent
   */
  remove(): void {
    if (!this.window.isDisposed) {
      this._native.remove();
    }
  }

  removeChildren(): void {
    if (!this.window.isDisposed) {
      this._native.removeChildren();
    }
  }

  destroy(): void {
    this.remove();
    let child = this.firstChild;
    while (child) {
      const next = child.nextSibling;
      child.destroy();
      child = next;
    }
    unregisterNode(this.window, this.nodeId);
  }
}

export class UzTextNode extends UzNode {
  constructor(window: Window, text: string) {
    super(window, core.createTextNode(window.id, text));
  }
}
