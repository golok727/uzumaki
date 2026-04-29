import { CHECKBOX_ATTR_NAMES } from '../constants';
import { createNativeElement } from '../core/element';
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
    super(createNativeElement(window, 'checkbox'), 'checkbox', window);
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
      if (key === 'id') {
        this.setElementIdProp(value);
        continue;
      }
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
      this.setAttribute(key, val);
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
    this.setAttribute('interactive', true);
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

    this.setElementIdProp(newProps.id);
    for (const key in newProps) {
      if (
        key === 'children' ||
        key === 'key' ||
        key === 'ref' ||
        key === 'id' ||
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
        this.setAttribute(key, val);
      }
    }
    for (const key of Object.keys(this.checkboxAttrs)) {
      if (!(key in newCheckboxAttrs)) {
        this.removeAttribute(key);
      }
    }
    this.checkboxAttrs = newCheckboxAttrs;
  }

  override destroy(): void {
    this.unbindOnChange();
    super.destroy();
  }
}
