pub mod runtime;

pub mod app;
pub mod clipboard;
pub mod cursor;
pub mod element;
pub mod event_dispatch;
pub mod geometry;
pub mod gpu;
pub mod input;
pub mod interactivity;
pub mod ops;
pub mod selection;
pub mod style;
pub mod text;
pub mod ui;
pub mod window;

use deno_core::*;

pub use crate::app::Application;

pub(crate) mod parse;
pub(crate) mod prop_keys;

pub use deno_core;
pub use deno_runtime;
pub use rustls;

pub static TS_VERSION: &str = "5.9.2";

#[cfg(feature = "snapshot")]
pub fn create_snapshot(
    snapshot_path: std::path::PathBuf,
    snapshot_options: deno_runtime::ops::bootstrap::SnapshotOptions,
) {
    deno_runtime::snapshot::create_runtime_snapshot(
        snapshot_path,
        snapshot_options,
        vec![uzumaki::init()],
    );
}

use crate::ops::*;

extension!(
  uzumaki,
  ops = [
    op_create_window,
    op_request_quit,
    op_request_redraw,
    op_get_root_node_id,
    op_create_element,
    op_create_text_node,
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
    op_get_selection,
    op_get_selected_text,
    op_read_clipboard_text,
    op_write_clipboard_text,
  ],
  esm_entry_point = "ext:uzumaki/runtime.js",
  esm = [ dir "core", "runtime.js" ],
);
