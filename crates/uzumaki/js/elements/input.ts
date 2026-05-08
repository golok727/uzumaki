import type { UzEventMap, UzInputEvent } from 'ext:uzumaki/events.ts';
import type { Window } from 'ext:uzumaki/window.ts';
import { UzElement } from 'ext:uzumaki/elements/base.ts';

export interface InputChangeEvent {
  readonly value: string;
}

export interface InputEventMap extends UzEventMap {
  /**
   * Fires before a pending text change is applied. Call `preventDefault()` to
   * cancel the change (e.g. for input filtering). Native may not emit this for
   * every modification source yet.
   */
  beforeinput: UzInputEvent;
  valuechange: string;
}

export class UzInputElement extends UzElement<InputEventMap> {
  constructor(window: Window) {
    super('input', window);

    this.on('input', () => {
      if (this._emitter._listenerCount('valuechange') > 0) {
        const value = this.value;
        this._emitter.emit('valuechange', value);
      }
    });
  }

  get value(): string {
    return String(this.getAttribute('value') ?? '');
  }
}
