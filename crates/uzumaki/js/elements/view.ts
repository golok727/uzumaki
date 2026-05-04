import type { Window } from '../window';
import { UzElement } from './base';

export class UzViewElement extends UzElement {
  constructor(window: Window) {
    super('view', window);
  }
}
