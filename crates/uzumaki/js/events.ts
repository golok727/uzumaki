import type { UzNode } from './node';
import type { NodeId } from './types';

import core from './core';
import { getNode } from './registry';

export const enum EventType {
  MouseMove = 0,
  MouseDown = 1,
  MouseUp = 2,
  Click = 3,
  KeyDown = 10,
  KeyUp = 11,
  Input = 20,
  Focus = 21,
  Blur = 22,
  Copy = 25,
  Cut = 26,
  Paste = 27,
  WindowLoad = 30,
}

export const enum EventPhase {
  None = 0,
  Capture = 1,
  Target = 2,
  Bubble = 3,
}

export interface UzumakiEvent {
  readonly type: EventType;
  readonly target: UzNode | null;
  currentTarget: UzNode | null;
  readonly eventPhase: EventPhase;
  readonly bubbles: boolean;
  readonly defaultPrevented: boolean;
  stopPropagation(): void;
  stopImmediatePropagation(): void;
  preventDefault(): void;
}

export interface UzumakiMouseEvent extends UzumakiEvent {
  readonly x: number;
  readonly y: number;
  readonly screenX: number;
  readonly screenY: number;
  readonly button: number;
  readonly buttons: number;
}

export interface UzumakiKeyboardEvent extends UzumakiEvent {
  readonly key: string;
  readonly code: string;
  readonly keyCode: number;
  readonly repeat: boolean;
  readonly ctrlKey: boolean;
  readonly altKey: boolean;
  readonly shiftKey: boolean;
  readonly metaKey: boolean;
}

export interface UzumakiInputEvent extends UzumakiEvent {
  readonly value: string;
  readonly inputType: string;
  readonly data: string | null;
}

export interface UzumakiFocusEvent extends UzumakiEvent {}

export interface UzumakiClipboardEvent extends UzumakiEvent {
  readonly selectionText: string | null;
  readonly clipboardText: string | null;
}

export interface EventHandlerMap {
  mousemove: UzumakiMouseEvent;
  mousedown: UzumakiMouseEvent;
  mouseup: UzumakiMouseEvent;
  click: UzumakiMouseEvent;
  keydown: UzumakiKeyboardEvent;
  keyup: UzumakiKeyboardEvent;
  input: UzumakiInputEvent;
  focus: UzumakiFocusEvent;
  blur: UzumakiFocusEvent;
  copy: UzumakiClipboardEvent;
  cut: UzumakiClipboardEvent;
  paste: UzumakiClipboardEvent;
  windowload: UzumakiEvent;
}

export type EventName = keyof EventHandlerMap;

export type EventHandler<K extends EventName = EventName> = (
  event: EventHandlerMap[K],
) => void;

const EVENT_NAME_TO_TYPE: Record<string, EventType> = {
  mousemove: EventType.MouseMove,
  mousedown: EventType.MouseDown,
  mouseup: EventType.MouseUp,
  click: EventType.Click,
  keydown: EventType.KeyDown,
  keyup: EventType.KeyUp,
  input: EventType.Input,
  focus: EventType.Focus,
  blur: EventType.Blur,
  copy: EventType.Copy,
  cut: EventType.Cut,
  paste: EventType.Paste,
  windowload: EventType.WindowLoad,
};

export { EVENT_NAME_TO_TYPE };

const NON_BUBBLING: Set<EventType> = new Set([
  EventType.Focus,
  EventType.Blur,
  EventType.WindowLoad,
]);

function isMouseType(t: EventType): boolean {
  return t >= 0 && t <= 3;
}

function isKeyboardType(t: EventType): boolean {
  return t >= 10 && t <= 11;
}

function isInputType(t: EventType): boolean {
  return t === EventType.Input;
}

function isFocusType(t: EventType): boolean {
  return t === EventType.Focus || t === EventType.Blur;
}

function isClipboardType(t: EventType): boolean {
  return t === EventType.Copy || t === EventType.Cut || t === EventType.Paste;
}

