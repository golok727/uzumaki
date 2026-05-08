import type { UzEventMap } from 'ext:uzumaki/events.ts';
import type { Window } from 'ext:uzumaki/window.ts';
import { Element, createNativeElement } from 'ext:uzumaki/elements/element.ts';

export class UzElement<M extends UzEventMap = UzEventMap> extends Element<M> {
  readonly type: string;

  constructor(type: string, window: Window) {
    super(window, createNativeElement(window, type));
    this.type = type;
  }
}
