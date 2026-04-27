import core, { setNativeProp, type NativeWindow } from './core';
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
  rootStyles: Record<string, unknown>;
}

export class Window {
  private _id: number;
  private _native: NativeWindow;
  private _label: string;
  private _title: string;
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
      rootStyles,
    }: Partial<WindowAttributes> = {},
  ) {
    const existing = windowsByLabel.get(label);
    if (existing) {
      throw new Error(`Window with label ${label} already exists`);
    }

    this._width = width;
    this._height = height;
    this._label = label;
    this._title = title;
    this._native = core.createWindow({ width, height, title });
    this._id = this._native.id;
    if (rootStyles) {
      const root = core.getRootNodeId(this._id);
      for (const [key, value] of Object.entries(rootStyles)) {
        if (value != null) setNativeProp(this._id, root, key, value);
      }
    }
    windowsByLabel.set(label, this);
    windowsById.set(this._id, this);
  }

  close() {
    eventManager.clearWindowHandlers(this._id);
    windowsByLabel.delete(this._label);
    windowsById.delete(this._id);
    this._native.close();
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
    return this._native.width ?? this._width;
  }

  get height(): number {
    return this._native.height ?? this._height;
  }

  get title(): string {
    return this._native.title ?? this._title;
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
    return this._native.remBase ?? this._remBase;
  }

  set remBase(value: number) {
    this._remBase = value;
    this._native.remBase = value;
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
