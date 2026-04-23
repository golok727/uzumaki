import { isValidElement as isReactElement } from 'react';
import ReactReconciler, { type EventPriority } from 'react-reconciler';
import { DefaultEventPriority } from 'react-reconciler/constants.js'; // fixme our runtime doesnt do probing for imports

import type { JSX } from './jsx/runtime';

import core, { PropKey } from '../core';
import { eventManager } from '../events';
import type { NodeId } from '../types';
import { Window } from '../window';

const PROP_NAME_TO_KEY: Record<string, number> = {
  w: PropKey.W,
  h: PropKey.H,
  minW: PropKey.MinW,
  minH: PropKey.MinH,
  p: PropKey.P,
  px: PropKey.Px,
  py: PropKey.Py,
  pt: PropKey.Pt,
  pb: PropKey.Pb,
  pl: PropKey.Pl,
  pr: PropKey.Pr,
  m: PropKey.M,
  mx: PropKey.Mx,
  my: PropKey.My,
  mt: PropKey.Mt,
  mb: PropKey.Mb,
  ml: PropKey.Ml,
  mr: PropKey.Mr,
  flex: PropKey.Flex,
  flexDir: PropKey.FlexDir,
  flexGrow: PropKey.FlexGrow,
  flexShrink: PropKey.FlexShrink,
  items: PropKey.Items,
  justify: PropKey.Justify,
  gap: PropKey.Gap,
  bg: PropKey.Bg,
  color: PropKey.Color,
  fontSize: PropKey.FontSize,
  fontWeight: PropKey.FontWeight,
  rounded: PropKey.Rounded,
  roundedTL: PropKey.RoundedTL,
  roundedTR: PropKey.RoundedTR,
  roundedBR: PropKey.RoundedBR,
  roundedBL: PropKey.RoundedBL,
  border: PropKey.Border,
  borderTop: PropKey.BorderTop,
  borderRight: PropKey.BorderRight,
  borderBottom: PropKey.BorderBottom,
  borderLeft: PropKey.BorderLeft,
  borderColor: PropKey.BorderColor,
  opacity: PropKey.Opacity,
  display: PropKey.Display,
  cursor: PropKey.Cursor,
  'hover:bg': PropKey.HoverBg,
  'hover:color': PropKey.HoverColor,
  'hover:opacity': PropKey.HoverOpacity,
  'hover:borderColor': PropKey.HoverBorderColor,
  'active:bg': PropKey.ActiveBg,
  'active:color': PropKey.ActiveColor,
  'active:opacity': PropKey.ActiveOpacity,
  'active:borderColor': PropKey.ActiveBorderColor,
  scrollable: PropKey.Scrollable,
  selectable: PropKey.TextSelect,
  visibility: PropKey.Visibility,
  overflowWrap: PropKey.OverflowWrap,
  wordBreak: PropKey.WordBreak,
};

const INTRINSIC_ELEMENTS = new Set([
  'view',
  'text',
  'input',
  'checkbox',
  'button',
  /* 'canvas' */ // todo
]);

const LENGTH_KEYS = new Set([PropKey.W, PropKey.H, PropKey.MinW, PropKey.MinH]);
const COLOR_KEYS = new Set([
  PropKey.Bg,
  PropKey.Color,
  PropKey.BorderColor,
  PropKey.HoverBg,
  PropKey.HoverColor,
  PropKey.HoverBorderColor,
  PropKey.ActiveBg,
  PropKey.ActiveColor,
  PropKey.ActiveBorderColor,
]);
const ENUM_KEYS = new Set([
  PropKey.FlexDir,
  PropKey.Items,
  PropKey.Justify,
  PropKey.Display,
  PropKey.OverflowWrap,
  PropKey.WordBreak,
]);
const STRING_KEYS = new Set([PropKey.Cursor]);

function toLength(value: any): { value: number; unit: number } {
  if (typeof value === 'number') return { value, unit: 0 };
  const s = String(value);
  if (s === 'auto') return { value: 0, unit: 3 };
  if (s === 'full') return { value: 1, unit: 1 };
  if (s.endsWith('rem')) return { value: Number.parseFloat(s), unit: 2 };
  if (s.endsWith('%')) return { value: Number.parseFloat(s) / 100, unit: 1 };
  return { value: Number.parseFloat(s) || 0, unit: 0 };
}

