import type { UzEventMap } from '../events';
import type { Window } from '../window';
import { Element, createNativeElement } from './element';

export class UzElement<M extends UzEventMap = UzEventMap> extends Element<M> {
  readonly type: string;

  constructor(type: string, window: Window) {
    super(window, createNativeElement(window, type));
    this.type = type;
  }
}
