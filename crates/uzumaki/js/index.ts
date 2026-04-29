import { eventManager, EventType } from './events';
import { disposeWindow, Window } from './window';

export { Window } from './window';
export { UzNode, UzTextNode } from './node';
export { Element } from './elements/element';
export { UzElement } from './elements/base';
export { UzRootElement } from './elements/root';
export { UzImageElement } from './elements/image';
export { Clipboard } from './clipboard';
export { eventManager, EventType } from './events';
export type {
  EventPhase,
  EventName,
  EventHandler,
  EventHandlerMap,
  UzumakiEvent,
  UzumakiMouseEvent,
  UzumakiKeyboardEvent,
  UzumakiInputEvent,
  UzumakiFocusEvent,
  UzumakiClipboardEvent,
} from './events';

interface AppEvent {
  type: string;
  windowId: number;
  nodeId?: any;
  key?: string;
  code?: string;
  keyCode?: number;
  modifiers?: number;
  repeat?: boolean;
  width?: number;
  height?: number;
  x?: number;
  y?: number;
  screenX?: number;
  screenY?: number;
  button?: number;
  buttons?: number;
  value?: string;
  inputType?: string;
  data?: string | null;
}

const EVENT_TYPE_MAP: Record<string, EventType> = {
  mouseDown: EventType.MouseDown,
  mouseUp: EventType.MouseUp,
  click: EventType.Click,
  keyDown: EventType.KeyDown,
  keyUp: EventType.KeyUp,
  input: EventType.Input,
  focus: EventType.Focus,
  blur: EventType.Blur,
  copy: EventType.Copy,
  cut: EventType.Cut,
  paste: EventType.Paste,
};

(globalThis as unknown as any).__uzumaki_on_app_event__ = function (
  event: AppEvent,
): boolean {
  // WindowLoad is a special event dispatched directly to window handlers
  if (event.type === 'windowLoad') {
    eventManager.dispatchWindowEvent(event.windowId, EventType.WindowLoad);
    return false;
  }

  if (event.type === 'windowClose') {
    const w = Window._getById(event.windowId);
    if (w) {
      disposeWindow(w);
    }
    return false;
  }

  if (event.type === 'hotReload') {
    console.log('[uzumaki] Hot reload');
    return false;
  }

  if (event.type === 'resize') {
    // todo: dispatch resize event to window handlers
    return false;
  }

  const eventType = EVENT_TYPE_MAP[event.type];
  if (eventType === undefined) return false;

  // Always dispatch — no more nodeId guards.
  // Events without a target node will only fire window-level handlers.
  return eventManager.onRawEvent(
    eventType,
    event.windowId,
    event.nodeId ?? null,
    event,
  );
};
