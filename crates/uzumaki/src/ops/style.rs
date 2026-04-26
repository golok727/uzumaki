use deno_core::*;
use serde_json::Value;

use crate::app::{SharedAppState, with_state};
use crate::element::UzNodeId;

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
        if let Some(entry) = s.windows.get_mut(&window_id) {
            entry.set_str_attribute(nid, name, value);
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
        if let Some(entry) = s.windows.get_mut(&window_id) {
            entry.set_number_attribute(nid, name, value);
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
        if let Some(entry) = s.windows.get_mut(&window_id) {
            entry.set_bool_attribute(nid, name, value);
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
        if let Some(entry) = s.windows.get_mut(&window_id) {
            entry.clear_attribute(nid, name);
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
        Ok(entry.get_attribute(nid, &name))
    })
}
