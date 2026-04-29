import type {
  NodeId,
  WindowAttributes,
  WindowLevel,
  WindowPosition,
  WindowSize,
  WindowTheme,
} from './types';

export interface CoreWindow {
  close(): void;
  readonly id: number;
  readonly innerWidth: number | null;
  readonly innerHeight: number | null;
  readonly title: string | null;
  setTitle(title: string): boolean;
  readonly visible: boolean | null;
  setVisible(visible: boolean): boolean;
  readonly transparent: boolean | null;
  setTransparent(transparent: boolean): boolean;
  readonly resizable: boolean | null;
  setResizable(resizable: boolean): boolean;
  readonly decorated: boolean | null;
  setDecorations(decorations: boolean): boolean;
  readonly maximized: boolean | null;
  setMaximized(maximized: boolean): boolean;
  readonly minimized: boolean | null;
  setMinimized(minimized: boolean): boolean;
  readonly fullscreen: boolean | null;
  setFullscreen(fullscreen: boolean): boolean;
  readonly windowLevel: WindowLevel | null;
  setWindowLevel(level: WindowLevel): boolean;
  setMinSize(width: number, height: number): boolean;
  setMaxSize(width: number, height: number): boolean;
  readonly innerSize: WindowSize | null;
  readonly outerSize: WindowSize | null;
  readonly position: WindowPosition | null;
  setPosition(x: number, y: number): boolean;
  readonly scaleFactor: number | null;
  readonly theme: WindowTheme | null;
  setTheme(theme: WindowTheme): boolean;
  readonly active: boolean | null;
  focus(): boolean;
  readonly contentProtected: boolean | null;
  setContentProtected(contentProtected: boolean): boolean;
  readonly closable: boolean | null;
  setClosable(closable: boolean): boolean;
  readonly minimizable: boolean | null;
  setMinimizable(minimizable: boolean): boolean;
  readonly maximizable: boolean | null;
  setMaximizable(maximizable: boolean): boolean;
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
  createWindow(options: WindowAttributes): CoreWindow;
  requestQuit(): void;
  requestRedraw(windowId: number): void;
  getRootNode(windowId: number): CoreNode;
  createCoreElementNode(windowId: number, elementType: string): CoreNode;
  createCoreTextNode(windowId: number, text: string): CoreNode;
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
