import {
  CHECKBOX_ATTR_NAMES,
  INPUT_ATTR_NAMES,
  STYLE_ATTRIBUTE_NAMES,
} from './constants';

export function splitVariantProp(key: string): {
  prefix: string;
  name: string;
} {
  const idx = key.indexOf(':');
  if (idx === -1) return { prefix: '', name: key };
  return { prefix: key.slice(0, idx + 1), name: key.slice(idx + 1) };
}

export function readPair(value: any): [number, number] {
  if (Array.isArray(value)) {
    const x = Number(value[0] ?? 0);
    const y = Number(value[1] ?? x);
    return [x, y];
  }
  if (typeof value === 'object' && value !== null) {
    const x = Number(value.x ?? 0);
    const y = Number(value.y ?? 0);
    return [x, y];
  }
  const n = Number(value ?? 0);
  return [n, n];
}

export function assignNativeStyle(
  styles: Record<string, any>,
  key: string,
  value: any,
): void {
  const { prefix, name } = splitVariantProp(key);
  if (name === 'translate') {
    const [x, y] = readPair(value);
    styles[`${prefix}translateX`] = x;
    styles[`${prefix}translateY`] = y;
    return;
  }
  if (name === 'scale') {
    const [x, y] = readPair(value);
    styles[`${prefix}scaleX`] = x;
    styles[`${prefix}scaleY`] = y;
    return;
  }
  styles[key] = value;
}

export function isEventProp(key: string): boolean {
  const thirdCharacterCode = key.codePointAt(2);
  return (
    key.length >= 3 &&
    key[0] === 'o' &&
    key[1] === 'n' &&
    thirdCharacterCode !== undefined &&
    thirdCharacterCode >= 65 &&
    thirdCharacterCode <= 90
  );
}

export function listenerKey(name: string, capture: boolean): string {
  return `${name}:${capture ? 'capture' : 'bubble'}`;
}

export function parseEventProp(key: string): {
  name: string;
  capture: boolean;
} {
  const raw = key.slice(2); // strip "on"
  if (raw.endsWith('Capture')) {
    return { name: raw.slice(0, -7).toLowerCase(), capture: true };
  }
  return { name: raw.toLowerCase(), capture: false };
}

export function isNativeAttribute(key: string): boolean {
  return (
    STYLE_ATTRIBUTE_NAMES.has(key) ||
    INPUT_ATTR_NAMES.has(key) ||
    CHECKBOX_ATTR_NAMES.has(key)
  );
}