function nodeOf(windowId: number, id: NodeId | null): UzNode | null {
  if (id == null) return null;
  return getNode(windowId, id) ?? null;
}

interface HandlerEntry {
  handler: Function;
  capture: boolean;
}

type NodeHandlers = Map<EventType, HandlerEntry[]>;

export class EventManager {
  // (windowId -> nodeId -> handlers) — node ids are scoped to a window.
  private handlers = new Map<number, Map<NodeId, NodeHandlers>>();
  private windowHandlers = new Map<number, Map<EventType, HandlerEntry[]>>();

  private nodeBucket(
    windowId: number,
    nodeId: NodeId,
    create: boolean,
  ): NodeHandlers | undefined {
    let perWindow = this.handlers.get(windowId);
    if (!perWindow) {
      if (!create) return undefined;
      perWindow = new Map();
      this.handlers.set(windowId, perWindow);
    }
    let typeMap = perWindow.get(nodeId);
    if (!typeMap && create) {
      typeMap = new Map();
      perWindow.set(nodeId, typeMap);
    }
    return typeMap;
  }

  addHandler(
    windowId: number,
    nodeId: NodeId,
    eventType: EventType,
    handler: Function,
    capture = false,
  ): void {
    const typeMap = this.nodeBucket(windowId, nodeId, true)!;
    let entries = typeMap.get(eventType);
    if (!entries) {
      entries = [];
      typeMap.set(eventType, entries);
    }
    entries.push({ handler, capture });
  }

  removeHandler(
    windowId: number,
    nodeId: NodeId,
    eventType: EventType,
    handler: Function,
    capture = false,
  ): void {
    const typeMap = this.nodeBucket(windowId, nodeId, false);
    if (!typeMap) return;
    const entries = typeMap.get(eventType);
    if (!entries) return;
    const idx = entries.findIndex(
      (e) => e.handler === handler && e.capture === capture,
    );
    if (idx !== -1) entries.splice(idx, 1);
    if (entries.length === 0) typeMap.delete(eventType);
    if (typeMap.size === 0) {
      const perWindow = this.handlers.get(windowId);
      perWindow?.delete(nodeId);
      if (perWindow && perWindow.size === 0) this.handlers.delete(windowId);
    }
  }

  clearNode(windowId: number, nodeId: NodeId): void {
    const perWindow = this.handlers.get(windowId);
    if (!perWindow) return;
    perWindow.delete(nodeId);
    if (perWindow.size === 0) this.handlers.delete(windowId);
  }

  hasHandlers(windowId: number, nodeId: NodeId): boolean {
    const typeMap = this.nodeBucket(windowId, nodeId, false);
    return typeMap != null && typeMap.size > 0;
  }

  addHandlerByName(
    windowId: number,
    nodeId: NodeId,
    eventName: string,
    handler: Function,
    capture = false,
  ): void {
    const t = EVENT_NAME_TO_TYPE[eventName];
    if (t !== undefined) this.addHandler(windowId, nodeId, t, handler, capture);
  }

  removeHandlerByName(
    windowId: number,
    nodeId: NodeId,
    eventName: string,
    handler: Function,
    capture = false,
  ): void {
    const t = EVENT_NAME_TO_TYPE[eventName];
    if (t !== undefined)
      this.removeHandler(windowId, nodeId, t, handler, capture);
  }

  addWindowHandler(
    windowId: number,
    eventType: EventType,
    handler: Function,
    capture = false,
  ): void {
    let typeMap = this.windowHandlers.get(windowId);
    if (!typeMap) {
      typeMap = new Map();
      this.windowHandlers.set(windowId, typeMap);
    }
    let entries = typeMap.get(eventType);
    if (!entries) {
      entries = [];
      typeMap.set(eventType, entries);
    }
    entries.push({ handler, capture });
  }

