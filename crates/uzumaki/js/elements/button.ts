import type { Window } from '../window';
import { UzElement } from './base';

export class UzButtonElement extends UzElement {
  constructor(window: Window) {
    super('button', window);
  }
}
