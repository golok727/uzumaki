import { NodeId } from './types';

interface Core {
  createWindow(options: {
    width: number;
    height: number;
    title: string;
  }): number;
  requestClose(): void;
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
  setRemBase(windowId: number, value: number): void;
  getWindowWidth(windowId: number): number | null;
  getWindowHeight(windowId: number): number | null;
  getWindowTitle(windowId: number): string | null;
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