  removeWindowHandler(
    windowId: number,
    eventType: EventType,
    handler: Function,
    capture = false,
  ): void {
    const typeMap = this.windowHandlers.get(windowId);
    if (!typeMap) return;
    const entries = typeMap.get(eventType);
    if (!entries) return;
    const idx = entries.findIndex(
      (e) => e.handler === handler && e.capture === capture,
    );
    if (idx !== -1) entries.splice(idx, 1);
    if (entries.length === 0) typeMap.delete(eventType);
    if (typeMap.size === 0) this.windowHandlers.delete(windowId);
  }

  clearWindowHandlers(windowId: number): void {
    this.windowHandlers.delete(windowId);
    this.handlers.delete(windowId);
  }

  addWindowHandlerByName(
    windowId: number,
    eventName: string,
    handler: Function,
    capture = false,
  ): void {
    const t = EVENT_NAME_TO_TYPE[eventName];
    if (t !== undefined) this.addWindowHandler(windowId, t, handler, capture);
  }

  removeWindowHandlerByName(
    windowId: number,
    eventName: string,
    handler: Function,
    capture = false,
  ): void {
    const t = EVENT_NAME_TO_TYPE[eventName];
    if (t !== undefined)
      this.removeWindowHandler(windowId, t, handler, capture);
  }

  private fireHandlers(
    windowId: number,
    nodeId: number,
    type: EventType,
    event: UzumakiEvent,
    capturePhase: boolean,
  ): { stopped: boolean; stoppedImmediate: boolean } {
    let stopped = false;
    let stoppedImmediate = false;

    const typeMap = this.nodeBucket(windowId, nodeId, false);
    if (!typeMap) return { stopped, stoppedImmediate };
    const entries = typeMap.get(type);
    if (!entries) return { stopped, stoppedImmediate };

    for (const entry of entries) {
      // target phase fires both capture and bubble handlers
      if (
        event.eventPhase === EventPhase.Target ||
        entry.capture === capturePhase
      ) {
        entry.handler(event);
        if ((event as any)._stoppedImmediate) {
          stoppedImmediate = true;
          stopped = true;
          break;
        }
        if ((event as any)._stopped) {
          stopped = true;
        }
      }
    }

    return { stopped, stoppedImmediate };
  }

  private fireWindowHandlers(
    windowId: number,
    type: EventType,
    event: UzumakiEvent,
    capturePhase: boolean,
  ): { stopped: boolean; stoppedImmediate: boolean } {
    let stopped = false;
    let stoppedImmediate = false;

    const typeMap = this.windowHandlers.get(windowId);
    if (!typeMap) return { stopped, stoppedImmediate };
    const entries = typeMap.get(type);
    if (!entries) return { stopped, stoppedImmediate };

    for (const entry of entries) {
      if (entry.capture === capturePhase) {
        entry.handler(event);
        if ((event as any)._stoppedImmediate) {
          stoppedImmediate = true;
          stopped = true;
          break;
        }
        if ((event as any)._stopped) {
          stopped = true;
        }
      }
    }

    return { stopped, stoppedImmediate };
  }

