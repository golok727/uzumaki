import type { Window } from 'ext:uzumaki/window.ts';
import { UzElement } from 'ext:uzumaki/elements/base.ts';

export class UzTextElement extends UzElement {
  constructor(window: Window) {
    super('text', window);
  }
}
