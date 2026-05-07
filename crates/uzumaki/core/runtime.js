import { primordials } from 'ext:core/mod.js';

import {
  op_create_window,
  op_request_quit,
  op_request_redraw,
  op_get_root_node,
  op_create_element_node,
  op_create_text_node,
  /** begin image */
  op_set_encoded_image_data,
  op_apply_cached_image,
  op_clear_image_data,
  /** end image */
  op_focus_element,
  op_get_ancestor_path,
  op_read_clipboard_text,
  op_write_clipboard_text,
  op_get_uz_runtime_version,
  AppPath,
} from 'ext:core/ops';

const { ObjectDefineProperty } = primordials;

const appEventSubscribers = [];

function onAppEvent(handler) {
  if (typeof handler !== 'function') {
    throw new TypeError('onAppEvent expects a function');
  }
  appEventSubscribers.push(handler);
  return function dispose() {
    const idx = appEventSubscribers.indexOf(handler);
    if (idx !== -1) appEventSubscribers.splice(idx, 1);
  };
}

// todo find a better way to do this
let appPath;
ObjectDefineProperty(globalThis, 'Uz', {
  value: {
    get path() {
      if (appPath === undefined) appPath = new AppPath();
      return appPath;
    },
  },
  writable: false,
  configurable: false,
});

ObjectDefineProperty(globalThis, '__uzumaki_ops_dont_touch_this__', {
  value: Object.freeze({
    createWindow: op_create_window,
    requestQuit: op_request_quit,
    requestRedraw: op_request_redraw,
    getRootNode: op_get_root_node,
    createElementNode: op_create_element_node,
    createTextNode: op_create_text_node,
    setEncodedImageData: op_set_encoded_image_data,
    applyCachedImage: op_apply_cached_image,
    clearImageData: op_clear_image_data,
    // todo dispatch focus event
    focusElement: op_focus_element,
    getAncestorPath: op_get_ancestor_path,
    readClipboardText: op_read_clipboard_text,
    writeClipboardText: op_write_clipboard_text,
    onAppEvent,
  }),
  writable: false,
  configurable: false,
});

// Native side looks up this exact name. Runtime owns it; user code should
// register through `__uzumaki.onAppEvent` instead of overwriting it.
globalThis.__uzumaki_on_app_event__ = function (event) {
  let prevented = false;
  const ctx = {
    preventDefault() {
      prevented = true;
    },
    get defaultPrevented() {
      return prevented;
    },
  };
  // copy so a subscriber unsubscribing during dispatch doesn't shift iteration
  const subs = appEventSubscribers.slice();
  for (let i = 0; i < subs.length; i++) {
    try {
      subs[i](event, ctx);
    } catch (err) {
      console.error('[uzumaki] app event subscriber threw:', err);
    }
  }
  return prevented;
};

export const RUNTIME_VERSION = op_get_uz_runtime_version();
