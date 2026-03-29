import core from './core';
import { eventManager, type UzumakiEvent } from './events';

const windowsByLabel = new Map<string, Window>();

type EventHandler = (ev: UzumakiEvent) => void;

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
  private _eventId: string;

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
    this._eventId = `__window_${this._id}`;
    windowsByLabel.set(label, this);
  }

  close() {
    eventManager.clearNode(this._eventId);
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

  get eventId(): string {
    return this._eventId;
  }

  get remBase(): number {
    return this._remBase;
  }

  set remBase(value: number) {
    this._remBase = value;
    core.setRemBase(this._id, value);
  }

  on(eventName: string, handler: EventHandler): void {
    eventManager.addHandlerByName(this._eventId, eventName, handler);
  }

  off(eventName: string, handler: EventHandler): void {
    eventManager.removeHandlerByName(this._eventId, eventName, handler);
  }
}
