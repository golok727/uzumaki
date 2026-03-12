import { fileURLToPath } from 'bun';
import { Application, createWindow } from './bindings';

export interface WindowAttributes {
  width: number;
  height: number;
  title: string;
}

export class Window {
  private _width: number;
  private _height: number;
  private _label: string;

  constructor(
    label: string,
    {
      width = 800,
      height = 600,
      title = 'uzumaki',
    }: Partial<WindowAttributes> = {},
  ) {
    this._width = width;
    this._height = height;
    this._label = label;

    createWindow({ width, height, label, title });
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
export { dispatchEvent } from './react/reconciler';

export function runApp({
  entryFilePath,
  title = 'uzumaki',
}: {
  entryFilePath: string;
  title?: string;
}) {
  let app = new Application();

  process.on('SIGINT', () => {});
  process.on('SIGTERM', () => {});

  const worker = new Worker(fileURLToPath(new URL('./main', import.meta.url)), {
    env: { ...process.env, entryPoint: entryFilePath },
  });
  worker.onerror = (e) => {
    console.error(e);
    process.exit(1);
  };

  app.onInit(() => {});

  // Route DOM events from Rust to the worker
  app.onDomEvent((label: string, nodeId: string, eventType: string) => {
    worker.postMessage({ type: 'domEvent', label, nodeId, eventType });
  });

  app.onWindowEvent(() => {});

  app.run();

  worker.terminate();

  console.log('Reach here');
}
