import type { Window } from '../window';
import { UzElement } from './base';

export class UzTextElement extends UzElement {
  constructor(window: Window) {
    super('text', window);
  }
}
