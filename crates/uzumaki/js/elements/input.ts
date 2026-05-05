import type { UzEventMap, UzInputEvent } from '../events';
import type { Window } from '../window';
import { UzElement } from './base';

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
