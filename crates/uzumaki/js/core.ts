import { NodeId } from './types';

export interface WindowPosition {
  x: number;
  y: number;
}

export interface WindowSize {
  width: number;
  height: number;
}

export type WindowTheme = 'light' | 'dark' | 'system';
export type WindowLevel = 'normal' | 'alwaysOnTop' | 'alwaysOnBottom';

export interface NativeWindow {
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
  readonly alwaysOnTop: boolean | null;
  setAlwaysOnTop(alwaysOnTop: boolean): boolean;
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

interface Core {
  createWindow(options: {
    width?: number;
    height?: number;
    title?: string;
    visible?: boolean;
    resizable?: boolean;
    decorations?: boolean;
    transparent?: boolean;
    maximized?: boolean;
    minimized?: boolean;
    fullscreen?: boolean;
    alwaysOnTop?: boolean;
    windowLevel?: WindowLevel;
    minWidth?: number;
    minHeight?: number;
    maxWidth?: number;
    maxHeight?: number;
    position?: WindowPosition;
    theme?: WindowTheme;
    active?: boolean;
    contentProtected?: boolean;
    closable?: boolean;
    minimizable?: boolean;
    maximizable?: boolean;
  }): NativeWindow;
  requestQuit(): void;
  requestRedraw(windowId: number): void;
  getRootNodeId(windowId: number): NodeId;
  createElement(windowId: number, elementType: string): NodeId;
  createTextNode(windowId: number, text: string): NodeId;
  setEncodedImageData(
    windowId: number,
    nodeId: NodeId,
    cacheKey: string,
    data: Uint8Array,
  ): void;
  applyCachedImage(windowId: number, nodeId: NodeId, cacheKey: string): boolean;
  clearImageData(windowId: number, nodeId: NodeId): void;
  appendChild(windowId: number, parentId: NodeId, childId: NodeId): void;
  insertBefore(
    windowId: number,
    parentId: NodeId,
    childId: NodeId,
    beforeId: NodeId,
  ): void;
  removeChild(windowId: number, parentId: NodeId, childId: NodeId): void;
  setText(windowId: number, nodeId: NodeId, text: string): void;
  resetDom(windowId: number): void;
  setStrAttribute(
    windowId: number,
    nodeId: NodeId,
    name: string,
    value: string,
  ): void;
  setNumberAttribute(
    windowId: number,
    nodeId: NodeId,
    name: string,
    value: number,
  ): void;
  setBoolAttribute(
    windowId: number,
    nodeId: NodeId,
    name: string,
    value: boolean,
  ): void;
  clearAttribute(windowId: number, nodeId: NodeId, name: string): void;
  getAttribute(windowId: number, nodeId: NodeId, name: string): unknown;
  focusInput(windowId: number, nodeId: NodeId): void;
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

export function setNativeProp(
  windowId: number,
  nodeId: any,
  propName: string,
  value: any,
): void {
  if (typeof value === 'boolean') {
    core.setBoolAttribute(windowId, nodeId, propName, value);
  } else if (typeof value === 'number') {
    core.setNumberAttribute(windowId, nodeId, propName, value);
  } else {
    core.setStrAttribute(windowId, nodeId, propName, String(value));
  }
}

export function clearNativeProp(
  windowId: number,
  nodeId: any,
  propName: string,
): void {
  core.clearAttribute(windowId, nodeId, propName);
}
