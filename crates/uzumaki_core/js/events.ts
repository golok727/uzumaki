// ── Event Types (shared numeric enum — Rust mirrors these values) ────

export const enum EventType {
  MouseMove = 0,
  MouseDown = 1,
  MouseUp   = 2,
  Click     = 3,
  KeyDown   = 10,
  KeyUp     = 11,
  Input     = 20,
  Focus     = 21,
  Blur      = 22,
}

// ── Event interfaces ─────────────────────────────────────────────────

export interface UzumakiEvent {
  type: EventType;
  target: any;
  currentTarget: any;
  bubbles: boolean;
  defaultPrevented: boolean;
  stopPropagation(): void;
  preventDefault(): void;
}

export interface UzumakiMouseEvent extends UzumakiEvent {
  x: number;
  y: number;
  screenX: number;
  screenY: number;
  button: number;   // 0=left 1=mid 2=right
  buttons: number;  // bitmask
}

export interface UzumakiKeyboardEvent extends UzumakiEvent {
  key: string;      // logical: "Enter", "a"
  code: string;     // physical: "KeyA"
  keyCode: number;
  repeat: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}

export interface UzumakiInputEvent extends UzumakiEvent {
  value: string;
  inputType: string;
  data: string | null;
}

export interface UzumakiFocusEvent extends UzumakiEvent {}

// ── Helpers ──────────────────────────────────────────────────────────

const EVENT_NAME_TO_TYPE: Record<string, EventType> = {
  mousemove: EventType.MouseMove,
  mousedown: EventType.MouseDown,
  mouseup:   EventType.MouseUp,
  click:     EventType.Click,
  keydown:   EventType.KeyDown,
  keyup:     EventType.KeyUp,
  input:     EventType.Input,
  focus:     EventType.Focus,
  blur:      EventType.Blur,
};

function nodeKey(id: any): string {
  return JSON.stringify(id);
}

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

// ── EventManager ─────────────────────────────────────────────────────

export class EventManager {
  // nodeKey -> EventType -> Set<handler>
  private handlers = new Map<string, Map<EventType, Set<Function>>>();
  // nodeKey -> raw parentNodeId (for bubbling)
  private parentMap = new Map<string, any>();
  // focus tracking
  private _focusNode: any = null;

  // ── Focus ────────────────────────────────────────────────────────

  setFocus(nodeId: any): void {
    this._focusNode = nodeId;
  }

  getFocus(): any {
    return this._focusNode;
  }

  // ── Handler registry ─────────────────────────────────────────────

  addHandler(nodeId: any, eventType: EventType, handler: Function): void {
    const key = nodeKey(nodeId);
    let typeMap = this.handlers.get(key);
    if (!typeMap) {
      typeMap = new Map();
      this.handlers.set(key, typeMap);
    }
    let set = typeMap.get(eventType);
    if (!set) {
      set = new Set();
      typeMap.set(eventType, set);
    }
    set.add(handler);
  }

  removeHandler(nodeId: any, eventType: EventType, handler: Function): void {
    const key = nodeKey(nodeId);
    const typeMap = this.handlers.get(key);
    if (!typeMap) return;
    const set = typeMap.get(eventType);
    if (!set) return;
    set.delete(handler);
    if (set.size === 0) typeMap.delete(eventType);
    if (typeMap.size === 0) this.handlers.delete(key);
  }

  clearHandlersForType(nodeId: any, eventType: EventType): void {
    const key = nodeKey(nodeId);
    const typeMap = this.handlers.get(key);
    if (!typeMap) return;
    typeMap.delete(eventType);
    if (typeMap.size === 0) this.handlers.delete(key);
  }

  clearNode(nodeId: any): void {
    const key = nodeKey(nodeId);
    this.handlers.delete(key);
    this.parentMap.delete(key);
    if (this._focusNode != null && nodeKey(this._focusNode) === key) {
      this._focusNode = null;
    }
  }

