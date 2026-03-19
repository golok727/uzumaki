import { Application, createWindow, pollEvents, resetDom, setRemBase } from './bindings';
import { requestQuit } from './bindings';
import { eventManager, EventType } from './events';
import type { UzumakiMouseEvent, UzumakiKeyboardEvent, UzumakiEvent } from './events';

export interface WindowAttributes {
  width: number;
  height: number;
  title: string;
}

const windowRegistry = new Map<string, Window>();

type EventHandler = (ev: UzumakiEvent) => void;

export class Window {
  private _width!: number;
  private _height!: number;
  private _label!: string;
  private _id!: number;
  private _remBase: number = 16;
  private _eventId!: string;

  constructor(
    label: string,
    {
      width = 800,
      height = 600,
      title = 'uzumaki',
    }: Partial<WindowAttributes> = {},
  ) {
    // Return existing window: for hot reload
    const existing = windowRegistry.get(label);
    if (existing) {
      return existing;
    }

    this._width = width;
    this._height = height;
    this._label = label;
    this._id = createWindow({ width, height, title });
    this._eventId = `__window_${this._id}`;
    windowRegistry.set(label, this);
  }

  close() {
    eventManager.clearNode(this._eventId);
  }

  setSize(width: number, height: number) {
    this._width = width;
    this._height = height;
  }

  get width(): number {
    return this._width;
  }

  get height(): number {
    return this._height;
  }

  get label(): string {
    return this._label;
  }

  get id(): number {
    return this._id;
  }

  get eventId(): string {
    return this._eventId;
  }

  get remBase(): number {
    return this._remBase;
  }

  set remBase(value: number) {
    this._remBase = value;
    setRemBase(this._id, value);
  }

  on(eventName: string, handler: EventHandler): void {
    eventManager.addHandlerByName(this._eventId, eventName, handler);
  }

  off(eventName: string, handler: EventHandler): void {
    eventManager.removeHandlerByName(this._eventId, eventName, handler);
  }
}

export { render } from './react';
export { eventManager, EventType } from './events';
export type { UzumakiEvent, UzumakiMouseEvent, UzumakiKeyboardEvent, UzumakiInputEvent, UzumakiFocusEvent } from './events';

interface AppEvent {
  type: string;
  windowId?: number;
  nodeId?: any;
  key?: string;
  width?: number;
  height?: number;
}

export async function runApp({
  entryFilePath,
  title = 'uzumaki',
  hot = false,
}: {
  entryFilePath: string;
  title?: string;
  hot?: boolean;
}) {
  process.env.WGPU_POWER_PREF = 'high';

  const app = new Application();

  let exiting = false;
  function shutdown() {
    if (exiting) {
      process.exit(1); // second signal = force kill
    }
    exiting = true;
    requestQuit();
  }

  process.on('SIGINT', shutdown);
  process.on('SIGTERM', shutdown);

  try {
    await import(entryFilePath);
  } catch (e) {
    console.error('Error running entry point');
    console.error(e);
    process.exit(1);
  }

  while (true) {
    const running = app.pumpAppEvents();

    const events: AppEvent[] = pollEvents();
    for (const event of events) {
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
          console.log('[uzumaki] Hot reload triggered');
          try {
            await import(entryFilePath + '?t=' + Date.now());
          } catch (e) {
            console.error('[uzumaki] Hot reload failed');
            console.error(e);
          }
          break;
      }
    }

    if (!running) break;

    await new Promise((resolve) => setImmediate(resolve));
  }

  app.destroy();
  console.log('Bye');
}
