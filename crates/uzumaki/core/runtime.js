import {
  op_create_window,
  op_request_quit,
  op_request_redraw,
  op_get_root_node,
  op_create_core_element_node,
  op_create_core_text_node,
  /** begin image */
  op_set_encoded_image_data,
  op_apply_cached_image,
  op_clear_image_data,
  /** end image */
  op_reset_dom,
  op_focus_element,
  op_get_ancestor_path,
  op_read_clipboard_text,
  op_write_clipboard_text,
  op_get_uz_runtime_version,
} from 'ext:core/ops';

Object.defineProperty(globalThis, '__uzumaki_ops_dont_touch_this__', {
  value: Object.freeze({
    createWindow: op_create_window,
    requestQuit: op_request_quit,
    requestRedraw: op_request_redraw,
    getRootNode: op_get_root_node,
    createCoreElementNode: op_create_core_element_node,
    createCoreTextNode: op_create_core_text_node,
    setEncodedImageData: op_set_encoded_image_data,
    applyCachedImage: op_apply_cached_image,
    clearImageData: op_clear_image_data,
    resetDom: op_reset_dom,
    focusElement: op_focus_element,
    getAncestorPath: op_get_ancestor_path,
    readClipboardText: op_read_clipboard_text,
    writeClipboardText: op_write_clipboard_text,
  }),
  writable: false,
  configurable: false,
});

export const RUNTIME_VERSION = op_get_uz_runtime_version();
