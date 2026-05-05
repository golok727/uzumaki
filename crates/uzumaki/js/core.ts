import type {
  NodeId,
  WindowOptions,
  WindowLevel,
  WindowPosition,
  WindowSize,
  WindowTheme,
} from './types';

export interface CoreWindow {
  readonly id: number;

  close(): void;

  readonly innerWidth: number | null;
  readonly innerHeight: number | null;

  title: string | null;
  visible: boolean | null;
  transparent: boolean | null;
  resizable: boolean | null;
  decorations: boolean | null;
  maximized: boolean | null;
  minimized: boolean | null;
  fullscreen: boolean | null;
  windowLevel: WindowLevel | null;

  setMinSize(width: number, height: number): boolean;
  setMaxSize(width: number, height: number): boolean;

  readonly innerSize: WindowSize | null;
  readonly outerSize: WindowSize | null;
  readonly position: WindowPosition | null;

  setPosition(x: number, y: number): boolean;

  readonly scaleFactor: number | null;

  theme: WindowTheme | null;

  readonly active: boolean | null;
  focus(): boolean;

  contentProtected: boolean | null;
  closable: boolean | null;
  minimizable: boolean | null;
  maximizable: boolean | null;

  remBase: number;
}

export interface CoreNode {
  readonly id: NodeId;
  readonly windowId: number;
  readonly nodeType: number;
  readonly nodeName: string;
  readonly parentNodeId: NodeId | null;
  readonly firstChildId: NodeId | null;
  readonly lastChildId: NodeId | null;
  readonly nextSiblingId: NodeId | null;
  readonly previousSiblingId: NodeId | null;
  textContent: string | null;
  appendChild(child: CoreNode): void;
  insertBefore(child: CoreNode, before: CoreNode | null): void;
  removeChild(child: CoreNode): void;
  remove(): void;
  setStrAttribute(name: string, value: string): void;
  setNumberAttribute(name: string, value: number): void;
  setBoolAttribute(name: string, value: boolean): void;
  removeAttribute(name: string): void;
  getAttribute(name: string): unknown;
}

interface Core {
  createWindow(options: WindowOptions): CoreWindow;
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
