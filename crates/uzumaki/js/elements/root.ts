import type { Window } from '../window';
import { Element, getNativeRootNode } from './element';

export class UzRootElement extends Element {
  readonly type = '#root';

  constructor(window: Window) {
    super(window, getNativeRootNode(window));
  }
}
