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

interface WindowState {
  width: number;
  height: number;
  title: string;
  visible: boolean;
  resizable: boolean;
  decorations: boolean;
  transparent: boolean;
  maximized: boolean;
  minimized: boolean;
  fullscreen: boolean;
  alwaysOnTop: boolean;
  windowLevel: WindowLevel;
  minWidth?: number;
  minHeight?: number;
  maxWidth?: number;
  maxHeight?: number;
  position?: WindowPosition;
  theme?: WindowTheme;
  active?: boolean;
  contentProtected: boolean;
  enabledButtons: Required<EnabledWindowButtons>;
}

function normalizeEnabledButtons(
  buttons: EnabledWindowButtons = {},
): Required<EnabledWindowButtons> {
  return {
    close: buttons.close ?? true,
    minimize: buttons.minimize ?? true,
    maximize: buttons.maximize ?? true,
  };
}

function createWindowState(attributes: WindowAttributes): WindowState {
  const {
    width = 800,
    height = 600,
    title = 'uzumaki',
    visible = true,
    resizable = true,
    decorations = true,
    transparent = false,
    maximized = false,
    minimized = false,
    fullscreen = false,
    alwaysOnTop = false,
    windowLevel = alwaysOnTop ? 'alwaysOnTop' : 'normal',
    minWidth,
    minHeight,
    maxWidth,
    maxHeight,
    position,
    theme,
    active,
    contentProtected = false,
    enabledButtons,
  } = attributes;

  return {
    width,
    height,
    title,
    visible,
    resizable,
    decorations,
    transparent,
    maximized,
    minimized,
    fullscreen,
    alwaysOnTop: windowLevel === 'alwaysOnTop',
    windowLevel,
    minWidth,
    minHeight,
    maxWidth,
    maxHeight,
    position,
    theme,
    active,
    contentProtected,
    enabledButtons: normalizeEnabledButtons(enabledButtons),
  };
}

export class Window {
  private _id: number;
  private _native: NativeWindow;
  private _label: string;
  private _state: WindowState;
  private _remBase: number = 16;
  private _disposed: boolean = false;
  private _disposables: (() => void)[] = [];

  constructor(label: string, attributes: WindowAttributes = {}) {
    const existing = windowsByLabel.get(label);
    if (existing) {
      throw new Error(`Window with label ${label} already exists`);
    }

    const { rootStyles } = attributes;
    const createOptions = createWindowState(attributes);

    this._state = createOptions;
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

  private setState<K extends keyof WindowState>(
    key: K,
    value: WindowState[K],
  ): void {
    this._state[key] = value;
  }

  private getStateValue<K extends keyof WindowState>(
    key: K,
    value: WindowState[K] | null | undefined,
  ): WindowState[K] {
    return value ?? this._state[key];
  }

  setSize(width: number, height: number): void {
    this.setState('width', width);
    this.setState('height', height);
  }

  setTitle(title: string): void {
    this.setState('title', title);
    this._native.setTitle(title);
  }

  setVisible(visible: boolean): void {
    this.setState('visible', visible);
    this._native.setVisible(visible);
  }

  setTransparent(transparent: boolean): void {
    this.setState('transparent', transparent);
    this._native.setTransparent(transparent);
  }

  setResizable(resizable: boolean): void {
    this.setState('resizable', resizable);
    this._native.setResizable(resizable);
  }

  setDecorations(decorations: boolean): void {
    this.setState('decorations', decorations);
    this._native.setDecorations(decorations);
  }

  setMaximized(maximized: boolean): void {
    this.setState('maximized', maximized);
    this._native.setMaximized(maximized);
  }

  setMinimized(minimized: boolean): void {
    this.setState('minimized', minimized);
    this._native.setMinimized(minimized);
  }

  setFullscreen(fullscreen: boolean): void {
    this.setState('fullscreen', fullscreen);
    this._native.setFullscreen(fullscreen);
  }

  setAlwaysOnTop(alwaysOnTop: boolean): void {
    this.setState('alwaysOnTop', alwaysOnTop);
    this.setState('windowLevel', alwaysOnTop ? 'alwaysOnTop' : 'normal');
    this._native.setAlwaysOnTop(alwaysOnTop);
  }

  setWindowLevel(windowLevel: WindowLevel): void {
    this.setState('windowLevel', windowLevel);
    this.setState('alwaysOnTop', windowLevel === 'alwaysOnTop');
    this._native.setWindowLevel(windowLevel);
  }

  setMinSize(width: number, height: number): void {
    this._native.setMinSize(width, height);
  }

  setMaxSize(width: number, height: number): void {
    this._native.setMaxSize(width, height);
  }

  setPosition(x: number, y: number): void {
    this.setState('position', { x, y });
    this._native.setPosition(x, y);
  }

  setTheme(theme: WindowTheme): void {
    this.setState('theme', theme);
    this._native.setTheme(theme);
  }

  focus(): void {
    this._native.focus();
  }

  setContentProtected(contentProtected: boolean): void {
    this.setState('contentProtected', contentProtected);
    this._native.setContentProtected(contentProtected);
  }

  setEnabledButtons(buttons: EnabledWindowButtons): void {
    const normalized = normalizeEnabledButtons({
      ...this._state.enabledButtons,
      ...buttons,
    });
    this.setState('enabledButtons', normalized);
    this._native.setEnabledButtons(normalized);
  }

  get scaleFactor(): number {
    return this._native.scaleFactor ?? 1;
  }

  get innerWidth(): number {
    return this.getStateValue('width', this._native.innerWidth);
  }

  get innerHeight(): number {
    return this.getStateValue('height', this._native.innerHeight);
  }

  get width(): number {
    return this.innerWidth;
  }

  get height(): number {
    return this.innerHeight;
  }

  get title(): string {
    return this.getStateValue('title', this._native.title);
  }

  get visible(): boolean {
    return this.getStateValue('visible', this._native.visible);
  }

  get transparent(): boolean {
    return this.getStateValue('transparent', this._native.transparent);
  }

  get resizable(): boolean {
    return this.getStateValue('resizable', this._native.resizable);
  }

  get decorated(): boolean {
    return this.getStateValue('decorations', this._native.decorated);
  }

  get maximized(): boolean {
    return this.getStateValue('maximized', this._native.maximized);
  }

  get minimized(): boolean {
    return this.getStateValue('minimized', this._native.minimized);
  }

  get fullscreen(): boolean {
    return this.getStateValue('fullscreen', this._native.fullscreen);
  }

  get alwaysOnTop(): boolean {
    return this.getStateValue('alwaysOnTop', this._native.alwaysOnTop);
  }

  get windowLevel(): WindowLevel {
    return this.getStateValue('windowLevel', this._native.windowLevel);
  }

  get innerSize(): WindowSize | null {
    return this._native.innerSize;
  }

  get outerSize(): WindowSize | null {
    return this._native.outerSize;
  }

  get position(): WindowPosition | null {
    return this._native.position ?? this._state.position ?? null;
  }

  get theme(): WindowTheme | null {
    return this._native.theme ?? this._state.theme ?? null;
  }

  get active(): boolean | null {
    return this._native.active ?? this._state.active ?? null;
  }

  get contentProtected(): boolean {
    return this.getStateValue(
      'contentProtected',
      this._native.contentProtected,
    );
  }

  get enabledButtons(): Required<EnabledWindowButtons> {
    return this._native.enabledButtons ?? this._state.enabledButtons;
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
