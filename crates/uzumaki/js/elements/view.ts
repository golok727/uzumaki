import core from '../core';
import { ListenerEntry } from '../types';
import {
  assignNativeStyle,
  isEventProp,
  listenerKey,
  parseEventProp,
  isNativeAttribute,
} from '../utils';
import { Window } from '../window';
import { BaseElement } from './base';

export class ViewElement extends BaseElement<Record<string, any>> {
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
      } else if (isNativeAttribute(key)) {
        assignNativeStyle(this.styles, key, value);
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
      } else if (isNativeAttribute(key)) {
        assignNativeStyle(newStyles, key, value);
      }
    }

    this.updateStyles(newStyles);
    this.updateEvents(newEvents);
  }
}
