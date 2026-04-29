import core from '../core';
import type { Window } from '../window';
import { UzElement } from './base';

const WINDOWS_DRIVE_PATH = /^[A-Za-z]:[\\/]/;
const URL_SCHEME = /^[A-Za-z][A-Za-z\d+\-.]*:/;

const inflightBytes = new Map<string, Promise<Uint8Array>>();

function isFilePath(source: string): boolean {
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
    const response = (await fetch(url)) as any; // fixme types
    if (!response.ok) {
      throw new Error(`HTTP ${response.status} while loading ${source}`);
    }
    return new Uint8Array(await response.arrayBuffer());
  }
  return Deno.readFile(source);
}

function loadImageBytes(source: string): Promise<Uint8Array> {
  let promise = inflightBytes.get(source);
  if (promise) return promise;
  promise = fetchImageBytes(source).catch((error) => {
    inflightBytes.delete(source);
    throw error;
  });
  inflightBytes.set(source, promise);
  return promise;
}

export type ImageLoadEvent = { src: string };
export type ImageErrorEvent = { src: string; message: string };

export class UzImageElement extends UzElement {
  /** Called once a load is kicked off (after `src` is set, before bytes resolve). */
  onLoadStart?: (event: ImageLoadEvent) => void;
  /** Called when bytes are decoded and uploaded to the GPU. */
  onLoad?: (event: ImageLoadEvent) => void;
  /** Called when the load fails. If unset, errors log to console. */
  onError?: (event: ImageErrorEvent) => void;

  private _generation = 0;
  private _disposed = false;
  private _src: string | undefined;

  constructor(window: Window) {
    super('image', window);
  }

  get src(): string | undefined {
    return this._src;
  }

  set src(value: string | undefined | null) {
    const next =
      typeof value === 'string' && value.length > 0 ? value : undefined;
    if (next === this._src) return;
    this._src = next;
    this._load(next);
  }

  private _load(src: string | undefined): void {
    const generation = ++this._generation;
    core.clearImageData(this.windowId, this.nodeId);
    core.requestRedraw(this.windowId);

    if (!src) return;

    callHandler(this.onLoadStart, { src });

    if (core.applyCachedImage(this.windowId, this.nodeId, src)) {
      if (!this._isCurrent(generation)) return;
      core.requestRedraw(this.windowId);
      callHandler(this.onLoad, { src });
      return;
    }

    void this._loadAsync(src, generation);
  }

  private async _loadAsync(src: string, generation: number): Promise<void> {
    try {
      const data = await loadImageBytes(src);
      if (!this._isCurrent(generation)) return;
      core.setEncodedImageData(this.windowId, this.nodeId, src, data);
      core.requestRedraw(this.windowId);
      callHandler(this.onLoad, { src });
    } catch (error) {
      if (!this._isCurrent(generation)) return;
      core.clearImageData(this.windowId, this.nodeId);
      core.requestRedraw(this.windowId);
      const message = error instanceof Error ? error.message : String(error);
      if (this.onError) {
        callHandler(this.onError, { src, message });
      } else {
        console.error(`[uzumaki] Failed to load image "${src}": ${message}`);
      }
    }
  }

  private _isCurrent(generation: number): boolean {
    return (
      !this._disposed &&
      !this._window.isDisposed &&
      generation === this._generation
    );
  }

  override destroy(): void {
    this._disposed = true;
    this._generation++;
    super.destroy();
  }
}

function callHandler<T>(
  handler: ((event: T) => void) | undefined,
  event: T,
): void {
  if (!handler) return;
  try {
    handler(event);
  } catch (error) {
    console.error('[uzumaki] image handler threw:', error);
  }
}
