import { INPUT_ATTR_NAMES } from '../constants';
import core, { clearNativeProp, setNativeProp } from '../core';
import { eventManager } from '../events';
import { ListenerEntry } from '../types';
import {
  assignNativeStyle,
  isEventProp,
  listenerKey,
  parseEventProp,
} from '../utils';
import { Window } from '../window';
import { BaseElement } from './base';

export class InputElement extends BaseElement<Record<string, any>> {
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
      } else {
        assignNativeStyle(this.styles, key, value);
      }
    }
  }

  private applyInputAttrs(): void {
    for (const [key, val] of Object.entries(this.inputAttrs)) {
      setNativeProp(this.windowId, this.id, key, val);
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
    core.setBoolAttribute(this.windowId, this.id, 'interactive', true);
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
      } else {
        assignNativeStyle(newStyles, key, value);
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
        setNativeProp(this.windowId, this.id, key, val);
      }
    }
    for (const key of Object.keys(this.inputAttrs)) {
      if (!(key in newInputAttrs)) {
        clearNativeProp(this.windowId, this.id, key);
      }
    }
    this.inputAttrs = newInputAttrs;
  }

  override destroy(): void {
    this.unbindOnChangeText();
    super.destroy();
  }
}
