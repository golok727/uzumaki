import { UzEventMap } from '../events';
import type { Window } from '../window';
import { UzElement } from './base';

export interface CheckboxEventHandlerMap extends UzEventMap {
  valuechange: boolean;
}

export class UzCheckboxElement extends UzElement<CheckboxEventHandlerMap> {
  constructor(window: Window) {
    super('checkbox', window);

    this.on('input', () => {
      if (this._emitter._listenerCount('valuechange') > 0) {
        const value = this.checked;
        this._emitter.emit('valuechange', value);
      }
    });
  }

  get checked(): boolean {
    const checked = this.getAttribute('checked');
    return checked === true || checked === 'true';
  }
}
