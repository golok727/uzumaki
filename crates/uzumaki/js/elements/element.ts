import core, { type CoreNode } from '../core';
import { dispatchDomEvent, dispatchEvent } from '../dispatcher';
import { UzEventTarget, type ListenerOptions } from '../event-target';
import {
  EVENT_NAME_TO_TYPE,
  EventPhase,
  UzEvent,
  type EventName,
  type UzEventMap,
} from '../events';
import { UzNode } from '../node';
import type { Window } from '../window';

export class Element<M extends UzEventMap = UzEventMap> extends UzNode {
  private _elementId: string | null = null;
  /** @internal */
  readonly _emitter: UzEventTarget<M>;

  constructor(window: Window, native: CoreNode) {
    super(window, native);
    this._emitter = new UzEventTarget<M>({
      dispatch: (name, event) => {
        if (typeof name !== 'string') return;
        const type = EVENT_NAME_TO_TYPE[name];
        if (type === undefined) return;
        if (event instanceof UzEvent) {
          if (!event.target) event._setTarget(this);
          if (!event.bubbles) {
            event._setPhase(EventPhase.Target);
            event.currentTarget = this;
            return this._emitter._emitLocal(
              name as keyof M,
              event as M[keyof M],
            );
          }
          return dispatchEvent(
            this._window,
            name as EventName,
            this.nodeId,
            event,
          );
        }
        return dispatchDomEvent(this._window, type, this.nodeId, event);
      },
    });
  }

  get id(): string | null {
    return this._elementId;
  }

  set id(value: string | null) {
    this._elementId =
      typeof value === 'string' && value.length > 0 ? value : null;
  }

  on<K extends keyof M>(
    name: K,
    handler: (event: M[K]) => void,
    options?: ListenerOptions,
  ): void {
    this._emitter.on(name, handler, options);
  }

  off<K extends keyof M>(
    name: K,
    handler: (event: M[K]) => void,
    options?: ListenerOptions,
  ): void {
    this._emitter.off(name, handler, options);
  }

  emit<K extends keyof M>(name: K, event: M[K]): boolean {
    return this._emitter.emit(name, event);
  }

  focus(): void {
    core.focusElement(this._window.id, this._native.id);
  }

  setAttribute(name: string, value: unknown): void {
    if (value == null) {
      this.removeAttribute(name);
      return;
    }
    if (typeof value === 'boolean') {
      this._native.setBoolAttribute(name, value);
    } else if (typeof value === 'number') {
      this._native.setNumberAttribute(name, value);
    } else {
      this._native.setStrAttribute(name, String(value));
    }
  }

  setAttributes(attributes: Record<string, unknown>): void {
    for (const [key, value] of Object.entries(attributes)) {
      this.setAttribute(key, value);
    }
  }

  removeAttribute(name: string): void {
    this._native.removeAttribute(name);
  }

  getAttribute(name: string): unknown {
    return this._native.getAttribute(name);
  }

  override destroy(): void {
    this._emitter._clear();
    super.destroy();
  }
}

export function createNativeElement(window: Window, type: string): CoreNode {
  return core.createElementNode(window.id, type);
}

export function getNativeRootNode(window: Window): CoreNode {
  return core.getRootNode(window.id);
}
