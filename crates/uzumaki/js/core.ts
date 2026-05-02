import { NodeId } from './types';

export interface CoreWindow {
  close(): void;
  readonly id: number;
  readonly innerWidth: number | null;
  readonly innerHeight: number | null;
  readonly title: string | null;
  readonly scaleFactor: number | null;
  remBase: number;
}

export interface CoreNode {
  readonly id: NodeId;
  readonly windowId: number;
  readonly nodeType: number;
  readonly nodeName: string;
  readonly parentNode: CoreNode | null;
  readonly firstChild: CoreNode | null;
  readonly lastChild: CoreNode | null;
  readonly nextSibling: CoreNode | null;
  readonly previousSibling: CoreNode | null;
  textContent: string | null;
  appendChild(child: CoreNode): void;
  insertBefore(child: CoreNode, before: CoreNode | null): void;
  removeChild(child: CoreNode): void;
  setStrAttribute(name: string, value: string): void;
  setNumberAttribute(name: string, value: number): void;
  setBoolAttribute(name: string, value: boolean): void;
  removeAttribute(name: string): void;
  getAttribute(name: string): unknown;
}

interface Core {
  createWindow(options: {
    width: number;
    height: number;
    title: string;
  }): CoreWindow;
  requestQuit(): void;
  requestRedraw(windowId: number): void;
  getRootNode(windowId: number): CoreNode;
  createElementNode(windowId: number, elementType: string): CoreNode;
  createTextNode(windowId: number, text: string): CoreNode;
  setEncodedImageData(
    windowId: number,
    nodeId: NodeId,
    cacheKey: string,
    data: Uint8Array,
  ): void;
  applyCachedImage(windowId: number, nodeId: NodeId, cacheKey: string): boolean;
  clearImageData(windowId: number, nodeId: NodeId): void;
  resetDom(windowId: number): void;
  focusElement(windowId: number, nodeId: NodeId): void;
  getAncestorPath(windowId: number, nodeId: NodeId): NodeId[];
  getSelection(windowId: number): SelectionState | null;
  getSelectedText(windowId: number): string;
  readClipboardText(): string | null;
  writeClipboardText(text: string): boolean;
  decodeImageSource(source: string): Promise<Uint8Array>;
  onAppEvent(
    handler: (
      event: any,
      ctx: { preventDefault(): void; readonly defaultPrevented: boolean },
    ) => void,
  ): () => void;
}

export interface SelectionState {
  /** The textSelect root node that owns this selection. */
  rootNodeId: NodeId;
  /** Flat grapheme offset where selection started (drag origin). */
  anchorOffset: number;
  /** Flat grapheme offset where selection currently ends (cursor). */
  activeOffset: number;
  /** Start offset (min of anchor and active). */
  start: number;
  /** End offset (max of anchor and active). */
  end: number;
  /** Total grapheme count in the selectable run. */
  runLength: number;
  /** Whether the selection is collapsed (anchor == active). */
  isCollapsed: boolean;
  /** The selected text content. */
  text: string;
}

const core: Core = (globalThis as unknown as any)
  .__uzumaki_ops_dont_touch_this__;

export default core;
