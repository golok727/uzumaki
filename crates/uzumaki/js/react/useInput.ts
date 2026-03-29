import { useRef, useEffect } from 'react';
import core from '../core';

export interface InputHandle {
  readonly value: string;
  set(value: string): void;
  focus(): void;
  readonly __handle: true;
  __nodeId: any;
  __windowId: number | null;
  __onChange: ((value: string) => void) | undefined;
}

export interface InputHandleOptions {
  onChange?: (value: string) => void;
}

export function createInputHandle(
  initialValue?: string,
  options?: InputHandleOptions,
): InputHandle {
  const handle: InputHandle = {
    __handle: true as const,
    __nodeId: null,
    __windowId: null,
    __onChange: options?.onChange,

    get value(): string {
      if (handle.__windowId != null && handle.__nodeId != null) {
        return core.getInputValue(handle.__windowId, handle.__nodeId);
      }
      return initialValue ?? '';
    },

    set(value: string): void {
      if (handle.__windowId != null && handle.__nodeId != null) {
        core.setInputValue(handle.__windowId, handle.__nodeId, value);
        core.requestRedraw(handle.__windowId);
      }
    },

    focus(): void {
      if (handle.__windowId != null && handle.__nodeId != null) {
        core.focusInput(handle.__windowId, handle.__nodeId);
        core.requestRedraw(handle.__windowId);
      }
    },
  };

  (handle as any).__initialValue = initialValue ?? '';

  return handle;
}

export function useInput(
  initialValue?: string,
  options?: InputHandleOptions,
): InputHandle {
  const ref = useRef<InputHandle | null>(null);
  if (ref.current === null) {
    ref.current = createInputHandle(initialValue, options);
  }
  useEffect(() => {
    if (ref.current) {
      ref.current.__onChange = options?.onChange;
    }
  });
  return ref.current;
}