function toColor(value: any): { r: number; g: number; b: number; a: number } {
  if (typeof value === 'string') {
    if (value.startsWith('#')) {
      const hex = value.slice(1);
      if (hex.length === 6) {
        return {
          r: Number.parseInt(hex.slice(0, 2), 16),
          g: Number.parseInt(hex.slice(2, 4), 16),
          b: Number.parseInt(hex.slice(4, 6), 16),
          a: 255,
        };
      }
      if (hex.length === 8) {
        return {
          r: Number.parseInt(hex.slice(0, 2), 16),
          g: Number.parseInt(hex.slice(2, 4), 16),
          b: Number.parseInt(hex.slice(4, 6), 16),
          a: Number.parseInt(hex.slice(6, 8), 16),
        };
      }
    }
    if (value === 'transparent') return { r: 0, g: 0, b: 0, a: 0 };
  }
  return { r: 255, g: 255, b: 255, a: 255 };
}

const FLEX_DIR_MAP: Record<string, number> = {
  row: 0,
  col: 1,
  column: 1,
  'row-reverse': 2,
  'col-reverse': 3,
  'column-reverse': 3,
};

function toEnumValue(key: number, value: any): number {
  if (typeof value === 'number') return value;
  const s = String(value);
  switch (key) {
    case PropKey.FlexDir: {
      return FLEX_DIR_MAP[s] ?? 0;
    }
    case PropKey.Items: {
      return (
        (
          {
            'flex-start': 0,
            start: 0,
            'flex-end': 1,
            end: 1,
            center: 2,
            stretch: 3,
            baseline: 4,
          } as any
        )[s] ?? 3
      );
    }
    case PropKey.Justify: {
      return (
        (
          {
            'flex-start': 0,
            start: 0,
            'flex-end': 1,
            end: 1,
            center: 2,
            'space-between': 3,
            between: 3,
            'space-around': 4,
            around: 4,
            'space-evenly': 5,
            evenly: 5,
          } as any
        )[s] ?? 0
      );
    }
    case PropKey.Display: {
      return ({ none: 0, flex: 1, block: 2 } as any)[s] ?? 1;
    }
    case PropKey.OverflowWrap: {
      return ({ normal: 0, anywhere: 1, 'break-word': 2 } as any)[s] ?? 0;
    }
    case PropKey.WordBreak: {
      return ({ normal: 0, 'break-all': 1, 'keep-all': 2 } as any)[s] ?? 0;
    }
    default: {
      return 0;
    }
  }
}

function setNativeProp(
  windowId: number,
  nodeId: any,
  propName: string,
  value: any,
): void {
  if (propName === 'flex' && typeof value === 'string') {
    const dir = FLEX_DIR_MAP[value];
    if (dir !== undefined) {
      core.setEnumProp(windowId, nodeId, PropKey.Display, 1);
      core.setEnumProp(windowId, nodeId, PropKey.FlexDir, dir);
      return;
    }
  }

  const key = PROP_NAME_TO_KEY[propName];
  if (key === undefined) return;

  if (LENGTH_KEYS.has(key)) {
    const l = toLength(value);
    core.setLengthProp(windowId, nodeId, key, l.value, l.unit);
  } else if (COLOR_KEYS.has(key)) {
    const c = toColor(value);
    core.setColorProp(windowId, nodeId, key, c.r, c.g, c.b, c.a);
  } else if (ENUM_KEYS.has(key)) {
    core.setEnumProp(windowId, nodeId, key, toEnumValue(key, value));
  } else if (STRING_KEYS.has(key)) {
    core.setStringProp(windowId, nodeId, key, String(value));
  } else if (key === PropKey.Visibility) {
    if (typeof value === 'string') {
      core.setF32Prop(windowId, nodeId, key, value === 'visible' ? 1 : 0);
    } else {
      core.setF32Prop(windowId, nodeId, key, value ? 1 : 0);
    }
  } else {
    let numValue: number;
    if (typeof value === 'boolean') {
      numValue = value ? 1 : 0;
    } else if (typeof value === 'number') {
      numValue = value;
    } else {
      numValue = Number.parseFloat(String(value)) || 0;
    }
    core.setF32Prop(windowId, nodeId, key, numValue);
  }
}

