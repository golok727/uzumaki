import {
  op_create_window,
  op_request_quit,
  op_request_redraw,
  op_get_root_node_id,
  op_create_element,
  op_create_text_node,
  op_set_image_data,
  op_clear_image_data,
  op_append_child,
  op_insert_before,
  op_remove_child,
  op_set_text,
  op_reset_dom,
  op_set_str_attribute,
  op_set_number_attribute,
  op_set_bool_attribute,
  op_clear_attribute,
  op_get_attribute,
  op_focus_input,
  op_set_rem_base,
  op_get_window_width,
  op_get_window_height,
  op_get_window_title,
  op_get_ancestor_path,
  op_read_clipboard_text,
  op_write_clipboard_text,
} from 'ext:core/ops';

const WINDOWS_DRIVE_PATH = /^[A-Za-z]:[\\/]/;
const URL_SCHEME = /^[A-Za-z][A-Za-z\d+\-.]*:/;

function isFilePath(source) {
  return (
    WINDOWS_DRIVE_PATH.test(source) ||
    source.startsWith('/') ||
    source.startsWith('./') ||
    source.startsWith('../') ||
    source.startsWith('\\\\')
  );
}

async function readImageSource(source) {
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

async function decodeImageSource(source) {
  return readImageSource(source);
}

Object.defineProperty(globalThis, '__uzumaki_ops_dont_touch_this__', {
  value: Object.freeze({
    createWindow: op_create_window,
    requestClose: op_request_quit,
    requestRedraw: op_request_redraw,
    getRootNodeId: op_get_root_node_id,
    createElement: op_create_element,
    createTextNode: op_create_text_node,
    setImageData: op_set_image_data,
    clearImageData: op_clear_image_data,
    appendChild: op_append_child,
    insertBefore: op_insert_before,
    removeChild: op_remove_child,
    setText: op_set_text,
    resetDom: op_reset_dom,
    setStrAttribute: op_set_str_attribute,
    setNumberAttribute: op_set_number_attribute,
    setBoolAttribute: op_set_bool_attribute,
    clearAttribute: op_clear_attribute,
    getAttribute: op_get_attribute,
    focusInput: op_focus_input,
    setRemBase: op_set_rem_base,
    getWindowWidth: op_get_window_width,
    getWindowHeight: op_get_window_height,
    getWindowTitle: op_get_window_title,
    getAncestorPath: op_get_ancestor_path,
    readClipboardText: op_read_clipboard_text,
    writeClipboardText: op_write_clipboard_text,
    decodeImageSource,
  }),
  writable: false,
  configurable: false,
});
