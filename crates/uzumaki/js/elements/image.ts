import { STYLE_ATTRIBUTE_NAMES } from '../constants';
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

function isFilePath(source: string) {
  return (
    WINDOWS_DRIVE_PATH.test(source) ||
    source.startsWith('/') ||
    source.startsWith('./') ||
    source.startsWith('../') ||
    source.startsWith(String.raw`\\`)
  );
}

async function readImageSource(source: string) {
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

export class ImageElement extends BaseElement<Record<string, any>> {
  private src: string | undefined;
  private loadGeneration = 0;
  private disposed = false;

  constructor(window: Window, props: Record<string, any>) {
    const id = core.createElement(window.id, 'image');
    super(id, 'image', window);
    this.parseProps(props);
    this.applyStyles();
    this.applyEvents();
    void this.updateSource(props.src);
  }

  private parseProps(props: Record<string, any>): void {
    for (const key in props) {
      if (
        key === 'children' ||
        key === 'key' ||
        key === 'ref' ||
        key === 'src'
      ) {
        continue;
      }
      const value = props[key];
      if (value == null) continue;
      if (isEventProp(key)) {
        const { name, capture } = parseEventProp(key);
        this.eventListeners.set(listenerKey(name, capture), {
          name,
          handler: value,
          capture,
        });
      } else if (STYLE_ATTRIBUTE_NAMES.has(key)) {
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
      return;
    }

    try {
      const data = await readImageSource(src);
      if (!this.isLoadCurrent(generation)) {
        return;
      }
      core.setEncodedImageData(this.windowId, this.id, data);
      core.requestRedraw(this.windowId);
    } catch (error) {
      if (!this.isLoadCurrent(generation)) {
        return;
      }
      core.clearImageData(this.windowId, this.id);
      core.requestRedraw(this.windowId);
      const message = error instanceof Error ? error.message : String(error);
      console.error(`[uzumaki] Failed to load image "${src}": ${message}`);
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
    const newStyles: Record<string, any> = {};
    const newEvents: Map<string, ListenerEntry> = new Map();

    for (const key in newProps) {
      if (
        key === 'children' ||
        key === 'key' ||
        key === 'ref' ||
        key === 'src'
      ) {
        continue;
      }
      const value = newProps[key];
      if (value == null) continue;
      if (isEventProp(key)) {
        const { name, capture } = parseEventProp(key);
        newEvents.set(listenerKey(name, capture), {
          name,
          handler: value,
          capture,
        });
      } else if (STYLE_ATTRIBUTE_NAMES.has(key)) {
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
