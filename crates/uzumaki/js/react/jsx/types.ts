import type { ReactNode } from 'react';

import type {
  UzMouseEvent,
  UzKeyboardEvent,
  UzInputEvent,
  UzFocusEvent,
} from '../../events';
import { UzNode } from '../../node';
import {
  UzButtonElement,
  UzCheckboxElement,
  UzImageElement,
  UzInputElement,
  UzTextElement,
  UzViewElement,
} from '../../elements';

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
  flexWrap?: 'nowrap' | 'wrap' | 'wrap-reverse';
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
  scroll?: boolean;
  scrollX?: boolean;
  scrollY?: boolean;
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
  extends ElementStyles, HoverStyles, ActiveStyles, FocusStyles {
  focusable?: boolean;
}

interface EventProps<T extends UzNode> {
  onClick?: (ev: UzMouseEvent<T>) => void;
  onClickCapture?: (ev: UzMouseEvent<T>) => void;
  onMouseDown?: (ev: UzMouseEvent<T>) => void;
  onMouseDownCapture?: (ev: UzMouseEvent<T>) => void;
  onMouseUp?: (ev: UzMouseEvent<T>) => void;
  onMouseUpCapture?: (ev: UzMouseEvent<T>) => void;
  onKeyDown?: (ev: UzKeyboardEvent<T>) => void;
  onKeyDownCapture?: (ev: UzKeyboardEvent<T>) => void;
  onKeyUp?: (ev: UzKeyboardEvent<T>) => void;
  onKeyUpCapture?: (ev: UzKeyboardEvent<T>) => void;
}

export namespace JSX {
  export type Element = ReactNode;

  export interface ElementClass {}

  export interface IntrinsicElements {
    view: ElementAttributes &
      EventProps<UzViewElement> & {
        children?: any;
        key?: string | number;
        id?: string;
      };
    text: ElementAttributes &
      EventProps<UzTextElement> & {
        children?: any;
        key?: string | number;
        id?: string;
      };
    button: ElementAttributes &
      EventProps<UzButtonElement> & {
        children?: any;
        key?: string | number;
        id?: string;
      };
    input: ElementAttributes &
      EventProps<UzInputElement> & {
        value?: string;
        placeholder?: string;
        disabled?: boolean;
        maxLength?: number;
        multiline?: boolean;
        secure?: boolean;
        // change = "after commit"
        // input = "while typing"
        // beforeinput = "before typing"
        // todo add after implementing "change" event
        // maybe a find a better name ?
        // onChange?: (ev: UzumakiInputEvent) => void;
        onInput?: (ev: UzInputEvent<UzInputElement>) => void;
        onFocus?: (ev: UzFocusEvent<UzInputElement>) => void;
        onBlur?: (ev: UzFocusEvent<UzInputElement>) => void;
        onValueChange?: (value: string) => void;
        children?: any;
        key?: string | number;
        id?: string;
      };
    checkbox: ElementAttributes &
      EventProps<UzCheckboxElement> & {
        checked?: boolean;
        // onChange?: (ev: UzumakiInputEvent) => void;
        onValueChange?: (value: boolean) => void;
        onInput?: (ev: UzInputEvent<UzCheckboxElement>) => void;
        onFocus?: (ev: UzFocusEvent<UzCheckboxElement>) => void;
        onBlur?: (ev: UzFocusEvent<UzCheckboxElement>) => void;
        children?: any;
        key?: string | number;
        id?: string;
      };
    image: ElementAttributes &
      EventProps<UzImageElement> & {
        src: string;
        // todo type this better
        onLoad?: (ev: { src: string }) => void;
        onLoadStart?: (ev: { src: string }) => void;
        onError?: (ev: { src: string; message: string }) => void;
        children?: any;
        key?: string | number;
        id?: string;
      };
  }
}
