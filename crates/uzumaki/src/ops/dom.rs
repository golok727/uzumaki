use deno_core::*;

use crate::app::{SharedAppState, with_state};
use crate::element::NodeId;
use crate::selection::{DomSelection, SelectionRange};
use crate::style::*;

#[op2(fast)]
pub fn op_get_root_node_id(state: &mut OpState, #[smi] window_id: u32) -> u32 {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get(&window_id).expect("window not found");
        entry.dom.root.expect("no root node") as u32
    })
}

#[op2(fast)]
pub fn op_create_element(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[string] element_type: String,
) -> u32 {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        let id = if element_type == "input" {
            entry.dom.create_input(Style::default())
        } else {
            entry.dom.create_view(Style::default())
        };
        id as u32
    })
}

#[op2(fast)]
pub fn op_create_text_node(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[string] text: String,
) -> u32 {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        entry.dom.create_text(text, Style::default()) as u32
    })
}

#[op2(fast)]
pub fn op_append_child(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] parent_id: u32,
    #[smi] child_id: u32,
) {
    let pid = parent_id as NodeId;
    let cid = child_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        entry.dom.append_child(pid, cid);
    });
}

#[op2(fast)]
pub fn op_insert_before(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] parent_id: u32,
    #[smi] child_id: u32,
    #[smi] before_id: u32,
) {
    let pid = parent_id as NodeId;
    let cid = child_id as NodeId;
    let bid = before_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        entry.dom.insert_before(pid, cid, bid);
    });
}

#[op2(fast)]
pub fn op_remove_child(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] parent_id: u32,
    #[smi] child_id: u32,
) {
    let pid = parent_id as NodeId;
    let cid = child_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        entry.dom.remove_child(pid, cid);
    });
}

#[op2(fast)]
pub fn op_set_text(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[string] text: String,
) {
    let nid = node_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        entry.dom.set_text_content(nid, text);
    });
}

#[op2(fast)]
pub fn op_reset_dom(state: &mut OpState, #[smi] window_id: u32) {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        if let Some(entry) = s.windows.get_mut(&window_id) {
            let root = entry.dom.root.expect("no root node");
            entry.dom.clear_children(root);
        }
    });
}

// ── Input attribute ops ─────────────────────────────────────────────

#[op2(fast)]
pub fn op_set_input_value(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[string] value: String,
) {
    let nid = node_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        if let Some(node) = entry.dom.nodes.get_mut(nid)
            && let Some(is) = node.behavior.as_input_mut()
        {
            is.set_value(value);
        }
    });
}

#[op2]
#[string]
pub fn op_get_input_value(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
) -> String {
    let nid = node_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get(&window_id).expect("window not found");
        entry
            .dom
            .nodes
            .get(nid)
            .and_then(|node| node.behavior.as_input())
            .map(|is| is.model.text())
            .unwrap_or_default()
    })
}

#[op2(fast)]
pub fn op_set_input_placeholder(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[string] placeholder: String,
) {
    let nid = node_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        if let Some(node) = entry.dom.nodes.get_mut(nid)
            && let Some(is) = node.behavior.as_input_mut()
        {
            is.placeholder = placeholder;
        }
    });
}

#[op2(fast)]
pub fn op_set_input_disabled(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    disabled: bool,
) {
    let nid = node_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        if let Some(node) = entry.dom.nodes.get_mut(nid)
            && let Some(is) = node.behavior.as_input_mut()
        {
            is.disabled = disabled;
        }
    });
}

#[op2(fast)]
pub fn op_set_input_max_length(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[smi] max_length: i32,
) {
    let nid = node_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        if let Some(node) = entry.dom.nodes.get_mut(nid)
            && let Some(is) = node.behavior.as_input_mut()
        {
            is.model.max_length = if max_length > 0 {
                Some(max_length as usize)
            } else {
                None
            };
        }
    });
}

#[op2(fast)]
pub fn op_set_input_multiline(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    multiline: bool,
) {
    let nid = node_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        if let Some(node) = entry.dom.nodes.get_mut(nid)
            && let Some(is) = node.behavior.as_input_mut()
        {
            is.multiline = multiline;
        }
    });
}

#[op2(fast)]
pub fn op_set_input_secure(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    secure: bool,
) {
    let nid = node_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        if let Some(node) = entry.dom.nodes.get_mut(nid)
            && let Some(is) = node.behavior.as_input_mut()
        {
            is.secure = secure;
        }
    });
}

#[op2(fast)]
pub fn op_focus_input(state: &mut OpState, #[smi] window_id: u32, #[smi] node_id: u32) {
    let nid = node_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        entry.dom.set_selection(DomSelection {
            root: nid,
            range: SelectionRange::default(),
        });
    });
}

#[op2]
#[serde]
pub fn op_get_ancestor_path(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
) -> Vec<u32> {
    let nid = node_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get(&window_id).expect("window not found");
        let mut path = Vec::new();
        let mut current = Some(nid);
        while let Some(id) = current {
            path.push(id as u32);
            current = entry.dom.nodes.get(id).and_then(|n| n.parent);
        }
        path
    })
}

// ── Selection query ops ──────────────────────────────────────────────

#[op2]
#[serde]
pub fn op_get_selection(state: &mut OpState, #[smi] window_id: u32) -> serde_json::Value {
    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct SelectionState {
        root_node_id: u32,
        anchor_offset: usize,
        active_offset: usize,
        start: usize,
        end: usize,
        run_length: usize,
        is_collapsed: bool,
        text: String,
    }

    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get(&window_id).expect("window not found");
        let dom = &entry.dom;
        let Some(sel) = dom.selection() else {
            return serde_json::Value::Null;
        };
        let run_length = dom.selection_run_length().unwrap_or(0);
        let text = dom.selected_text();
        serde_json::to_value(SelectionState {
            root_node_id: sel.root as u32,
            anchor_offset: sel.anchor(),
            active_offset: sel.active(),
            start: sel.start(),
            end: sel.end(),
            run_length,
            is_collapsed: sel.is_collapsed(),
            text,
        })
        .unwrap()
    })
}

#[op2]
#[string]
pub fn op_get_selected_text(state: &mut OpState, #[smi] window_id: u32) -> String {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get(&window_id).expect("window not found");
        entry.dom.selected_text()
    })
}
