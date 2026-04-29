import type { Window } from '../window';
import { Element, createNativeElement } from './element';

export class UzElement extends Element {
  readonly type: string;
  private _elementId: string | null = null;

  constructor(type: string, window: Window) {
    super(window, createNativeElement(window, type));
    this.type = type;
  }

  get id(): string | null {
    return this._elementId;
  }

  set id(value: string | null) {
    this._elementId =
      typeof value === 'string' && value.length > 0 ? value : null;
  }
}
