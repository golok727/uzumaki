use deno_core::*;

use crate::app::{SharedAppState, with_state};
use crate::element::UzNodeId;
use crate::style::UzStyle;

#[op2(fast)]
pub fn op_get_root_node_id(
    state: &mut OpState,
    #[smi] window_id: u32,
) -> Result<u32, deno_error::JsErrorBox> {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get(&window_id) else {
            return Err(deno_error::JsErrorBox::generic("window not found"));
        };
        Ok(entry.dom.root.expect("no root node") as u32)
    })
}

#[op2(fast)]
pub fn op_create_element(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[string] element_type: String,
) -> Result<u32, deno_error::JsErrorBox> {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(deno_error::JsErrorBox::generic("window not found"));
        };
        let id = if element_type == "input" {
            entry.dom.create_input(UzStyle::default())
        } else {
            entry.dom.create_view(UzStyle::default())
        };
        Ok(id as u32)
    })
}

#[op2(fast)]
pub fn op_create_text_node(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[string] text: String,
) -> Result<u32, deno_error::JsErrorBox> {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(deno_error::JsErrorBox::generic("window not found"));
        };
        Ok(entry.dom.create_text(text, UzStyle::default()) as u32)
    })
}

#[op2(fast)]
pub fn op_append_child(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] parent_id: u32,
    #[smi] child_id: u32,
) -> Result<(), deno_error::JsErrorBox> {
    let pid = parent_id as UzNodeId;
    let cid = child_id as UzNodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(deno_error::JsErrorBox::generic("window not found"));
        };
        entry.dom.append_child(pid, cid);
        Ok(())
    })
}

#[op2(fast)]
pub fn op_insert_before(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] parent_id: u32,
    #[smi] child_id: u32,
    #[smi] before_id: u32,
) -> Result<(), deno_error::JsErrorBox> {
    let pid = parent_id as UzNodeId;
    let cid = child_id as UzNodeId;
    let bid = before_id as UzNodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(deno_error::JsErrorBox::generic("window not found"));
        };
        entry.dom.insert_before(pid, cid, bid);
        Ok(())
    })
}

#[op2(fast)]
pub fn op_remove_child(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] parent_id: u32,
    #[smi] child_id: u32,
) -> Result<(), deno_error::JsErrorBox> {
    let pid = parent_id as UzNodeId;
    let cid = child_id as UzNodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(deno_error::JsErrorBox::generic("window not found"));
        };
        entry.dom.remove_child(pid, cid);
        Ok(())
    })
}

#[op2(fast)]
pub fn op_set_text(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[string] text: String,
) -> Result<(), deno_error::JsErrorBox> {
    let nid = node_id as UzNodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(deno_error::JsErrorBox::generic("window not found"));
        };
        entry.dom.set_text_content(nid, text);
        Ok(())
    })
}

#[op2(fast)]
pub fn op_reset_dom(
    state: &mut OpState,
    #[smi] window_id: u32,
) -> Result<(), deno_error::JsErrorBox> {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(deno_error::JsErrorBox::generic("window not found"));
        };
        let root = entry.dom.root.expect("no root node");
        entry.dom.clear_children(root);
        Ok(())
    })
}

#[op2(fast)]
pub fn op_set_input_value(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[string] value: String,
) -> Result<(), deno_error::JsErrorBox> {
    let nid = node_id as UzNodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(deno_error::JsErrorBox::generic("window not found"));
        };
        if let Some(node) = entry.dom.nodes.get_mut(nid)
            && let Some(is) = node.as_text_input_mut()
        {
            is.set_value(&value);
        }
        Ok(())
    })
}

#[op2]
#[string]
pub fn op_get_input_value(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
) -> Result<String, deno_error::JsErrorBox> {
    let nid = node_id as UzNodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get(&window_id) else {
            return Err(deno_error::JsErrorBox::generic("window not found"));
        };
        Ok(entry
            .dom
            .nodes
            .get(nid)
            .and_then(|node| node.as_text_input())
            .map(|is| is.text())
            .unwrap_or_default())
    })
}

