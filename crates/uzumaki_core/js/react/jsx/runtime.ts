import type { ReactNode } from 'react';
import { createElement } from 'react';
import type { UzumakiMouseEvent, UzumakiKeyboardEvent, UzumakiInputEvent, UzumakiFocusEvent } from '../../events';

interface ElementStyles {
  h?: number | string;
  w?: number | string;
  p?: number | string;
  px?: number | string;
  py?: number | string;
  pt?: number | string;
  pb?: number | string;
  pl?: number | string;
  pr?: number | string;
  m?: number | string;
  mx?: number | string;
  my?: number | string;
  mt?: number | string;
  mb?: number | string;
  ml?: number | string;
  mr?: number | string;
  flex?: string | number | true;
  flexDir?: 'row' | 'col' | 'column';
  flexGrow?: number | string;
  flexShrink?: number | string;
  items?: 'start' | 'end' | 'center' | 'stretch' | 'baseline';
  justify?: 'start' | 'end' | 'center' | 'between' | 'around' | 'evenly';
  gap?: number | string;
  bg?: string;
  color?: string;
  fontSize?: number | string;
  fontWeight?: string;
  rounded?: number | string;
  roundedTL?: number | string;
  roundedTR?: number | string;
  roundedBR?: number | string;
  roundedBL?: number | string;
  border?: number | string;
  borderTop?: number | string;
  borderRight?: number | string;
  borderBottom?: number | string;
  borderLeft?: number | string;
  borderColor?: string;
  opacity?: number | string;
  cursor?: string;
  display?: 'flex' | 'none' | 'block';
}

type PrefixedStyles<Prefix extends string> = {
  [K in keyof ElementStyles as `${Prefix}:${string & K}`]?: ElementStyles[K];
};

type HoverStyles = PrefixedStyles<'hover'>;
type ActiveStyles = PrefixedStyles<'active'>;
type FocusStyles = PrefixedStyles<'focus'>;

interface ElementAttributes
  extends ElementStyles, HoverStyles, ActiveStyles, FocusStyles {}

interface EventProps {
  onClick?: (ev: UzumakiMouseEvent) => void;
  onMouseDown?: (ev: UzumakiMouseEvent) => void;
  onMouseUp?: (ev: UzumakiMouseEvent) => void;
  onKeyDown?: (ev: UzumakiKeyboardEvent) => void;
  onKeyUp?: (ev: UzumakiKeyboardEvent) => void;
}

export namespace JSX {
  export type Element = ReactNode;

  export interface ElementClass {}

  export interface IntrinsicElements {
    view: ElementAttributes &
      EventProps & {
        children?: any;
        key?: string | number;
      };
    text: ElementAttributes &
      EventProps & {
        children?: any;
        key?: string | number;
      };
    p: ElementAttributes &
      EventProps & {
        children?: any;
        key?: string | number;
      };
    button: ElementAttributes &
      EventProps & {
        children?: any;
        key?: string | number;
      };
    input: ElementAttributes &
      EventProps & {
        value?: string;
        placeholder?: string;
        disabled?: boolean;
        maxLength?: number;
        multiline?: boolean;
        secure?: boolean;
        onInput?: (ev: UzumakiInputEvent) => void;
        onFocus?: (ev: UzumakiFocusEvent) => void;
        onBlur?: (ev: UzumakiFocusEvent) => void;
        children?: any;
        key?: string | number;
      };
  }
}

export function jsx(
  type: string,
  props: Record<string, any>,
  key?: string,
): JSX.Element {
  if (key !== undefined) {
    return createElement(type, { ...props, key });
  }
  return createElement(type, props);
}

export function jsxs(
  type: string,
  props: Record<string, any>,
  key?: string,
): JSX.Element {
  if (key !== undefined) {
    return createElement(type, { ...props, key });
  }
  return createElement(type, props);
}

export const jsxDEV = jsx;

export const Fragment = Symbol('Fragment');
