import { NodeId } from './types';
export { PropKey } from './generated/prop_keys';

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
  setLengthProp(
    windowId: number,
    nodeId: NodeId,
    prop: number,
    value: number,
    unit: number,
  ): void;
  setColorProp(
    windowId: number,
    nodeId: NodeId,
    prop: number,
    r: number,
    g: number,
    b: number,
    a: number,
  ): void;
  setF32Prop(
    windowId: number,
    nodeId: NodeId,
    prop: number,
    value: number,
  ): void;
  setEnumProp(
    windowId: number,
    nodeId: NodeId,
    prop: number,
    value: number,
  ): void;
  setStringProp(
    windowId: number,
    nodeId: NodeId,
    prop: number,
    value: string,
  ): void;
  setInputValue(windowId: number, nodeId: NodeId, value: string): void;
  getInputValue(windowId: number, nodeId: NodeId): string;
  setInputPlaceholder(
    windowId: number,
    nodeId: NodeId,
    placeholder: string,
  ): void;
  setInputDisabled(windowId: number, nodeId: NodeId, disabled: boolean): void;
  setInputMaxLength(windowId: number, nodeId: NodeId, maxLength: number): void;
  setInputMultiline(windowId: number, nodeId: NodeId, multiline: boolean): void;
  setInputSecure(windowId: number, nodeId: NodeId, secure: boolean): void;
  setCheckboxChecked(windowId: number, nodeId: NodeId, checked: boolean): void;
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