#[op2(fast)]
pub fn op_set_input_placeholder(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[string] placeholder: String,
) -> Result<(), deno_error::JsErrorBox> {
    let nid = node_id as UzNodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(deno_error::JsErrorBox::generic("window not found"));
        };
        if let Some(node) = entry.dom.nodes.get_mut(nid)
            && let Some(is) = node.as_text_input_mut()
        {
            is.placeholder = placeholder;
        }
        Ok(())
    })
}

#[op2(fast)]
pub fn op_set_input_disabled(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    disabled: bool,
) -> Result<(), deno_error::JsErrorBox> {
    let nid = node_id as UzNodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(deno_error::JsErrorBox::generic("window not found"));
        };
        if let Some(node) = entry.dom.nodes.get_mut(nid)
            && let Some(is) = node.as_text_input_mut()
        {
            is.disabled = disabled;
        }
        Ok(())
    })
}

#[op2(fast)]
pub fn op_set_input_max_length(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[smi] max_length: i32,
) -> Result<(), deno_error::JsErrorBox> {
    let nid = node_id as UzNodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(deno_error::JsErrorBox::generic("window not found"));
        };
        if let Some(node) = entry.dom.nodes.get_mut(nid)
            && let Some(is) = node.as_text_input_mut()
        {
            is.max_length = if max_length > 0 {
                Some(max_length as usize)
            } else {
                None
            };
        }
        Ok(())
    })
}

#[op2(fast)]
pub fn op_set_input_multiline(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    multiline: bool,
) -> Result<(), deno_error::JsErrorBox> {
    let nid = node_id as UzNodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(deno_error::JsErrorBox::generic("window not found"));
        };
        if let Some(node) = entry.dom.nodes.get_mut(nid)
            && let Some(is) = node.as_text_input_mut()
        {
            is.multiline = multiline;
        }
        Ok(())
    })
}

#[op2(fast)]
pub fn op_set_input_secure(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    secure: bool,
) -> Result<(), deno_error::JsErrorBox> {
    let nid = node_id as UzNodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(deno_error::JsErrorBox::generic("window not found"));
        };
        if let Some(node) = entry.dom.nodes.get_mut(nid)
            && let Some(is) = node.as_text_input_mut()
        {
            is.secure = secure;
        }
        Ok(())
    })
}

#[op2(fast)]
pub fn op_focus_input(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
) -> Result<(), deno_error::JsErrorBox> {
    let nid = node_id as UzNodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(deno_error::JsErrorBox::generic("window not found"));
        };
        entry.dom.focus_input(nid);
        Ok(())
    })
}

#[op2]
#[serde]
pub fn op_get_ancestor_path(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
) -> Result<Vec<u32>, deno_error::JsErrorBox> {
    let nid = node_id as UzNodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get(&window_id) else {
            return Err(deno_error::JsErrorBox::generic("window not found"));
        };
        let mut path = Vec::new();
        let mut current = Some(nid);
        while let Some(id) = current {
            path.push(id as u32);
            current = entry.dom.nodes.get(id).and_then(|n| n.parent);
        }
        Ok(path)
    })
}

// Selection
#[op2]
#[serde]
pub fn op_get_selection(
    state: &mut OpState,
    #[smi] window_id: u32,
) -> Result<serde_json::Value, deno_error::JsErrorBox> {
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
        let Some(entry) = s.windows.get(&window_id) else {
            return Err(deno_error::JsErrorBox::generic("window not found"));
        };
        let dom = &entry.dom;
        let Some(sel) = dom.get_selection() else {
            return Ok(serde_json::Value::Null);
        };
        let Some(root) = sel.root else {
            return Ok(serde_json::Value::Null);
        };
        let run_length = dom.selection_run_length().unwrap_or(0);
        let text = dom.selected_text();
        Ok(serde_json::to_value(SelectionState {
            root_node_id: root as u32,
            anchor_offset: sel.anchor(),
            active_offset: sel.active(),
            start: sel.start(),
            end: sel.end(),
            run_length,
            is_collapsed: sel.is_collapsed(),
            text,
        })
        .unwrap())
    })
}

#[op2]
#[string]
pub fn op_get_selected_text(
    state: &mut OpState,
    #[smi] window_id: u32,
) -> Result<String, deno_error::JsErrorBox> {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get(&window_id) else {
            return Err(deno_error::JsErrorBox::generic("window not found"));
        };
        Ok(entry.dom.selected_text())
    })
}
