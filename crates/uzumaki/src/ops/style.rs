use deno_core::*;
use serde_json::Value;

use crate::app::{SharedAppState, with_state};
use crate::element::UzNodeId;

use super::style_util::{
    StyleEffect, clear_attribute, get_attribute, set_bool_attribute, set_number_attribute,
    set_str_attribute, sync_taffy, update_cursor,
};
#[op2(fast)]
pub fn op_set_str_attribute(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[string] name: &str,
    #[string] value: &str,
) {
    let nid = node_id as UzNodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return;
        };
        let effect = {
            let Some(node) = entry.dom.nodes.get_mut(nid) else {
                return;
            };
            set_str_attribute(node, name, value, entry.rem_base)
        };
        if matches!(effect, StyleEffect::AppliedNeedsSync) {
            sync_taffy(&mut entry.dom, nid);
        }
        if name == "cursor" {
            update_cursor(entry);
        }
    });
}

#[op2(fast)]
pub fn op_set_number_attribute(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[string] name: &str,
    value: f64,
) {
    let nid = node_id as UzNodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return;
        };
        let effect = {
            let Some(node) = entry.dom.nodes.get_mut(nid) else {
                return;
            };
            set_number_attribute(node, name, value as f32)
        };
        if matches!(effect, StyleEffect::AppliedNeedsSync) {
            sync_taffy(&mut entry.dom, nid);
        }
    });
}

#[op2(fast)]
pub fn op_set_bool_attribute(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[string] name: &str,
    value: bool,
) {
    let nid = node_id as UzNodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return;
        };
        let effect = {
            let Some(node) = entry.dom.nodes.get_mut(nid) else {
                return;
            };
            set_bool_attribute(node, name, value)
        };
        if matches!(effect, StyleEffect::AppliedNeedsSync) {
            sync_taffy(&mut entry.dom, nid);
        }
    });
}

#[op2(fast)]
pub fn op_clear_attribute(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[string] name: &str,
) {
    let nid = node_id as UzNodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return;
        };
        let effect = {
            let Some(node) = entry.dom.nodes.get_mut(nid) else {
                return;
            };
            clear_attribute(node, name)
        };
        if matches!(effect, StyleEffect::AppliedNeedsSync) {
            sync_taffy(&mut entry.dom, nid);
        }
        if name == "cursor" {
            update_cursor(entry);
        }
    });
}

#[op2]
#[serde]
pub fn op_get_attribute(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[string] name: String,
) -> Result<serde_json::Value, deno_error::JsErrorBox> {
    let nid = node_id as UzNodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get(&window_id) else {
            return Ok(Value::Null);
        };
        let Some(node) = entry.dom.nodes.get(nid) else {
            return Ok(Value::Null);
        };
        Ok(get_attribute(node, &name))
    })
}
