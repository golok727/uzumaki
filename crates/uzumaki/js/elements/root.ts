import type { Window } from 'ext:uzumaki/window.ts';
import { Element, getNativeRootNode } from 'ext:uzumaki/elements/element.ts';

export class UzRootElement extends Element {
  readonly type = '#root';

  constructor(window: Window) {
    super(window, getNativeRootNode(window));
  }
}