function clearNativeProp(
  windowId: number,
  nodeId: any,
  propName: string,
): void {
  const key = PROP_NAME_TO_KEY[propName];
  if (key === undefined) return;

  if (LENGTH_KEYS.has(key)) {
    core.setLengthProp(windowId, nodeId, key, 0, 3);
  } else if (COLOR_KEYS.has(key)) {
    core.setColorProp(windowId, nodeId, key, 255, 255, 255, 255);
  } else if (ENUM_KEYS.has(key)) {
    core.setEnumProp(windowId, nodeId, key, 0);
  } else if (STRING_KEYS.has(key)) {
    core.setStringProp(windowId, nodeId, key, '');
  } else {
    core.setF32Prop(windowId, nodeId, key, 0);
  }
}

function isEventProp(key: string): boolean {
  const thirdCharacterCode = key.codePointAt(2);
  return (
    key.length >= 3 &&
    key[0] === 'o' &&
    key[1] === 'n' &&
    thirdCharacterCode !== undefined &&
    thirdCharacterCode >= 65 &&
    thirdCharacterCode <= 90
  );
}

interface ListenerEntry {
  name: string;
  handler: Function;
  capture: boolean;
}

function listenerKey(name: string, capture: boolean): string {
  return `${name}:${capture ? 'capture' : 'bubble'}`;
}

function parseEventProp(key: string): { name: string; capture: boolean } {
  const raw = key.slice(2); // strip "on"
  if (raw.endsWith('Capture')) {
    return { name: raw.slice(0, -7).toLowerCase(), capture: true };
  }
  return { name: raw.toLowerCase(), capture: false };
}

abstract class BaseElement<
  TProps extends Record<string, any> = Record<string, any>,
> {
  readonly id: any;
  readonly type: string;
  readonly window: Window;
  readonly windowId: number;
  styles: Record<string, any> = {};
  /** Keyed by stable event identity (name + phase). */
  eventListeners: Map<string, ListenerEntry> = new Map();
  children: BaseElement[] = [];
  parent: BaseElement | null = null;

  constructor(id: any, type: string, window: Window) {
    this.id = id;
    this.type = type;
    this.window = window;
    this.windowId = window.id;
  }

  abstract commitUpdate(newProps: TProps, oldProps: TProps): void;

  applyStyles(): void {
    for (const [key, val] of Object.entries(this.styles)) {
      setNativeProp(this.windowId, this.id, key, val);
    }
  }

  applyEvents(): void {
    if (this.eventListeners.size > 0) {
      core.setF32Prop(this.windowId, this.id, PropKey.Interactive, 1);
      for (const entry of this.eventListeners.values()) {
        eventManager.addHandlerByName(
          this.id,
          entry.name,
          entry.handler,
          entry.capture,
        );
      }
    }
  }

  updateStyles(newStyles: Record<string, any>): void {
    for (const [key, val] of Object.entries(newStyles)) {
      if (this.styles[key] !== val) {
        setNativeProp(this.windowId, this.id, key, val);
      }
    }
    for (const key of Object.keys(this.styles)) {
      if (!(key in newStyles)) {
        clearNativeProp(this.windowId, this.id, key);
      }
    }
    this.styles = newStyles;
  }

  updateEvents(newListeners: Map<string, ListenerEntry>): void {
    for (const [key, newEntry] of newListeners) {
      const old = this.eventListeners.get(key);
      if (
        !old ||
        old.handler !== newEntry.handler ||
        old.capture !== newEntry.capture
      ) {
        if (old)
          eventManager.removeHandlerByName(
            this.id,
            old.name,
            old.handler,
            old.capture,
          );
        eventManager.addHandlerByName(
          this.id,
          newEntry.name,
          newEntry.handler,
          newEntry.capture,
        );
      }
    }
    for (const [key, old] of this.eventListeners) {
      if (!newListeners.has(key)) {
        eventManager.removeHandlerByName(
          this.id,
          old.name,
          old.handler,
          old.capture,
        );
      }
    }

    if (newListeners.size > 0 && this.eventListeners.size === 0) {
      core.setF32Prop(this.windowId, this.id, PropKey.Interactive, 1);
    } else if (newListeners.size === 0 && this.eventListeners.size > 0) {
      core.setF32Prop(this.windowId, this.id, PropKey.Interactive, 0);
    }
    this.eventListeners = newListeners;
  }

  destroy(): void {
    for (const child of this.children) {
      child.destroy();
    }
    eventManager.clearNode(this.id);
    this.eventListeners.clear();
    this.children = [];
  }
}

