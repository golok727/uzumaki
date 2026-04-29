import { Element } from './element';
import { Window } from '../window';

export class UzElement extends Element {
  _elementId: string | null = null;

  constructor(type: string, window: Window) {
    super(type, window);
  }

  get id() {
    return this._elementId;
  }

  set id(value: string | null) {
    this._elementId =
      typeof value === 'string' && value.length > 0 ? value : null;
  }

  destroy(): void {
    super.destroy();
  }
}
