import type { NativeElement } from '../core';
import { CoreElement } from '../core/element';
import { eventManager } from '../events';
import { ListenerEntry } from '../types';
import { Window } from '../window';

export abstract class BaseElement<
  TProps extends Record<string, any> = Record<string, any>,
> extends CoreElement {
  /** User-supplied string id from the `id` prop, or null. */
  elementId: string | null = null;
  styles: Record<string, any> = {};
  /** Keyed by stable event identity (name + phase). */
  eventListeners: Map<string, ListenerEntry> = new Map();

  constructor(native: NativeElement, type: string, window: Window) {
    super(window, native, type);
  }

  setElementIdProp(value: any): void {
    this.elementId =
      typeof value === 'string' && value.length > 0 ? value : null;
  }

  abstract commitUpdate(newProps: TProps, oldProps: TProps): void;

  applyStyles(): void {
    for (const [key, val] of Object.entries(this.styles)) {
      this.setAttribute(key, val);
    }
  }

  applyEvents(): void {
    if (this.eventListeners.size > 0) {
      this.setAttribute('interactive', true);
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
        this.setAttribute(key, val);
      }
    }
    for (const key of Object.keys(this.styles)) {
      if (!(key in newStyles)) {
        this.removeAttribute(key);
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
      this.setAttribute('interactive', true);
    } else if (newListeners.size === 0 && this.eventListeners.size > 0) {
      this.setAttribute('interactive', false);
    }
    this.eventListeners = newListeners;
  }

  destroy(): void {
    this.eventListeners.clear();
    super.destroy();
  }
}