class ViewElement extends BaseElement<Record<string, any>> {
  constructor(window: Window, type: string, props: Record<string, any>) {
    const id = core.createElement(window.id, type);
    super(id, type, window);
    this.parseProps(props);
    this.applyStyles();
    this.applyEvents();
  }

  private parseProps(props: Record<string, any>): void {
    for (const key in props) {
      if (key === 'children' || key === 'key' || key === 'ref') continue;
      const value = props[key];
      if (value == null) continue;
      if (isEventProp(key)) {
        const { name, capture } = parseEventProp(key);
        this.eventListeners.set(listenerKey(name, capture), {
          name,
          handler: value,
          capture,
        });
      } else if (PROP_NAME_TO_KEY[key] !== undefined) {
        this.styles[key] = value;
      }
    }
  }

  commitUpdate(
    newProps: Record<string, any>,
    _oldProps: Record<string, any>,
  ): void {
    const newStyles: Record<string, any> = {};
    const newEvents: Map<string, ListenerEntry> = new Map();

    for (const key in newProps) {
      if (key === 'children' || key === 'key' || key === 'ref') continue;
      const value = newProps[key];
      if (value == null) continue;
      if (isEventProp(key)) {
        const { name, capture } = parseEventProp(key);
        newEvents.set(listenerKey(name, capture), {
          name,
          handler: value,
          capture,
        });
      } else if (PROP_NAME_TO_KEY[key] !== undefined) {
        newStyles[key] = value;
      }
    }

    this.updateStyles(newStyles);
    this.updateEvents(newEvents);
  }
}

import { __DEV__ } from '../constants';

const INPUT_ATTR_NAMES = new Set([
  'value',
  'placeholder',
  'disabled',
  'maxLength',
  'multiline',
  'secure',
]);
const CHECKBOX_ATTR_NAMES = new Set(['checked']);

class InputElement extends BaseElement<Record<string, any>> {
  inputAttrs: Record<string, any> = {};
  private onChangeText: ((value: string) => void) | undefined;
  private onChangeTextListener: ((ev: any) => void) | null = null;

  constructor(window: Window, props: Record<string, any>) {
    const id = core.createElement(window.id, 'input');
    super(id, 'input', window);
    this.parseProps(props);
    this.applyStyles();
    this.applyInputAttrs();
    this.applyEvents();
    this.bindOnChangeText(props.onChangeText);
  }

  private parseProps(props: Record<string, any>): void {
    for (const key in props) {
      if (
        key === 'children' ||
        key === 'key' ||
        key === 'ref' ||
        key === 'onChangeText'
      )
        continue;
      const value = props[key];
      if (value == null) continue;
      if (isEventProp(key)) {
        const { name, capture } = parseEventProp(key);
        this.eventListeners.set(listenerKey(name, capture), {
          name,
          handler: value,
          capture,
        });
      } else if (INPUT_ATTR_NAMES.has(key)) {
        this.inputAttrs[key] = value;
      } else if (PROP_NAME_TO_KEY[key] !== undefined) {
        this.styles[key] = value;
      }
    }
  }

  private applyInputAttrs(): void {
    for (const [key, val] of Object.entries(this.inputAttrs)) {
      InputElement.setInputAttr(this.windowId, this.id, key, val);
    }
  }

  private bindOnChangeText(
    onChangeText: ((value: string) => void) | undefined,
  ): void {
    if (!onChangeText) return;
    this.onChangeText = onChangeText;
    this.onChangeTextListener = (ev: any) => {
      this.onChangeText?.(ev.value);
    };
    eventManager.addHandlerByName(this.id, 'input', this.onChangeTextListener);
    core.setF32Prop(this.windowId, this.id, PropKey.Interactive, 1);
  }

