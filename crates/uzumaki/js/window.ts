import core, { type CoreWindow } from './core';
import type {
  WindowOptions,
  WindowLevel,
  WindowPosition,
  WindowSize,
  WindowTheme,
} from './types';
import { UzTextNode } from './node';
import { Element } from './elements/element';
import { UzElement } from './elements/base';
import { UzRootElement } from './elements/root';
import { UzViewElement } from './elements/view';
import { UzTextElement } from './elements/text';
import { UzButtonElement } from './elements/button';
import { UzImageElement } from './elements/image';
import { UzInputElement } from './elements/input';
import { UzCheckboxElement } from './elements/checkbox';
import { UzEventTarget, type ListenerOptions } from './event-target';
import {
  buildLifecycleEvent,
  type WindowEventMap,
  type WindowEventName,
  type WindowEventHandler,
} from './events';
import { clearWindowNodes } from './registry';

const windowsByLabel = new Map<string, Window>();
const windowsById = new Map<number, Window>();
const DEFAULT_WINDOW_WIDTH = 800;
const DEFAULT_WINDOW_HEIGHT = 600;
const DEFAULT_WINDOW_TITLE = 'uzumaki';
const DEFAULT_WINDOW_LEVEL: WindowLevel = 'normal';
const DEFAULT_WINDOW_THEME: WindowTheme | null = null;

type ElementConstructor<T extends Element<any> = Element<any>> = new (
  window: Window,
) => T;

const ELEMENT_CONSTRUCTORS = {
  view: UzViewElement,
  text: UzTextElement,
  button: UzButtonElement,
  input: UzInputElement,
  checkbox: UzCheckboxElement,
  image: UzImageElement,
} satisfies Record<string, ElementConstructor>;

export type ElementTagName = keyof typeof ELEMENT_CONSTRUCTORS;
export type ElementForTag<T extends ElementTagName> = InstanceType<
  (typeof ELEMENT_CONSTRUCTORS)[T]
>;

export interface WindowAttributes {
  width: number;
  height: number;
  title: string;
  rootStyles: Record<string, unknown>;
}

export class Window {
  private _id: number;
  private _native: CoreWindow;
  private _label: string;
  private _remBase: number = 16;
  private _disposed: boolean = false;
  private _disposables: (() => void)[] = [];
  private _root: UzRootElement | null = null;
  /** @internal Used by the dispatcher and runtime glue. */
  readonly _emitter: UzEventTarget<WindowEventMap> = new UzEventTarget();

  constructor(label: string, attributes: WindowOptions = {}) {
    const existing = windowsByLabel.get(label);
    if (existing) {
      throw new Error(`Window with label ${label} already exists`);
    }

    const { rootStyles, ...createOptions } = attributes;

    this._label = label;
    this._native = core.createWindow(createOptions);
    this._id = this._native.id;

    if (rootStyles) {
      const root = this.root;
      for (const [key, value] of Object.entries(rootStyles)) {
        if (value != null) root.setAttribute(key, value);
      }
    }

    windowsByLabel.set(label, this);
    windowsById.set(this._id, this);
  }

  close() {
    this._emitter._clear();
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

  set title(title: string) {
    this._native.title = title;
  }

  set decorations(decorations: boolean) {
    this._native.decorations = decorations;
  }

  set visible(visible: boolean) {
    this._native.visible = visible;
  }

  set transparent(transparent: boolean) {
    this._native.transparent = transparent;
  }

  set resizable(resizable: boolean) {
    this._native.resizable = resizable;
  }

  set maximized(maximized: boolean) {
    this._native.maximized = maximized;
  }

  set minimized(minimized: boolean) {
    this._native.minimized = minimized;
  }

  set fullscreen(fullscreen: boolean) {
    this._native.fullscreen = fullscreen;
  }

  set windowLevel(windowLevel: WindowLevel) {
    this._native.windowLevel = windowLevel;
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

  set theme(theme: WindowTheme) {
    this._native.theme = theme;
  }

  focus(): void {
    this._native.focus();
  }

  set contentProtected(contentProtected: boolean) {
    this._native.contentProtected = contentProtected;
  }

  set closable(closable: boolean) {
    this._native.closable = closable;
  }

  set minimizable(minimizable: boolean) {
    this._native.minimizable = minimizable;
  }

  set maximizable(value: boolean) {
    this._native.maximizable = value;
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

  get decorations(): boolean {
    return this._native.decorations ?? true;
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
    return this.windowLevel === 'alwaysOnTop';
  }

  get alwaysOnBottom(): boolean {
    return this.windowLevel === 'alwaysOnBottom';
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

  get closable(): boolean {
    return this._native.closable ?? true;
  }

  get minimizable(): boolean {
    return this._native.minimizable ?? true;
  }

  get maximizable(): boolean {
    return this._native.maximizable ?? true;
  }

  get label(): string {
    return this._label;
  }

  get id(): number {
    return this._id;
  }

  get root(): UzRootElement {
    if (!this._root) {
      this._root = new UzRootElement(this);
    }
    return this._root;
  }

  createElement<T extends ElementTagName>(type: T): ElementForTag<T>;
  createElement(type: string): Element<any>;
  createElement(type: string): Element<any> {
    const Constructor = ELEMENT_CONSTRUCTORS[type as ElementTagName] as
      | ElementConstructor
      | undefined;
    if (Constructor) return new Constructor(this);
    return new UzElement(type, this);
  }

  createTextNode(text: string): UzTextNode {
    return new UzTextNode(this, text);
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

  on<K extends WindowEventName>(
    eventName: K,
    handler: WindowEventHandler<K>,
    options?: ListenerOptions,
  ): void {
    this._emitter.on(eventName, handler, options);
  }

  off<K extends WindowEventName>(
    eventName: K,
    handler: WindowEventHandler<K>,
    options?: ListenerOptions,
  ): void {
    this._emitter.off(eventName, handler, options);
  }

  /** @internal Fire a lifecycle event (load/close/resize). */
  _dispatchLifecycle(
    name: 'load' | 'close' | 'resize',
    payload?: any,
  ): boolean {
    const event = buildLifecycleEvent(name, payload);
    this._emitter.emit(name, event as any);
    return event.defaultPrevented;
  }
}

export function getWindow(label: string): Window | undefined {
  return windowsByLabel.get(label);
}

/** @internal Called when the native window is destroyed. */
export function disposeWindow(_window: Window) {
  const window = _window as never as {
    id: number;
    label: string;
    _disposed: boolean;
    _disposables: (() => void)[];
    _emitter: { _clear(): void };
  };

  window._disposed = true;
  for (const cb of window._disposables) {
    cb();
  }
  window._disposables = [];
  window._emitter._clear();
  clearWindowNodes(window.id);
  windowsByLabel.delete(window.label);
  windowsById.delete(window.id);
}
