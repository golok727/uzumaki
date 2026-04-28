import core from '../core';
import { ListenerEntry } from '../types';
import {
  assignNativeStyle,
  isEventProp,
  listenerKey,
  parseEventProp,
} from '../utils';
import { Window } from '../window';
import { BaseElement } from './base';

export class TextElement extends BaseElement<Record<string, any>> {
  textContent: string;

  constructor(
    window: Window,
    type: string,
    text: string,
    props: Record<string, any>,
    public getTextContent: (children: any) => string,
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
      } else {
        assignNativeStyle(this.styles, key, value);
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

    this.setElementIdProp(newProps.id);
    for (const key in newProps) {
      if (key === 'children' || key === 'key' || key === 'ref' || key === 'id')
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
      } else {
        assignNativeStyle(newStyles, key, value);
      }
    }

    this.updateStyles(newStyles);
    this.updateEvents(newEvents);

    const newText = this.getTextContent(newProps.children);
    this.setText(newText);
  }
}