  private unbindOnChangeText(): void {
    if (this.onChangeTextListener) {
      eventManager.removeHandlerByName(
        this.id,
        'input',
        this.onChangeTextListener,
      );
      this.onChangeTextListener = null;
    }
    this.onChangeText = undefined;
  }

  commitUpdate(
    newProps: Record<string, any>,
    _oldProps: Record<string, any>,
  ): void {
    const newStyles: Record<string, any> = {};
    const newInputAttrs: Record<string, any> = {};
    const newEvents: Map<string, ListenerEntry> = new Map();

    for (const key in newProps) {
      if (
        key === 'children' ||
        key === 'key' ||
        key === 'ref' ||
        key === 'onChangeText'
      )
        continue;
      const value = newProps[key];
      if (value == null) continue;
      if (isEventProp(key)) {
        const { name, capture } = parseEventProp(key);
        newEvents.set(listenerKey(name, capture), {
          name,
          handler: value,
          capture,
        });
      } else if (INPUT_ATTR_NAMES.has(key)) {
        newInputAttrs[key] = value;
      } else if (PROP_NAME_TO_KEY[key] !== undefined) {
        newStyles[key] = value;
      }
    }

    this.updateStyles(newStyles);
    this.updateEvents(newEvents);

    const newOnChangeText = newProps.onChangeText;
    if (newOnChangeText !== this.onChangeText) {
      this.unbindOnChangeText();
      this.bindOnChangeText(newOnChangeText);
    }

    for (const [key, val] of Object.entries(newInputAttrs)) {
      if (this.inputAttrs[key] !== val) {
        InputElement.setInputAttr(this.windowId, this.id, key, val);
      }
    }
    this.inputAttrs = newInputAttrs;
  }

  override destroy(): void {
    this.unbindOnChangeText();
    super.destroy();
  }

  static setInputAttr(
    windowId: number,
    nodeId: any,
    key: string,
    value: any,
  ): void {
    switch (key) {
      case 'value': {
        core.setInputValue(windowId, nodeId, String(value ?? ''));
        break;
      }
      case 'placeholder': {
        core.setInputPlaceholder(windowId, nodeId, String(value ?? ''));
        break;
      }
      case 'disabled': {
        core.setInputDisabled(windowId, nodeId, !!value);
        break;
      }
      case 'maxLength': {
        core.setInputMaxLength(
          windowId,
          nodeId,
          typeof value === 'number' ? value : -1,
        );
        break;
      }
      case 'multiline': {
        core.setInputMultiline(windowId, nodeId, !!value);
        break;
      }
      case 'secure': {
        core.setInputSecure(windowId, nodeId, !!value);
        break;
      }
    }
  }
}

class CheckboxElement extends BaseElement<Record<string, any>> {
  checkboxAttrs: Record<string, any> = {};
  private onChange: ((checked: boolean) => void) | undefined;
  private onChangeListener: ((ev: any) => void) | null = null;

  constructor(window: Window, props: Record<string, any>) {
    const id = core.createElement(window.id, 'checkbox');
    super(id, 'checkbox', window);
    this.parseProps(props);
    this.applyStyles();
    this.applyCheckboxAttrs();
    this.applyEvents();
    this.bindOnChange(props.onChange);
  }

  private parseProps(props: Record<string, any>): void {
    for (const key in props) {
      if (
        key === 'children' ||
        key === 'key' ||
        key === 'ref' ||
        key === 'onChange'
      )
        continue;
      const value = props[key];
      if (value == null) continue;
      if (isEventProp(key)) {
        const { name, capture } = parseEventProp(key);
        this.eventListeners.set(listenerKey(name, capture), {
          name,
          handler: value,
          capture,
        });
      } else if (CHECKBOX_ATTR_NAMES.has(key)) {
        this.checkboxAttrs[key] = value;
      } else if (PROP_NAME_TO_KEY[key] !== undefined) {
        this.styles[key] = value;
      }
    }
  }

  private applyCheckboxAttrs(): void {
    for (const [key, val] of Object.entries(this.checkboxAttrs)) {
      CheckboxElement.setCheckboxAttr(this.windowId, this.id, key, val);
    }
  }

