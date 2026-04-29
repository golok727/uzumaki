import type { Window } from '../window';
import { Element, createNativeElement } from './element';

export class UzElement extends Element {
  readonly type: string;

  constructor(type: string, window: Window) {
    super(window, createNativeElement(window, type));
    this.type = type;
  }
}
