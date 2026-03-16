import { Application, createWindow, pollEvents, resetDom } from './bindings';
import { dispatchEvent } from './react/reconciler';
import { requestQuit } from './bindings';

export interface WindowAttributes {
  width: number;
  height: number;
  title: string;
}

const windowRegistry = new Map<string, Window>();

export class Window {
  private _width!: number;
  private _height!: number;
  private _label!: string;

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

    createWindow({ width, height, label, title });
    windowRegistry.set(label, this);
  }

  close() {}

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
}

export { render } from './react';

interface AppEvent {
  type: string;
  windowLabel?: string;
  nodeId?: string;
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
        case 'click':
        case 'mouseDown':
        case 'mouseUp':
          if (event.nodeId) {
            dispatchEvent(event.nodeId, event.type);
          }
          break;
        case 'keyDown':
        case 'keyUp':
          // TODO: dispatch to focused element or global handler
          break;
        case 'resize':
          // TODO: dispatch resize if needed
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