  hasHandlers(nodeId: any): boolean {
    const typeMap = this.handlers.get(nodeKey(nodeId));
    return typeMap != null && typeMap.size > 0;
  }

  // ── Parent tracking (for bubbling) ───────────────────────────────

  setParent(childId: any, parentId: any): void {
    this.parentMap.set(nodeKey(childId), parentId);
  }

  removeParent(childId: any): void {
    this.parentMap.delete(nodeKey(childId));
  }

  // ── Convenience: string event name ↔ EventType ───────────────────

  addHandlerByName(nodeId: any, eventName: string, handler: Function): void {
    const t = EVENT_NAME_TO_TYPE[eventName];
    if (t !== undefined) this.addHandler(nodeId, t, handler);
  }

  removeHandlerByName(nodeId: any, eventName: string, handler: Function): void {
    const t = EVENT_NAME_TO_TYPE[eventName];
    if (t !== undefined) this.removeHandler(nodeId, t, handler);
  }

  clearHandlersByName(nodeId: any, eventName: string): void {
    const t = EVENT_NAME_TO_TYPE[eventName];
    if (t !== undefined) this.clearHandlersForType(nodeId, t);
  }

  // ── Raw event entry point (called from the bridge) ───────────────

  onRawEvent(type: EventType, targetNodeId: any, payload: any): void {
    let target = targetNodeId;

    // Keyboard events route to the focus node, not the hit-test target
    if (isKeyboardType(type)) {
      target = this._focusNode;
      if (target == null) return;
    }

    // Build the event object
    let stopped = false;
    let prevented = false;

    const base = {
      type,
      target,
      currentTarget: target,
      bubbles: true,
      get defaultPrevented() { return prevented; },
      stopPropagation() { stopped = true; },
      preventDefault() { prevented = true; },
    };

    let event: UzumakiEvent;

    if (isMouseType(type)) {
      event = {
        ...base,
        x: payload?.x ?? 0,
        y: payload?.y ?? 0,
        screenX: payload?.screenX ?? 0,
        screenY: payload?.screenY ?? 0,
        button: payload?.button ?? 0,
        buttons: payload?.buttons ?? 0,
      } as UzumakiMouseEvent;
    } else if (isKeyboardType(type)) {
      const mods: number = payload?.modifiers ?? 0;
      event = {
        ...base,
        key: payload?.key ?? '',
        code: payload?.code ?? '',
        keyCode: payload?.keyCode ?? 0,
        repeat: payload?.repeat ?? false,
        ctrlKey:  !!(mods & 1),
        altKey:   !!(mods & 2),
        shiftKey: !!(mods & 4),
        metaKey:  !!(mods & 8),
      } as UzumakiKeyboardEvent;
    } else if (isInputType(type)) {
      event = {
        ...base,
        value: payload?.value ?? '',
        inputType: payload?.inputType ?? '',
        data: payload?.data ?? null,
      } as UzumakiInputEvent;
    } else if (isFocusType(type)) {
      event = base as UzumakiFocusEvent;
    } else {
      return;
    }

    // Bubble: target → root
    let currentId: any = target;
    bubble: while (currentId != null) {
      event.currentTarget = currentId;

      const key = nodeKey(currentId);
      const typeMap = this.handlers.get(key);
      if (typeMap) {
        const handlers = typeMap.get(type);
        if (handlers) {
          for (const h of handlers) {
            h(event);
            if (stopped) break bubble;
          }
        }
      }

      if (!event.bubbles) break;
      currentId = this.parentMap.get(key) ?? null;
    }

    // Post-bubble hook — placeholder for defaultPrevented handling
    this.postBubble(event);
  }

  private postBubble(_event: UzumakiEvent): void {
    // No-op hook. Will be used by InputEvent to handle defaultPrevented.
  }

  // ── Reset ────────────────────────────────────────────────────────

  clear(): void {
    this.handlers.clear();
    this.parentMap.clear();
    this._focusNode = null;
  }
}

// Re-export singleton
export const eventManager = new EventManager();
