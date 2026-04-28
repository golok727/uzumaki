import type { ReactNode } from 'react';

import type {
  UzumakiMouseEvent,
  UzumakiKeyboardEvent,
  UzumakiInputEvent,
  UzumakiFocusEvent,
} from '../../events';

interface ElementStyles {
  h?: number | string;
  w?: number | string;
  minH?: number | string;
  minW?: number | string;
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
  fontWeight?: string | number;
  textWrap?: 'wrap' | 'nowrap' | 'anywhere' | 'break-word';
  wordBreak?: 'normal' | 'break-all' | 'keep-all';
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
  outline?: number | string;
  outlineColor?: string;
  outlineOffset?: number | string;
  opacity?: number | string;
  cursor?:
    | 'default'
    | 'auto'
    | 'pointer'
    | 'text'
    | 'wait'
    | 'crosshair'
    | 'move'
    | 'not-allowed'
    | 'grab'
    | 'grabbing'
    | 'help'
    | 'progress'
    | 'ew-resize'
    | 'ns-resize'
    | 'nesw-resize'
    | 'nwse-resize'
    | 'col-resize'
    | 'row-resize'
    | 'all-scroll'
    | 'zoom-in'
    | 'zoom-out';
  display?: 'flex' | 'none' | 'block';
  position?: 'relative' | 'absolute';
  top?: number | string;
  right?: number | string;
  bottom?: number | string;
  left?: number | string;
  translate?: number | [number, number] | { x?: number; y?: number };
  translateX?: number | string;
  translateY?: number | string;
  rotate?: number | string;
  scale?: number | [number, number] | { x?: number; y?: number };
  scaleX?: number | string;
  scaleY?: number | string;
  scrollable?: boolean;
  // if true text inside this view can be selected
  selectable?: boolean;
  visibility?: 'visible' | 'hidden';
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
  onClickCapture?: (ev: UzumakiMouseEvent) => void;
  onMouseDown?: (ev: UzumakiMouseEvent) => void;
  onMouseDownCapture?: (ev: UzumakiMouseEvent) => void;
  onMouseUp?: (ev: UzumakiMouseEvent) => void;
  onMouseUpCapture?: (ev: UzumakiMouseEvent) => void;
  onKeyDown?: (ev: UzumakiKeyboardEvent) => void;
  onKeyDownCapture?: (ev: UzumakiKeyboardEvent) => void;
  onKeyUp?: (ev: UzumakiKeyboardEvent) => void;
  onKeyUpCapture?: (ev: UzumakiKeyboardEvent) => void;
}

export namespace JSX {
  export type Element = ReactNode;

  export interface ElementClass {}

  export interface IntrinsicElements {
    view: ElementAttributes &
      EventProps & {
        children?: any;
        key?: string | number;
        id?: string;
      };
    text: ElementAttributes &
      EventProps & {
        children?: any;
        key?: string | number;
        id?: string;
      };
    p: ElementAttributes &
      EventProps & {
        children?: any;
        key?: string | number;
        id?: string;
      };
    button: ElementAttributes &
      EventProps & {
        children?: any;
        key?: string | number;
        id?: string;
      };
    input: ElementAttributes &
      EventProps & {
        value?: string;
        placeholder?: string;
        disabled?: boolean;
        maxLength?: number;
        multiline?: boolean;
        secure?: boolean;
        onChangeText?: (value: string) => void;
        onInput?: (ev: UzumakiInputEvent) => void;
        onFocus?: (ev: UzumakiFocusEvent) => void;
        onBlur?: (ev: UzumakiFocusEvent) => void;
        children?: any;
        key?: string | number;
        id?: string;
      };
    checkbox: ElementAttributes &
      EventProps & {
        checked?: boolean;
        onChange?: (checked: boolean) => void;
        onInput?: (ev: UzumakiInputEvent) => void;
        onFocus?: (ev: UzumakiFocusEvent) => void;
        onBlur?: (ev: UzumakiFocusEvent) => void;
        children?: any;
        key?: string | number;
        id?: string;
      };
    image: ElementAttributes &
      EventProps & {
        src: string;
        onLoad?: (ev: { src: string }) => void;
        onLoadStart?: (ev: { src: string }) => void;
        onError?: (ev: { src: string; message: string }) => void;
        children?: any;
        key?: string | number;
        id?: string;
      };
  }
}
