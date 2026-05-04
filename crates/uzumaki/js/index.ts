import core from './core';
import { dispatchDomEvent } from './dispatcher';
import { EventType } from './events';
import { disposeWindow, Window } from './window';

export { getWindow, Window } from './window';
export type {
  WindowOptions,
  WindowLevel,
  WindowPosition,
  WindowSize,
  WindowTheme,
} from './types';
export { UzNode, UzTextNode } from './node';
export { Element } from './elements/element';
// todo cleanup imports
export { UzElement } from './elements/base';
export { UzRootElement } from './elements/root';
export { UzViewElement } from './elements/view';
export { UzTextElement } from './elements/text';
export { UzButtonElement } from './elements/button';
export { UzImageElement } from './elements/image';
export { UzInputElement } from './elements/input';
export { UzCheckboxElement } from './elements/checkbox';

export { Clipboard } from './clipboard';
export { UzEventTarget as EventEmitter } from './event-target';
export { EventType, UzEvent } from './events';
export { EventPhase } from './events';
export type {
  EventName,
  EventHandler,
  UzEventMap as EventHandlerMap,
  WindowEventName,
  WindowEventHandler,
  WindowEventMap,
  UzumakiEvent,
  UzMouseEvent as UzumakiMouseEvent,
  UzKeyboardEvent as UzumakiKeyboardEvent,
  UzInputEvent as UzumakiInputEvent,
  UzFocusEvent as UzumakiFocusEvent,
  UzClipboardEvent as UzumakiClipboardEvent,
  UzumakiResizeEvent,
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

core.onAppEvent((event: AppEvent, ctx) => {
  if (event.type === 'windowLoad') {
    const w = Window._getById(event.windowId);
    if (w) w._dispatchLifecycle('load');
    return;
  }

  if (event.type === 'windowClose') {
    const w = Window._getById(event.windowId);
    if (w) {
      w._dispatchLifecycle('close');
      disposeWindow(w);
    }
    return;
  }

  if (event.type === 'resize') {
    const w = Window._getById(event.windowId);
    if (w) {
      w._dispatchLifecycle('resize', {
        width: event.width ?? 0,
        height: event.height ?? 0,
      });
    }
    return;
  }

  if (event.type === 'hotReload') {
    console.log('[uzumaki] Hot reload');
    return;
  }

  const eventType = EVENT_TYPE_MAP[event.type];
  if (eventType === undefined) return;

  const w = Window._getById(event.windowId);
  if (!w) return;

  const prevented = dispatchDomEvent(w, eventType, event.nodeId ?? null, event);
  if (prevented) ctx.preventDefault();
});