  /**
   * Dispatch an event through the capture -> target -> bubble phases.
   * Returns true if `preventDefault()` was called.
   */
  onRawEvent(
    type: EventType,
    windowId: number,
    targetNodeId: NodeId | null,
    payload: any,
  ): boolean {
    const bubbles = !NON_BUBBLING.has(type);

    let path: NodeId[] = [];
    if (targetNodeId != null) {
      path = core.getAncestorPath(windowId, targetNodeId);
    }

    let _stopped = false;
    let _prevented = false;
    let _eventPhase: EventPhase = EventPhase.None;

    const targetNode = nodeOf(windowId, targetNodeId);

    const base: UzumakiEvent = {
      type,
      target: targetNode,
      currentTarget: targetNode,
      get eventPhase(): EventPhase {
        return _eventPhase;
      },
      bubbles,
      get defaultPrevented(): boolean {
        return _prevented;
      },
      stopPropagation() {
        _stopped = true;
        (base as any)._stopped = true;
      },
      stopImmediatePropagation() {
        _stopped = true;
        (base as any)._stopped = true;
        (base as any)._stoppedImmediate = true;
      },
      preventDefault() {
        _prevented = true;
      },
    };

    (base as any)._stopped = false;
    (base as any)._stoppedImmediate = false;

    let event: UzumakiEvent;

    if (isMouseType(type)) {
      event = Object.assign(base, {
        x: payload?.x ?? 0,
        y: payload?.y ?? 0,
        screenX: payload?.screenX ?? 0,
        screenY: payload?.screenY ?? 0,
        button: payload?.button ?? 0,
        buttons: payload?.buttons ?? 0,
      }) as UzumakiMouseEvent;
    } else if (isKeyboardType(type)) {
      const mods: number = payload?.modifiers ?? 0;
      event = Object.assign(base, {
        key: payload?.key ?? '',
        code: payload?.code ?? '',
        keyCode: payload?.keyCode ?? 0,
        repeat: payload?.repeat ?? false,
        ctrlKey: !!(mods & 1),
        altKey: !!(mods & 2),
        shiftKey: !!(mods & 4),
        metaKey: !!(mods & 8),
      }) as UzumakiKeyboardEvent;
    } else if (isInputType(type)) {
      event = Object.assign(base, {
        value: payload?.value ?? '',
        inputType: payload?.inputType ?? '',
        data: payload?.data ?? null,
      }) as UzumakiInputEvent;
    } else if (isClipboardType(type)) {
      event = Object.assign(base, {
        selectionText: payload?.selectionText ?? null,
        clipboardText: payload?.clipboardText ?? null,
      }) as UzumakiClipboardEvent;
    } else if (isFocusType(type)) {
      event = base as UzumakiFocusEvent;
    } else {
      event = base;
    }

    // no DOM target (e.g. click on empty space)
    if (path.length === 0) {
      _eventPhase = EventPhase.Bubble;
      event.currentTarget = null;
      this.fireWindowHandlers(windowId, type, event, false);
      return _prevented;
    }

    // capture: window -> root -> ... -> parent of target
    _eventPhase = EventPhase.Capture;

    if (!_stopped) {
      event.currentTarget = null;
      const res = this.fireWindowHandlers(windowId, type, event, true);
      if (res.stopped) _stopped = true;
    }

    for (let i = path.length - 1; i > 0 && !_stopped; i--) {
      event.currentTarget = nodeOf(windowId, path[i]);
      const res = this.fireHandlers(windowId, path[i], type, event, true);
      if (res.stopped) _stopped = true;
    }

    // target
    if (!_stopped) {
      _eventPhase = EventPhase.Target;
      event.currentTarget = nodeOf(windowId, path[0]);
      const res = this.fireHandlers(windowId, path[0], type, event, false);
      if (res.stopped) _stopped = true;
    }

    // bubble: target -> ... -> root -> window
    if (bubbles && !_stopped) {
      _eventPhase = EventPhase.Bubble;

      for (let i = 1; i < path.length && !_stopped; i++) {
        event.currentTarget = nodeOf(windowId, path[i]);
        const res = this.fireHandlers(windowId, path[i], type, event, false);
        if (res.stopped) _stopped = true;
      }

      if (!_stopped) {
        event.currentTarget = null;
        this.fireWindowHandlers(windowId, type, event, false);
      }
    }

    return _prevented;
  }

  dispatchWindowEvent(windowId: number, type: EventType): void {
    let _prevented = false;
    const event: UzumakiEvent = {
      type,
      target: null,
      currentTarget: null,
      eventPhase: EventPhase.Target,
      bubbles: false,
      get defaultPrevented(): boolean {
        return _prevented;
      },
      stopPropagation() {},
      stopImmediatePropagation() {},
      preventDefault() {
        _prevented = true;
      },
    };

    const typeMap = this.windowHandlers.get(windowId);
    if (!typeMap) return;
    const entries = typeMap.get(type);
    if (!entries) return;
    for (const entry of entries) {
      entry.handler(event);
    }
  }

  clear(): void {
    this.handlers.clear();
    this.windowHandlers.clear();
  }
}

export const eventManager = new EventManager();
