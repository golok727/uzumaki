import core from '../core';
import { ListenerEntry } from '../types';
import {
  assignNativeStyle,
  isEventProp,
  listenerKey,
  parseEventProp,
} from '../utils';
import { Window } from '../window';
import { BaseElement } from './base';

const WINDOWS_DRIVE_PATH = /^[A-Za-z]:[\\/]/;
const URL_SCHEME = /^[A-Za-z][A-Za-z\d+\-.]*:/;

const LIFECYCLE_PROPS = new Set([
  'children',
  'key',
  'ref',
  'src',
  'onLoad',
  'onLoadStart',
  'onError',
]);

type ImageStatus = 'idle' | 'loading' | 'loaded' | 'error';

export interface ImageLoadEvent {
  src: string;
  width?: number;
  height?: number;
}

export interface ImageErrorEvent {
  src: string;
  message: string;
}

function isFilePath(source: string) {
  return (
    WINDOWS_DRIVE_PATH.test(source) ||
    source.startsWith('/') ||
    source.startsWith('./') ||
    source.startsWith('../') ||
    source.startsWith(String.raw`\\`)
  );
}

async function fetchImageBytes(source: string): Promise<Uint8Array> {
  if (isFilePath(source)) {
    return Deno.readFile(source);
  }

  if (URL_SCHEME.test(source)) {
    const url = new URL(source);
    if (url.protocol === 'file:') {
      return Deno.readFile(url);
    }
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(`HTTP ${response.status} while loading ${source}`);
    }
    return new Uint8Array(await response.arrayBuffer());
  }

  return Deno.readFile(source);
}

const inflightBytes = new Map<string, Promise<Uint8Array>>();

function loadImageBytes(source: string): Promise<Uint8Array> {
  let p = inflightBytes.get(source);
  if (p) return p;
  p = fetchImageBytes(source).catch((error) => {
    inflightBytes.delete(source);
    throw error;
  });
  inflightBytes.set(source, p);
  return p;
}

export class ImageElement extends BaseElement<Record<string, any>> {
  private src: string | undefined;
  private loadGeneration = 0;
  private disposed = false;
  private status: ImageStatus = 'idle';
  private onLoad: ((ev: ImageLoadEvent) => void) | undefined;
  private onLoadStart: ((ev: { src: string }) => void) | undefined;
  private onError: ((ev: ImageErrorEvent) => void) | undefined;

  constructor(window: Window, props: Record<string, any>) {
    const id = core.createElement(window.id, 'image');
    super(id, 'image', window);
    this.parseProps(props);
    this.applyStyles();
    this.applyEvents();
    void this.updateSource(props.src);
  }

  private parseProps(props: Record<string, any>): void {
    this.onLoad = typeof props.onLoad === 'function' ? props.onLoad : undefined;
    this.onLoadStart =
      typeof props.onLoadStart === 'function' ? props.onLoadStart : undefined;
    this.onError =
      typeof props.onError === 'function' ? props.onError : undefined;

    for (const key in props) {
      if (LIFECYCLE_PROPS.has(key)) continue;
      const value = props[key];
      if (value == null) continue;
      if (isEventProp(key)) {
        const { name, capture } = parseEventProp(key);
        this.eventListeners.set(listenerKey(name, capture), {
          name,
          handler: value,
          capture,
        });
      } else {
        assignNativeStyle(this.styles, key, value);
      }
    }
  }

  private async updateSource(src: string | undefined): Promise<void> {
    if (typeof src !== 'string' || src.length === 0) {
      src = undefined;
    }

    if (src === this.src) return;

    this.src = src;
    const generation = ++this.loadGeneration;
    core.clearImageData(this.windowId, this.id);
    core.requestRedraw(this.windowId);

    if (!src) {
      this.status = 'idle';
      return;
    }

    this.status = 'loading';
    try {
      this.onLoadStart?.({ src });
    } catch (error) {
      console.error('[uzumaki] onLoadStart handler threw:', error);
    }

    if (core.applyCachedImage(this.windowId, this.id, src)) {
      if (!this.isLoadCurrent(generation)) return;
      core.requestRedraw(this.windowId);
      this.status = 'loaded';
      try {
        this.onLoad?.({ src });
      } catch (error) {
        console.error('[uzumaki] onLoad handler threw:', error);
      }
      return;
    }

    try {
      const data = await loadImageBytes(src);
      if (!this.isLoadCurrent(generation)) return;
      core.setEncodedImageData(this.windowId, this.id, src, data);
      core.requestRedraw(this.windowId);
      this.status = 'loaded';
      try {
        this.onLoad?.({ src });
      } catch (error) {
        console.error('[uzumaki] onLoad handler threw:', error);
      }
    } catch (error) {
      if (!this.isLoadCurrent(generation)) return;
      core.clearImageData(this.windowId, this.id);
      core.requestRedraw(this.windowId);
      this.status = 'error';
      const message = error instanceof Error ? error.message : String(error);
      if (this.onError) {
        try {
          this.onError({ src, message });
        } catch (error) {
          console.error('[uzumaki] onError handler threw:', error);
        }
      } else {
        console.error(`[uzumaki] Failed to load image "${src}": ${message}`);
      }
    }
  }

  private isLoadCurrent(generation: number): boolean {
    return (
      !this.disposed &&
      !this.window.isDisposed &&
      generation === this.loadGeneration
    );
  }

  commitUpdate(
    newProps: Record<string, any>,
    _oldProps: Record<string, any>,
  ): void {
    this.onLoad =
      typeof newProps.onLoad === 'function' ? newProps.onLoad : undefined;
    this.onLoadStart =
      typeof newProps.onLoadStart === 'function'
        ? newProps.onLoadStart
        : undefined;
    this.onError =
      typeof newProps.onError === 'function' ? newProps.onError : undefined;

    const newStyles: Record<string, any> = {};
    const newEvents: Map<string, ListenerEntry> = new Map();

    for (const key in newProps) {
      if (LIFECYCLE_PROPS.has(key)) continue;
      const value = newProps[key];
      if (value == null) continue;
      if (isEventProp(key)) {
        const { name, capture } = parseEventProp(key);
        newEvents.set(listenerKey(name, capture), {
          name,
          handler: value,
          capture,
        });
      } else {
        assignNativeStyle(newStyles, key, value);
      }
    }

    this.updateStyles(newStyles);
    this.updateEvents(newEvents);
    void this.updateSource(newProps.src);
  }

  override destroy(): void {
    this.disposed = true;
    this.loadGeneration += 1;
    super.destroy();
  }
}
