import core, { type CoreCreateWindowOptions } from './core';

const windowsByLabel = new Map<string, Window>();

export function getWindow(label: string) {
  return windowsByLabel.get(label);
}

export type WindowCreateOptions = Omit<CoreCreateWindowOptions, 'label'>;

export class Window {
  private __id: number;
  private _label: string;
  private _title: string;
  private _width: number;
  private _height: number;

  constructor(label: string, options: WindowCreateOptions) {
    this.__id = core.createWindow({ label, ...options });
    this._label = label;
    this._title = options.title;
    this._width = options.width;
    this._height = options.height;

    windowsByLabel.set(label, this);
  }

  get label() {
    return this._label;
  }

  get id() {
    return this.__id;
  }

  get width() {
    return this._width;
  }

  get height() {
    return this._height;
  }

  get title() {
    return this._title;
  }

  set height(val: number) {
    this._height = val;
    // todo core set height
  }

  set width(val: number) {
    this._width = val;
    // todo core set width
  }

  set title(val: string) {
    this._title = val;
    // todo core set title
  }
}
