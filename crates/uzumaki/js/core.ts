import { NodeId } from './types';

export const enum PropKey {
  W = 0,
  H = 1,
  P = 2,
  Px = 3,
  Py = 4,
  Pt = 5,
  Pb = 6,
  Pl = 7,
  Pr = 8,
  M = 9,
  Mx = 10,
  My = 11,
  Mt = 12,
  Mb = 13,
  Ml = 14,
  Mr = 15,
  Flex = 16,
  FlexDir = 17,
  FlexGrow = 18,
  FlexShrink = 19,
  Items = 20,
  Justify = 21,
  Gap = 22,
  Bg = 23,
  Color = 24,
  FontSize = 25,
  FontWeight = 26,
  Rounded = 27,
  RoundedTL = 28,
  RoundedTR = 29,
  RoundedBR = 30,
  RoundedBL = 31,
  Border = 32,
  BorderTop = 33,
  BorderRight = 34,
  BorderBottom = 35,
  BorderLeft = 36,
  BorderColor = 37,
  Opacity = 38,
  Display = 39,
  Cursor = 40,
  Interactive = 41,
  Visible = 42,
  HoverBg = 43,
  HoverColor = 44,
  HoverOpacity = 45,
  HoverBorderColor = 46,
  ActiveBg = 47,
  ActiveColor = 48,
  ActiveOpacity = 49,
  ActiveBorderColor = 50,
  Scrollable = 51,
  MinW = 52,
  MinH = 53,
  TextSelect = 54,
}

interface Core {
  createWindow(options: {
    width: number;
    height: number;
    title: string;
  }): number;
  requestClose(): void;
  requestRedraw(windowId: number): void;
  getRootNodeId(windowId: number): any;
  createElement(windowId: number, elementType: string): any;
  createTextNode(windowId: number, text: string): any;
  appendChild(windowId: number, parentId: any, childId: any): void;
  insertBefore(
    windowId: number,
    parentId: any,
    childId: any,
    beforeId: any,
  ): void;
  removeChild(windowId: number, parentId: any, childId: any): void;
  setText(windowId: number, nodeId: any, text: string): void;
  resetDom(windowId: number): void;
  setLengthProp(
    windowId: number,
    nodeId: any,
    prop: number,
    value: number,
    unit: number,
  ): void;
  setColorProp(
    windowId: number,
    nodeId: any,
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
  focusInput(windowId: number, nodeId: NodeId): void;
  setRemBase(windowId: number, value: number): void;
  getWindowWidth(windowId: number): number | null;
  getWindowHeight(windowId: number): number | null;
  getWindowTitle(windowId: number): string | null;
  getAncestorPath(windowId: number, nodeId: NodeId): any[]; // returns NodeId[]
  getSelection(windowId: number): SelectionState | null;
  getSelectedText(windowId: number): string;
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
