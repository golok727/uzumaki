import core from './core';
import {
  eventManager,
  EVENT_NAME_TO_TYPE,
  type EventName,
  type EventHandler,
} from './events';

const windowsByLabel = new Map<string, Window>();
const windowsById = new Map<number, Window>();

export interface WindowAttributes {
  width: number;
  height: number;
  title: string;
}

export class Window {
  private _id: number;
  private _label: string;
  private _width: number;
  private _height: number;
  private _remBase: number = 16;
  private _disposed: boolean = false;
  private _disposables: (() => void)[] = [];

  constructor(
    label: string,
    {
      width = 800,
      height = 600,
      title = 'uzumaki',
    }: Partial<WindowAttributes> = {},
  ) {
    const existing = windowsByLabel.get(label);
    if (existing) {
      throw new Error(`Window with label ${label} already exists`);
    }

    this._width = width;
    this._height = height;
    this._label = label;
    this._id = core.createWindow({ width, height, title });
    windowsByLabel.set(label, this);
    windowsById.set(this._id, this);
  }

  close() {
    eventManager.clearWindowHandlers(this._id);
    windowsByLabel.delete(this._label);
    windowsById.delete(this._id);
    core.requestClose();
  }

  addDisposable(cb: () => void): void {
    this._disposables.push(cb);
  }

  static _getById(id: number): Window | undefined {
    return windowsById.get(id);
  }

  setSize(width: number, height: number) {
    this._width = width;
    this._height = height;
  }

  get width(): number {
    return core.getWindowWidth(this._id) ?? this._width;
  }

  get height(): number {
    return core.getWindowHeight(this._id) ?? this._height;
  }

  get label(): string {
    return this._label;
  }

  get id(): number {
    return this._id;
  }

  get isDisposed(): boolean {
    return this._disposed;
  }

  get remBase(): number {
    return this._remBase;
  }

  set remBase(value: number) {
    this._remBase = value;
    core.setRemBase(this._id, value);
  }

  on<K extends EventName>(
    eventName: K,
    handler: EventHandler<K>,
    options?: { capture?: boolean },
  ): void {
    const t = EVENT_NAME_TO_TYPE[eventName];
    if (t !== undefined) {
      eventManager.addWindowHandler(
        this._id,
        t,
        handler as Function,
        options?.capture ?? false,
      );
    }
  }

  off<K extends EventName>(
    eventName: K,
    handler: EventHandler<K>,
    options?: { capture?: boolean },
  ): void {
    const t = EVENT_NAME_TO_TYPE[eventName];
    if (t !== undefined) {
      eventManager.removeWindowHandler(
        this._id,
        t,
        handler as Function,
        options?.capture ?? false,
      );
    }
  }
}

/** @internal Called when the native window is destroyed. */
export function disposeWindow(_window: Window) {
  const window = _window as never as {
    id: number;
    label: string;
    _disposed: boolean;
    _disposables: (() => void)[];
  };

  window._disposed = true;
  for (const cb of window._disposables) {
    cb();
  }
  window._disposables = [];
  windowsByLabel.delete(window.label);
  windowsById.delete(window.id);
}
