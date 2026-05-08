import { UzEventMap } from 'ext:uzumaki/events.ts';
import type { Window } from 'ext:uzumaki/window.ts';
import { UzElement } from 'ext:uzumaki/elements/base.ts';

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