  private bindOnChange(
    onChange: ((checked: boolean) => void) | undefined,
  ): void {
    if (!onChange) return;
    this.onChange = onChange;
    this.onChangeListener = (ev: any) => {
      this.onChange?.(ev.value === 'true');
    };
    eventManager.addHandlerByName(this.id, 'input', this.onChangeListener);
    core.setF32Prop(this.windowId, this.id, PropKey.Interactive, 1);
  }

  private unbindOnChange(): void {
    if (this.onChangeListener) {
      eventManager.removeHandlerByName(this.id, 'input', this.onChangeListener);
      this.onChangeListener = null;
    }
    this.onChange = undefined;
  }

  commitUpdate(
    newProps: Record<string, any>,
    _oldProps: Record<string, any>,
  ): void {
    const newStyles: Record<string, any> = {};
    const newCheckboxAttrs: Record<string, any> = {};
    const newEvents: Map<string, ListenerEntry> = new Map();

    for (const key in newProps) {
      if (
        key === 'children' ||
        key === 'key' ||
        key === 'ref' ||
        key === 'onChange'
      )
        continue;
      const value = newProps[key];
      if (value == null) continue;
      if (isEventProp(key)) {
        const { name, capture } = parseEventProp(key);
        newEvents.set(listenerKey(name, capture), {
          name,
          handler: value,
          capture,
        });
      } else if (CHECKBOX_ATTR_NAMES.has(key)) {
        newCheckboxAttrs[key] = value;
      } else if (PROP_NAME_TO_KEY[key] !== undefined) {
        newStyles[key] = value;
      }
    }

    this.updateStyles(newStyles);
    this.updateEvents(newEvents);

    const newOnChange = newProps.onChange;
    if (newOnChange !== this.onChange) {
      this.unbindOnChange();
      this.bindOnChange(newOnChange);
    }

    for (const [key, val] of Object.entries(newCheckboxAttrs)) {
      if (this.checkboxAttrs[key] !== val) {
        CheckboxElement.setCheckboxAttr(this.windowId, this.id, key, val);
      }
    }
    this.checkboxAttrs = newCheckboxAttrs;
  }

  override destroy(): void {
    this.unbindOnChange();
    super.destroy();
  }

  static setCheckboxAttr(
    windowId: number,
    nodeId: any,
    key: string,
    value: any,
  ): void {
    if (key === 'checked') {
      core.setCheckboxChecked(windowId, nodeId, !!value);
    }
  }
}

class TextElement extends BaseElement<Record<string, any>> {
  textContent: string;

  constructor(
    window: Window,
    type: string,
    text: string,
    props: Record<string, any>,
  ) {
    const id = core.createTextNode(window.id, text);
    super(id, type, window);
    this.textContent = text;
    this.parseProps(props);
    this.applyStyles();
    this.applyEvents();
  }

  private parseProps(props: Record<string, any>): void {
    for (const key in props) {
      if (key === 'children' || key === 'key' || key === 'ref') continue;
      const value = props[key];
      if (value == null) continue;
      if (isEventProp(key)) {
        const { name, capture } = parseEventProp(key);
        this.eventListeners.set(listenerKey(name, capture), {
          name,
          handler: value,
          capture,
        });
      } else if (PROP_NAME_TO_KEY[key] !== undefined) {
        this.styles[key] = value;
      }
    }
  }

  setText(text: string): void {
    if (this.textContent !== text) {
      this.textContent = text;
      core.setText(this.windowId, this.id, text);
    }
  }

  commitUpdate(
    newProps: Record<string, any>,
    _oldProps: Record<string, any>,
  ): void {
    const newStyles: Record<string, any> = {};
    const newEvents: Map<string, ListenerEntry> = new Map();

    for (const key in newProps) {
      if (key === 'children' || key === 'key' || key === 'ref') continue;
      const value = newProps[key];
      if (value == null) continue;
      if (isEventProp(key)) {
        const { name, capture } = parseEventProp(key);
        newEvents.set(listenerKey(name, capture), {
          name,
          handler: value,
          capture,
        });
      } else if (PROP_NAME_TO_KEY[key] !== undefined) {
        newStyles[key] = value;
      }
    }

    this.updateStyles(newStyles);
    this.updateEvents(newEvents);

    const newText = getTextContent(newProps.children);
    this.setText(newText);
  }
}

