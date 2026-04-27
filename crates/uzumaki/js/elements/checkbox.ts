import { CHECKBOX_ATTR_NAMES } from '../constants';
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

export class CheckboxElement extends BaseElement<Record<string, any>> {
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
      } else {
        assignNativeStyle(this.styles, key, value);
      }
    }
  }

  private applyCheckboxAttrs(): void {
    for (const [key, val] of Object.entries(this.checkboxAttrs)) {
      setNativeProp(this.windowId, this.id, key, val);
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
    core.setBoolAttribute(this.windowId, this.id, 'interactive', true);
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
      } else {
        assignNativeStyle(newStyles, key, value);
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
        setNativeProp(this.windowId, this.id, key, val);
      }
    }
    for (const key of Object.keys(this.checkboxAttrs)) {
      if (!(key in newCheckboxAttrs)) {
        clearNativeProp(this.windowId, this.id, key);
      }
    }
    this.checkboxAttrs = newCheckboxAttrs;
  }

  override destroy(): void {
    this.unbindOnChange();
    super.destroy();
  }
}
