import { eventManager, EventType } from './events';

export { Window } from './window';
export { eventManager, EventType } from './events';
export type {
  UzumakiEvent,
  UzumakiMouseEvent,
  UzumakiKeyboardEvent,
  UzumakiInputEvent,
  UzumakiFocusEvent,
} from './events';

interface AppEvent {
  type: string;
  windowId?: number;
  nodeId?: any;
  key?: string;
  width?: number;
  height?: number;
}

(globalThis as unknown as any).__uzumaki_on_app_event__ = function (
  event: AppEvent,
) {
  switch (event.type) {
    case 'mouseDown':
      if (event.nodeId != null) {
        eventManager.onRawEvent(EventType.MouseDown, event.nodeId, event);
      }
      break;
    case 'mouseUp':
      if (event.nodeId != null) {
        eventManager.onRawEvent(EventType.MouseUp, event.nodeId, event);
      }
      break;
    case 'click':
      if (event.nodeId != null) {
        eventManager.onRawEvent(EventType.Click, event.nodeId, event);
      }
      break;
    case 'keyDown':
      eventManager.onRawEvent(EventType.KeyDown, null, event);
      break;
    case 'keyUp':
      eventManager.onRawEvent(EventType.KeyUp, null, event);
      break;
    case 'input':
      if (event.nodeId != null) {
        eventManager.onRawEvent(EventType.Input, event.nodeId, event);
      }
      break;
    case 'focus':
      if (event.nodeId != null) {
        eventManager.onRawEvent(EventType.Focus, event.nodeId, event);
      }
      break;
    case 'blur':
      if (event.nodeId != null) {
        eventManager.onRawEvent(EventType.Blur, event.nodeId, event);
      }
      break;
    case 'resize':
      break;
    case 'hotReload':
      // todo this doesnt work :p
      console.log('[uzumaki] Hot reload');
      break;
  }
};
