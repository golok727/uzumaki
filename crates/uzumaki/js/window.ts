import core, {
  setNativeProp,
  type EnabledWindowButtons,
  type NativeWindow,
  type WindowLevel,
  type WindowPosition,
  type WindowSize,
  type WindowTheme,
} from './core';
import {
  eventManager,
  EVENT_NAME_TO_TYPE,
  type EventName,
  type EventHandler,
} from './events';

export type {
  EnabledWindowButtons,
  WindowLevel,
  WindowPosition,
  WindowSize,
  WindowTheme,
} from './core';

const windowsByLabel = new Map<string, Window>();
const windowsById = new Map<number, Window>();
const DEFAULT_WINDOW_WIDTH = 800;
const DEFAULT_WINDOW_HEIGHT = 600;
const DEFAULT_WINDOW_TITLE = 'uzumaki';
const DEFAULT_WINDOW_LEVEL: WindowLevel = 'normal';
const DEFAULT_WINDOW_THEME: WindowTheme | null = null;
const DEFAULT_ENABLED_BUTTONS: Required<EnabledWindowButtons> = {
  close: true,
  minimize: true,
  maximize: true,
};

export interface WindowAttributes {
  width?: number;
  height?: number;
  title?: string;
  visible?: boolean;
  resizable?: boolean;
  decorations?: boolean;
  transparent?: boolean;
  maximized?: boolean;
  minimized?: boolean;
  fullscreen?: boolean;
  alwaysOnTop?: boolean;
  windowLevel?: WindowLevel;
  minWidth?: number;
  minHeight?: number;
  maxWidth?: number;
  maxHeight?: number;
  position?: WindowPosition;
  theme?: WindowTheme;
  active?: boolean;
  contentProtected?: boolean;
  enabledButtons?: EnabledWindowButtons;
  rootStyles?: Record<string, unknown>;
}

function normalizeEnabledButtons(
  buttons: EnabledWindowButtons | null | undefined,
): Required<EnabledWindowButtons> {
  return {
    close: buttons?.close ?? DEFAULT_ENABLED_BUTTONS.close,
    minimize: buttons?.minimize ?? DEFAULT_ENABLED_BUTTONS.minimize,
    maximize: buttons?.maximize ?? DEFAULT_ENABLED_BUTTONS.maximize,
  };
}

export class Window {
  private _id: number;
  private _native: NativeWindow;
  private _label: string;
  private _remBase: number = 16;
  private _disposed: boolean = false;
  private _disposables: (() => void)[] = [];

  constructor(label: string, attributes: WindowAttributes = {}) {
    const existing = windowsByLabel.get(label);
    if (existing) {
      throw new Error(`Window with label ${label} already exists`);
    }

    const { rootStyles, ...createOptions } = attributes;

    this._label = label;
    this._native = core.createWindow(createOptions);
    this._id = this._native.id;

    if (rootStyles) {
      const root = core.getRootNodeId(this._id);
      for (const [key, value] of Object.entries(rootStyles)) {
        if (value != null) {
          setNativeProp(this._id, root, key, value);
        }
      }
    }

    windowsByLabel.set(label, this);
    windowsById.set(this._id, this);
  }

  close(): void {
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

  setTitle(title: string): void {
    this._native.setTitle(title);
  }

  setVisible(visible: boolean): void {
    this._native.setVisible(visible);
  }

  setTransparent(transparent: boolean): void {
    this._native.setTransparent(transparent);
  }

  setResizable(resizable: boolean): void {
    this._native.setResizable(resizable);
  }

  setDecorations(decorations: boolean): void {
    this._native.setDecorations(decorations);
  }

  setMaximized(maximized: boolean): void {
    this._native.setMaximized(maximized);
  }

  setMinimized(minimized: boolean): void {
    this._native.setMinimized(minimized);
  }

  setFullscreen(fullscreen: boolean): void {
    this._native.setFullscreen(fullscreen);
  }

  setAlwaysOnTop(alwaysOnTop: boolean): void {
    this._native.setAlwaysOnTop(alwaysOnTop);
  }

  setWindowLevel(windowLevel: WindowLevel): void {
    this._native.setWindowLevel(windowLevel);
  }

  setMinSize(width: number, height: number): void {
    this._native.setMinSize(width, height);
  }

  setMaxSize(width: number, height: number): void {
    this._native.setMaxSize(width, height);
  }

  setPosition(x: number, y: number): void {
    this._native.setPosition(x, y);
  }

  setTheme(theme: WindowTheme): void {
    this._native.setTheme(theme);
  }

  focus(): void {
    this._native.focus();
  }

  setContentProtected(contentProtected: boolean): void {
    this._native.setContentProtected(contentProtected);
  }

  setEnabledButtons(buttons: EnabledWindowButtons): void {
    this._native.setEnabledButtons(buttons);
  }

  get scaleFactor(): number {
    return this._native.scaleFactor ?? 1;
  }

  get innerWidth(): number {
    return this._native.innerWidth ?? DEFAULT_WINDOW_WIDTH;
  }

  get innerHeight(): number {
    return this._native.innerHeight ?? DEFAULT_WINDOW_HEIGHT;
  }

  get title(): string {
    return this._native.title ?? DEFAULT_WINDOW_TITLE;
  }

  get visible(): boolean {
    return this._native.visible ?? true;
  }

  get transparent(): boolean {
    return this._native.transparent ?? false;
  }

  get resizable(): boolean {
    return this._native.resizable ?? true;
  }

  get decorated(): boolean {
    return this._native.decorated ?? true;
  }

  get maximized(): boolean {
    return this._native.maximized ?? false;
  }

  get minimized(): boolean {
    return this._native.minimized ?? false;
  }

  get fullscreen(): boolean {
    return this._native.fullscreen ?? false;
  }

  get alwaysOnTop(): boolean {
    return this._native.alwaysOnTop ?? false;
  }

  get windowLevel(): WindowLevel {
    return this._native.windowLevel ?? DEFAULT_WINDOW_LEVEL;
  }

  get innerSize(): WindowSize | null {
    return this._native.innerSize;
  }

  get outerSize(): WindowSize | null {
    return this._native.outerSize;
  }

  get position(): WindowPosition | null {
    return this._native.position;
  }

  get theme(): WindowTheme | null {
    return this._native.theme ?? DEFAULT_WINDOW_THEME;
  }

  get active(): boolean | null {
    return this._native.active;
  }

  get contentProtected(): boolean {
    return this._native.contentProtected ?? false;
  }

  get enabledButtons(): Required<EnabledWindowButtons> {
    return normalizeEnabledButtons(this._native.enabledButtons);
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

export function getWindow(label: string): Window | undefined {
  return windowsByLabel.get(label);
}

/** @internal Called when the native window is destroyed. */
export function disposeWindow(_window: Window): void {
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