type Container = {
  window: Window;
  rootNodeId: NodeId;
};

function getWindowId(container: Container): number {
  return container.window.id;
}

/**
 * Get text content of a <text> node. will throw an error if you nest a react element inside this
 */
function getTextContent(children: any): string {
  if (children == null) return '';
  if (Array.isArray(children)) {
    return children
      .map((child) => {
        if (__DEV__ && isReactElement(child)) {
          throw new Error(
            `[uzumaki] <text> received a React element as a child (<${child.type}>). ` +
              `Only strings and numbers are allowed inside <text>.`,
          );
        }
        return child == null ? '' : String(child);
      })
      .join('');
  }

  if (__DEV__ && isReactElement(children)) {
    throw new Error(
      `[uzumaki] <text> received a React element as a child (<${children.type}>). ` +
        `Only strings and numbers are allowed inside <text>.`,
    );
  }

  return String(children);
}

function isTextType(type: string): boolean {
  return type === 'text';
}

function createElementInstance(
  type: string,
  props: Record<string, any>,
  window: Window,
): BaseElement {
  if (!INTRINSIC_ELEMENTS.has(type)) {
    throw new Error(
      `[uzumaki] Unknown intrinsic element: <${type}>. Did you mean <view>?`,
    );
  }

  if (type === 'input') {
    return new InputElement(window, props);
  }

  if (type === 'checkbox') {
    return new CheckboxElement(window, props);
  }

  if (isTextType(type)) {
    return new TextElement(window, type, getTextContent(props.children), props);
  }
  return new ViewElement(window, type, props);
}

type Type = string;
type Props = Record<string, any>;
type Instance = BaseElement;
type TextInstance = TextElement;
type SuspenseInstance = any;
type HydratableInstance = any;
type FormInstance = any;
type PublicInstance = BaseElement;
type HostContext = {};
type ChildSet = any;
type TimeoutHandle = ReturnType<typeof setTimeout>;
type NoTimeout = undefined;
type TransitionStatus = any;

let currentPriority: EventPriority = DefaultEventPriority;
const reconciler = ReactReconciler<
  Type,
  Props,
  Container,
  Instance,
  TextInstance,
  SuspenseInstance,
  HydratableInstance,
  FormInstance,
  PublicInstance,
  HostContext,
  ChildSet,
  TimeoutHandle,
  NoTimeout,
  TransitionStatus
