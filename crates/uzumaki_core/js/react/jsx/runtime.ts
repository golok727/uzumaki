import type { ReactNode } from 'react';
import { createElement } from 'react';

interface StyleProps {
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
  flex?: string | number;
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
  'hover:bg'?: string;
  'hover:color'?: string;
  'hover:opacity'?: number | string;
  'hover:borderColor'?: string;
  'active:bg'?: string;
  'active:color'?: string;
}

interface EventProps {
  onClick?: (payload?: unknown) => void;
  onMouseDown?: (payload?: unknown) => void;
  onMouseUp?: (payload?: unknown) => void;
}

export namespace JSX {
  export type Element = ReactNode;

  export interface ElementClass {}

  export interface IntrinsicElements {
    view: StyleProps & EventProps & {
      children?: any;
      key?: string | number;
    };
    text: StyleProps & EventProps & {
      children?: any;
      key?: string | number;
    };
    p: StyleProps & EventProps & {
      children?: any;
      key?: string | number;
    };
    button: StyleProps & EventProps & {
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
