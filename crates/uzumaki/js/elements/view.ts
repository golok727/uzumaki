import type { Window } from 'ext:uzumaki/window.ts';
import { UzElement } from 'ext:uzumaki/elements/base.ts';

export class UzViewElement extends UzElement {
  constructor(window: Window) {
    super('view', window);
  }
}