>({
  supportsMutation: true,
  supportsPersistence: false,

  createInstance(type, props, rootContainer) {
    return createElementInstance(type, props, rootContainer.window);
  },

  createTextInstance(text, rootContainer) {
    return new TextElement(rootContainer.window, '#text', text, {});
  },

  shouldSetTextContent(type) {
    return isTextType(type);
  },

  appendInitialChild(parent, child) {
    parent.children.push(child);
    child.parent = parent;
    if (parent.window.isDisposed) return;
    core.appendChild(parent.windowId, parent.id, child.id);
  },

  finalizeInitialChildren() {
    return false;
  },

  appendChildToContainer(container, child) {
    child.parent = null;
    if (container.window.isDisposed) return;
    const windowId = getWindowId(container);
    core.appendChild(windowId, container.rootNodeId, child.id);
  },

  appendChild(parent, child) {
    parent.children.push(child);
    child.parent = parent;
    if (parent.window.isDisposed) return;
    core.appendChild(parent.windowId, parent.id, child.id);
  },

  insertBefore(parent, child, before) {
    const idx = parent.children.indexOf(before);
    if (idx === -1) {
      parent.children.push(child);
    } else {
      parent.children.splice(idx, 0, child);
    }
    child.parent = parent;
    if (parent.window.isDisposed) return;
    core.insertBefore(parent.windowId, parent.id, child.id, before.id);
  },

  insertInContainerBefore(container, child, before) {
    child.parent = null;
    if (container.window.isDisposed) return;
    const windowId = getWindowId(container);
    core.insertBefore(windowId, container.rootNodeId, child.id, before.id);
  },

  removeChild(parent, child) {
    const idx = parent.children.indexOf(child);
    if (idx !== -1) parent.children.splice(idx, 1);
    child.parent = null;
    if (!parent.window.isDisposed) {
      core.removeChild(parent.windowId, parent.id, child.id);
    }
    child.destroy();
  },

  removeChildFromContainer(container, child) {
    child.parent = null;
    if (!container.window.isDisposed) {
      const windowId = getWindowId(container);
      core.removeChild(windowId, container.rootNodeId, child.id);
    }
    child.destroy();
  },

  commitUpdate(instance, _type, oldProps, newProps, _internalHandle) {
    if (instance.window.isDisposed) return;
    instance.commitUpdate(newProps, oldProps);
  },

  commitTextUpdate(instance, _oldText, newText) {
    if (instance.window.isDisposed) return;
    instance.setText(newText);
  },

  detachDeletedInstance(instance) {
    instance.destroy();
  },

  hideInstance(instance) {
    core.setF32Prop(instance.windowId, instance.id, PropKey.Visibility, 0);
  },

  unhideInstance(instance) {
    core.setF32Prop(instance.windowId, instance.id, PropKey.Visibility, 1);
  },

  hideTextInstance(instance) {
    core.setF32Prop(instance.windowId, instance.id, PropKey.Visibility, 0);
  },

  unhideTextInstance(instance) {
    core.setF32Prop(instance.windowId, instance.id, PropKey.Visibility, 1);
  },

  resetTextContent(instance) {
    core.setText(instance.windowId, instance.id, '');
  },

  clearContainer(container) {
    const windowId = getWindowId(container);
    core.resetDom(windowId);
  },

  getRootHostContext: () => ({}),
  getChildHostContext: (parentHostContext) => parentHostContext,
  getPublicInstance: (instance) => instance,

  prepareForCommit(_container) {
    return null;
  },

  resetAfterCommit(container) {
    core.requestRedraw(container.window.id);
  },

  preparePortalMount: () => {},
  scheduleTimeout: (fn, delay) => setTimeout(fn, delay),
  cancelTimeout: (id) => clearTimeout(id),
  noTimeout: undefined,
  isPrimaryRenderer: true,
  getInstanceFromNode: () => null,
  beforeActiveInstanceBlur: () => {},
  afterActiveInstanceBlur: () => {},
  prepareScopeUpdate: () => {},
  getInstanceFromScope: () => null,
  supportsHydration: false,
  NotPendingTransition: undefined,
  HostTransitionContext: {
    $$typeof: Symbol.for('react.context'),
    _currentValue: null,
    _currentValue2: null,
  } as any,
  setCurrentUpdatePriority: (newPriority) => {
    currentPriority = newPriority;
  },
  getCurrentUpdatePriority: () => currentPriority,
  resolveUpdatePriority: () => DefaultEventPriority,
  resetFormInstance: () => {},
  requestPostPaintCallback: () => {},
  shouldAttemptEagerTransition: () => false,
  trackSchedulerEvent: () => {},
  resolveEventType: () => null,
  resolveEventTimeStamp: () => Date.now(),
  maySuspendCommit: () => false,
  preloadInstance: () => false,
  startSuspendingCommit: () => false,
  suspendInstance: () => {},
  waitForCommitToBeReady: () => null,
});

const roots = new Map<string, { root: any; container: Container }>();

export function render(window: Window, element: JSX.Element) {
  const rootNodeId = core.getRootNodeId(window.id);
  const container: Container = { window, rootNodeId };

  const root = reconciler.createContainer(
    container,
    1,
    null,
    false,
    null,
    '',
    console.error,
    console.error,
    console.error,
    () => {},
  );

  roots.set(window.label, { root, container });
  reconciler.updateContainer(element, root, null, null);

  function dispose() {
    reconciler.updateContainer(null, root, null, null);
    roots.delete(window.label);
  }

  window.addDisposable(dispose);

  return {
    dispose,
  };
}

export function disposeRoot(windowLabel: string) {
  const entry = roots.get(windowLabel);
  if (entry) {
    reconciler.updateContainer(null, entry.root, null, null);
    roots.delete(windowLabel);
  }
}

export function disposeAllRoots() {
  roots.clear();
}

export function clearEventRegistry() {
  eventManager.clear();
}
