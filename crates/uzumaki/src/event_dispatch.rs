use serde::Serialize;
use winit::keyboard::{Key, NamedKey};

use crate::clipboard::SystemClipboard;
use crate::element::{ElementTree, NodeId, ScrollDragState};
use crate::input;
use crate::selection::{DomSelection, SelectionRange};
use crate::window::Window;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseEventData {
    pub window_id: u32,
    pub node_id: NodeId,
    pub x: f32,
    pub y: f32,
    pub screen_x: f32,
    pub screen_y: f32,
    pub button: u8,
    pub buttons: u8,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyEventData {
    pub window_id: u32,
    pub node_id: Option<NodeId>,
    pub key: String,
    pub code: String,
    pub key_code: u32,
    pub modifiers: u32,
    pub repeat: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowLoadEventData {
    pub window_id: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResizeEventData {
    pub window_id: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputEventData {
    pub window_id: u32,
    pub node_id: NodeId,
    pub value: String,
    pub input_type: String,
    pub data: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusEventData {
    pub window_id: u32,
    pub node_id: NodeId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardEventData {
    pub window_id: u32,
    pub node_id: Option<NodeId>,
    pub selection_text: Option<String>,
    pub clipboard_text: Option<String>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AppEvent {
    Click(MouseEventData),
    MouseDown(MouseEventData),
    MouseUp(MouseEventData),
    KeyDown(KeyEventData),
    KeyUp(KeyEventData),
    Resize(ResizeEventData),
    Input(InputEventData),
    Focus(FocusEventData),
    Blur(FocusEventData),
    Copy(ClipboardEventData),
    Cut(ClipboardEventData),
    Paste(ClipboardEventData),
    #[serde(rename = "windowLoad")]
    WindowLoad(WindowLoadEventData),
    HotReload,
}

pub fn handle_redraw(
    dom: &mut ElementTree,
    handle: &mut Window,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) {
    handle.paint_and_present(device, queue, dom);
}

/// Scroll the focused input so the cursor stays visible.
/// Call this after any action that moves the cursor (key press, click, drag).
pub fn scroll_input_to_cursor(dom: &mut ElementTree, handle: &mut Window) {
    let Some(focused_id) = dom.focused_node else {
        return;
    };
    let scroll_info = dom.nodes.get(focused_id).and_then(|node| {
        node.behavior.as_input().map(|is| {
            let display_text = is.display_text();
            let font_size = node.style.text.font_size;
            let padding = node.style.padding.left;
            let input_padding = if padding > 0.0 { padding } else { 8.0 };
            let pt = node.style.padding.top;
            let top_pad = if pt > 0.0 { pt } else { 4.0 };
            let cursor_pos = is.range().active;
            let taffy_node = node.taffy_node;
            let multiline = is.multiline;
            (
                display_text,
                font_size,
                input_padding,
                top_pad,
                cursor_pos,
                taffy_node,
                multiline,
            )
        })
    });
    let Some((display_text, font_size, input_padding, top_pad, cursor_pos, taffy_node, multiline)) =
        scroll_info
    else {
        return;
    };

    let (input_width, input_height) = dom
        .taffy
        .layout(taffy_node)
        .map(|l| (l.size.width - input_padding * 2.0, l.size.height))
        .unwrap_or((200.0, 100.0));

    if multiline {
        let positions =
            handle
                .text_renderer
                .grapheme_positions_2d(&display_text, font_size, Some(input_width));
        let cursor_y = if cursor_pos < positions.len() {
            positions[cursor_pos].y
        } else {
            positions.last().map(|p| p.y).unwrap_or(0.0)
        };
        let line_height = (font_size * 1.2).round();
        if let Some(node) = dom.nodes.get_mut(focused_id)
            && let Some(is) = node.behavior.as_input_mut()
        {
            is.update_scroll_y(cursor_y, line_height, input_height - top_pad * 2.0);
        }
    } else {
        let positions = handle
            .text_renderer
            .grapheme_x_positions(&display_text, font_size);
        let cursor_x = if cursor_pos < positions.len() {
            positions[cursor_pos]
        } else {
            positions.last().copied().unwrap_or(0.0)
        };
        if let Some(node) = dom.nodes.get_mut(focused_id)
            && let Some(is) = node.behavior.as_input_mut()
        {
            is.update_scroll(cursor_x, input_width);
        }
    }
}

// ── Cursor moved ─────────────────────────────────────────────────────

pub fn handle_cursor_moved(
    dom: &mut ElementTree,
    handle: &mut Window,
    position: winit::dpi::PhysicalPosition<f64>,
    mouse_buttons: u8,
) -> bool {
    let mut needs_redraw = false;
    let scale = handle.winit_window.scale_factor();
    let logical_x = position.x / scale;
    let logical_y = position.y / scale;
    let old_top = dom.hit_state.top_node;
    dom.update_hit_test(logical_x, logical_y);
    if old_top != dom.hit_state.top_node {
        needs_redraw = true;
    }

    // Scroll thumb drag
    if let Some(ref drag) = dom.scroll_drag {
        let delta_y = logical_y - drag.start_mouse_y;
        let new_offset = if drag.track_range > 0.0 {
            drag.start_scroll_offset + (delta_y as f32 / drag.track_range as f32) * drag.max_scroll
        } else {
            drag.start_scroll_offset
        };
        let nid = drag.node_id;
        let max = drag.max_scroll;
        if let Some(node) = dom.nodes.get_mut(nid) {
            if let Some(ss) = &mut node.scroll_state {
                ss.scroll_offset_y = new_offset.clamp(0.0, max);
            } else if let Some(is) = node.behavior.as_input_mut() {
                is.scroll_offset_y = new_offset.clamp(0.0, max);
            }
        }
        needs_redraw = true;
    }

    // Input drag selection
    if mouse_buttons & 1 != 0 {
        if let Some(drag_nid) = dom.dragging_input {
            let cursor_info = dom.nodes.get(drag_nid).and_then(|node| {
                node.behavior.as_input().map(|is| {
                    let display_text = is.display_text();
                    let font_size = node.style.text.font_size;
                    let scroll_offset = is.scroll_offset;
                    let scroll_offset_y = is.scroll_offset_y;
                    let is_multiline = is.multiline;
                    let padding = node.style.padding.left as f64;
                    let input_padding = if padding > 0.0 { padding } else { 8.0 };
                    let pad_top = node.style.padding.top;
                    let top_pad = if pad_top > 0.0 { pad_top } else { 4.0 };
                    let hitbox_bounds = node
                        .interactivity
                        .hitbox_id
                        .and_then(|hid| dom.hitbox_store.get(hid))
                        .map(|hb| hb.bounds);
                    let taffy_node = node.taffy_node;
                    (
                        display_text,
                        font_size,
                        scroll_offset,
                        scroll_offset_y,
                        is_multiline,
                        input_padding,
                        top_pad,
                        hitbox_bounds,
                        taffy_node,
                    )
                })
            });

            // TODO  oh my gaaah TT please refactor this
            if let Some((
                display_text,
                font_size,
                scroll_offset,
                scroll_offset_y,
                is_multiline,
                input_padding,
                top_pad,
                Some(hb),
                taffy_node,
            )) = cursor_info
            {
                let grapheme_idx = if !display_text.is_empty() {
                    if is_multiline {
                        let wrap_width = dom
                            .taffy
                            .layout(taffy_node)
                            .map(|l| l.size.width - input_padding as f32 * 2.0)
                            .unwrap_or(200.0);
                        let relative_x = (logical_x - hb.x - input_padding) as f32;
                        let relative_y = (logical_y - hb.y) as f32 + scroll_offset_y - top_pad;
                        handle.text_renderer.hit_to_grapheme_2d(
                            &display_text,
                            font_size,
                            Some(wrap_width),
                            relative_x,
                            relative_y,
                        )
                    } else {
                        let relative_x = (logical_x - hb.x - input_padding) as f32 + scroll_offset;
                        handle
                            .text_renderer
                            .hit_to_grapheme(&display_text, font_size, relative_x)
                    }
                } else {
                    0
                };

                if let Some(node) = dom.nodes.get_mut(drag_nid)
                    && let Some(is) = node.behavior.as_input_mut()
                {
                    is.update_range(|range| {
                        range.active = grapheme_idx;
                    });
                    is.reset_blink();
                }
                scroll_input_to_cursor(dom, handle);
                needs_redraw = true;
            }
        }

        // View text selection drag
        if let Some(root_id) = dom.dragging_view_selection
            && let Some(flat_idx) = hit_text_in_run(
                dom,
                &mut handle.text_renderer,
                root_id,
                logical_x,
                logical_y,
            )
        {
            if let Some(mut sel) = dom.selection() {
                sel.range.active = flat_idx;
                dom.set_selection(sel);
            }
            needs_redraw = true;
        }
    }

    needs_redraw
}

// ── View text selection helpers ──────────────────────────────────────

/// Hit-test a mouse position against all text nodes in a textSelect run.
/// Returns the flat grapheme index if a suitable text node is found.
fn hit_text_in_run(
    dom: &ElementTree,
    text_renderer: &mut crate::text::TextRenderer,
    root_id: crate::element::NodeId,
    mx: f64,
    my: f64,
) -> Option<usize> {
    use crate::style::Bounds;

    let run = dom
        .selectable_text_runs
        .iter()
        .find(|r| r.root_id == root_id)?;

    // Find the text node closest to mouse position
    let mut best: Option<(crate::element::NodeId, f64, Bounds)> = None;
    for entry in &run.entries {
        let node = dom.nodes.get(entry.node_id)?;
        let hid = node.interactivity.hitbox_id?;
        let hb = dom.hitbox_store.get(hid)?;
        let dist = point_to_rect_dist(mx, my, &hb.bounds);
        if best.is_none() || dist < best.unwrap().1 {
            best = Some((entry.node_id, dist, hb.bounds));
        }
    }

    let (node_id, _, bounds) = best?;
    let entry = run.entries.iter().find(|e| e.node_id == node_id)?;
    let node = dom.nodes.get(node_id)?;
    let text = node.behavior.as_text()?;
    let font_size = node.style.text.font_size;

    if text.content.is_empty() {
        return Some(entry.flat_start);
    }

    let relative_x = (mx - bounds.x) as f32;
    let relative_y = (my - bounds.y) as f32;
    let local_idx = text_renderer.hit_to_grapheme_2d(
        &text.content,
        font_size,
        Some(bounds.width as f32),
        relative_x,
        relative_y,
    );

    Some(entry.flat_start + local_idx.min(entry.grapheme_count))
}

fn point_to_rect_dist(px: f64, py: f64, bounds: &crate::style::Bounds) -> f64 {
    let cx = px.clamp(bounds.x, bounds.x + bounds.width);
    let cy = py.clamp(bounds.y, bounds.y + bounds.height);
    let dx = px - cx;
    let dy = py - cy;
    (dx * dx + dy * dy).sqrt()
}

/// Find word boundaries around a flat grapheme index within a text run.
fn word_boundaries_in_run(
    dom: &ElementTree,
    root_id: crate::element::NodeId,
    flat_idx: usize,
) -> (usize, usize) {
    let Some(run) = dom
        .selectable_text_runs
        .iter()
        .find(|r| r.root_id == root_id)
    else {
        return (flat_idx, flat_idx);
    };
    let chars: Vec<char> = run.flat_text.chars().collect();
    let graphemes: Vec<&str> =
        unicode_segmentation::UnicodeSegmentation::graphemes(run.flat_text.as_str(), true)
            .collect();
    // Map grapheme index to char index
    let mut char_idx = 0usize;
    for (i, g) in graphemes.iter().enumerate() {
        if i == flat_idx {
            break;
        }
        char_idx += g.chars().count();
    }

    let is_word = |c: char| c.is_alphanumeric() || c == '_';

    // Find word start
    let mut start_char = char_idx;
    if start_char < chars.len() && is_word(chars[start_char]) {
        while start_char > 0 && is_word(chars[start_char - 1]) {
            start_char -= 1;
        }
    }
    // Find word end
    let mut end_char = char_idx;
    if end_char < chars.len() && is_word(chars[end_char]) {
        while end_char < chars.len() && is_word(chars[end_char]) {
            end_char += 1;
        }
    } else if end_char < chars.len() {
        end_char += 1;
    }

    // Convert char indices back to grapheme indices
    let mut gi = 0usize;
    let mut ci = 0usize;
    let mut start_g = 0;
    let mut end_g = graphemes.len();
    for g in &graphemes {
        if ci == start_char {
            start_g = gi;
        }
        ci += g.chars().count();
        gi += 1;
        if ci == end_char {
            end_g = gi;
        }
    }

    (start_g, end_g)
}

// ── Mouse input ──────────────────────────────────────────────────────

pub fn handle_mouse_input(
    dom: &mut ElementTree,
    handle: &mut Window,
    wid: u32,
    btn_state: winit::event::ElementState,
    button: winit::event::MouseButton,
    mouse_buttons: u8,
) -> (bool, Vec<AppEvent>) {
    use winit::event::ElementState;

    let mut needs_redraw = false;
    let mut events: Vec<AppEvent> = Vec::new();

    let mouse_button = match button {
        winit::event::MouseButton::Left => crate::interactivity::MouseButton::Left,
        winit::event::MouseButton::Right => crate::interactivity::MouseButton::Right,
        winit::event::MouseButton::Middle => crate::interactivity::MouseButton::Middle,
        _ => crate::interactivity::MouseButton::Left,
    };

    let button_num: u8 = match button {
        winit::event::MouseButton::Left => 0,
        winit::event::MouseButton::Middle => 1,
        winit::event::MouseButton::Right => 2,
        _ => 0,
    };

    let Some((mx, my)) = dom.hit_state.mouse_position else {
        return (needs_redraw, events);
    };
    let x = mx as f32;
    let y = my as f32;

    // Check scroll thumb click (left button press)
    if btn_state == ElementState::Pressed && button == winit::event::MouseButton::Left {
        let thumb_hit = dom
            .scroll_thumbs
            .iter()
            .rev()
            .find(|t| t.thumb_bounds.contains(mx, my));
        if let Some(t) = thumb_hit {
            let nid = t.node_id;
            let visible_h = t.visible_height;
            let content_h = t.content_height;
            let max_scroll = (content_h - visible_h).max(0.0);
            let ratio = visible_h as f64 / content_h as f64;
            let thumb_height = (t.view_bounds.height * ratio).max(24.0);
            let track_range = t.view_bounds.height - thumb_height;
            let start_offset = dom
                .nodes
                .get(nid)
                .map(|n| {
                    n.scroll_state
                        .as_ref()
                        .map(|ss| ss.scroll_offset_y)
                        .or_else(|| n.behavior.as_input().map(|is| is.scroll_offset_y))
                        .unwrap_or(0.0)
                })
                .unwrap_or(0.0);
            dom.scroll_drag = Some(ScrollDragState {
                node_id: nid,
                start_mouse_y: my,
                start_scroll_offset: start_offset,
                track_range,
                max_scroll,
            });
            return (true, events);
        }
    }

    // End scroll drag on mouse up
    if btn_state == ElementState::Released
        && button == winit::event::MouseButton::Left
        && dom.scroll_drag.is_some()
    {
        dom.scroll_drag = None;
    }

    // Resolve topmost hit → NodeId for JS event target
    let js_target = dom.hit_state.top_node;

    match btn_state {
        ElementState::Pressed => {
            dom.set_active(js_target);
            dom.dispatch_mouse_down(mx, my, mouse_button);
            if let Some(target) = js_target {
                events.push(AppEvent::MouseDown(MouseEventData {
                    window_id: wid,
                    node_id: target,
                    x,
                    y,
                    screen_x: x,
                    screen_y: y,
                    button: button_num,
                    buttons: mouse_buttons,
                }));
            }

            // Input focus handling (left button)
            if mouse_button == crate::interactivity::MouseButton::Left {
                let clicked_is_input = js_target
                    .and_then(|nid| dom.nodes.get(nid))
                    .map(|n| n.behavior.is_input())
                    .unwrap_or(false);

                let old_focus = dom.focused_node;

                if clicked_is_input {
                    let nid = js_target.unwrap();

                    // Multi-click detection (double=word, triple=line, quad=select all)
                    let now = std::time::Instant::now();
                    let is_consecutive = dom.last_click_node == Some(nid)
                        && dom
                            .last_click_time
                            .is_some_and(|t| now.duration_since(t).as_millis() < 400);
                    dom.last_click_time = Some(now);
                    dom.last_click_node = Some(nid);
                    if is_consecutive {
                        dom.click_count = (dom.click_count + 1).min(4);
                    } else {
                        dom.click_count = 1;
                    }

                    // Focus if not already focused
                    if old_focus != Some(nid) {
                        if let Some(old_id) = old_focus {
                            events.push(AppEvent::Blur(FocusEventData {
                                window_id: wid,
                                node_id: old_id,
                            }));
                        }
                        events.push(AppEvent::Focus(FocusEventData {
                            window_id: wid,
                            node_id: nid,
                        }));
                    }

                    // Place cursor at click position
                    let cursor_info = {
                        let node = &dom.nodes[nid];
                        let is = node.behavior.as_input().unwrap();
                        let display_text = is.display_text();
                        let font_size = node.style.text.font_size;
                        let scroll_offset = is.scroll_offset;
                        let scroll_offset_y = is.scroll_offset_y;
                        let is_multiline = is.multiline;
                        let padding = node.style.padding.left as f64;
                        let input_padding = if padding > 0.0 { padding } else { 8.0 };
                        let pad_top = node.style.padding.top;
                        let top_pad = if pad_top > 0.0 { pad_top } else { 4.0 };
                        let hitbox_bounds = node
                            .interactivity
                            .hitbox_id
                            .and_then(|hid| dom.hitbox_store.get(hid))
                            .map(|hb| hb.bounds);
                        let taffy_node = node.taffy_node;
                        (
                            display_text,
                            font_size,
                            scroll_offset,
                            scroll_offset_y,
                            is_multiline,
                            input_padding,
                            top_pad,
                            hitbox_bounds,
                            taffy_node,
                        )
                    };
                    let (
                        display_text,
                        font_size,
                        scroll_offset,
                        scroll_offset_y,
                        is_multiline,
                        input_padding,
                        top_pad,
                        hitbox_bounds,
                        taffy_node,
                    ) = cursor_info;

                    if let Some(hb) = hitbox_bounds {
                        let grapheme_idx = if !display_text.is_empty() {
                            if is_multiline {
                                let wrap_width = dom
                                    .taffy
                                    .layout(taffy_node)
                                    .map(|l| l.size.width - input_padding as f32 * 2.0)
                                    .unwrap_or(200.0);
                                let relative_x = (mx - hb.x - input_padding) as f32;
                                let relative_y = (my - hb.y) as f32 + scroll_offset_y - top_pad;
                                handle.text_renderer.hit_to_grapheme_2d(
                                    &display_text,
                                    font_size,
                                    Some(wrap_width),
                                    relative_x,
                                    relative_y,
                                )
                            } else {
                                let relative_x = (mx - hb.x - input_padding) as f32 + scroll_offset;
                                handle.text_renderer.hit_to_grapheme(
                                    &display_text,
                                    font_size,
                                    relative_x,
                                )
                            }
                        } else {
                            0
                        };

                        // Ensure selection root is set to this input before
                        // any input methods touch the shared range.
                        dom.set_selection(DomSelection {
                            root: nid,
                            range: SelectionRange::default(),
                        });

                        if let Some(node) = dom.nodes.get_mut(nid)
                            && let Some(is) = node.behavior.as_input_mut()
                        {
                            match dom.click_count {
                                2 => {
                                    // Double-click: select word
                                    let (ws, we) = is.word_at(grapheme_idx);
                                    is.set_selection(ws, we);
                                }
                                3 => {
                                    // Triple-click: select line/paragraph
                                    let (ls, le) = is.line_at(grapheme_idx);
                                    is.set_selection(ls, le);
                                }
                                4 => {
                                    // Quad-click: select all
                                    is.select_all();
                                }
                                _ => {
                                    // Single click: place cursor
                                    is.set_cursor(grapheme_idx);
                                }
                            }
                            is.reset_blink();
                        }
                    }

                    scroll_input_to_cursor(dom, handle);
                    dom.dragging_input = Some(nid);
                } else {
                    // Clicked non-input: blur focused input
                    if let Some(old_id) = old_focus {
                        if let Some(old_node) = dom.nodes.get_mut(old_id)
                            && let Some(is) = old_node.behavior.as_input_mut()
                        {
                            is.focused = false;
                        }
                        dom.focused_node = None;
                        events.push(AppEvent::Blur(FocusEventData {
                            window_id: wid,
                            node_id: old_id,
                        }));
                    }

                    // Check if clicked on a text node inside a textSelect view
                    let clicked_text_selectable = js_target
                        .map(|nid| dom.is_text_selectable(nid))
                        .unwrap_or(false);

                    if clicked_text_selectable {
                        let nid = js_target.unwrap();

                        // Starting a view selection blurs any focused input
                        if let Some(old_id) = dom.focused_node.take() {
                            if let Some(old_node) = dom.nodes.get_mut(old_id)
                                && let Some(is) = old_node.behavior.as_input_mut()
                            {
                                is.focused = false;
                            }
                            events.push(AppEvent::Blur(FocusEventData {
                                window_id: wid,
                                node_id: old_id,
                            }));
                        }

                        // Find the run this text node belongs to
                        if let Some((run_root, flat_idx)) = {
                            dom.find_run_entry_for_node(nid).and_then(|(run, entry)| {
                                let node = dom.nodes.get(nid)?;
                                let text = node.behavior.as_text()?;
                                let font_size = node.style.text.font_size;
                                let bounds = node
                                    .interactivity
                                    .hitbox_id
                                    .and_then(|hid| dom.hitbox_store.get(hid))
                                    .map(|hb| hb.bounds)?;
                                let local_idx = if text.content.is_empty() {
                                    0
                                } else {
                                    let rel_x = (mx - bounds.x) as f32;
                                    let rel_y = (my - bounds.y) as f32;
                                    handle.text_renderer.hit_to_grapheme_2d(
                                        &text.content,
                                        font_size,
                                        Some(bounds.width as f32),
                                        rel_x,
                                        rel_y,
                                    )
                                };
                                let flat = entry.flat_start + local_idx.min(entry.grapheme_count);
                                Some((run.root_id, flat))
                            })
                        } {
                            // Multi-click detection
                            let now = std::time::Instant::now();
                            let is_consecutive = dom.last_click_node == Some(nid)
                                && dom
                                    .last_click_time
                                    .is_some_and(|t| now.duration_since(t).as_millis() < 400);
                            dom.last_click_time = Some(now);
                            dom.last_click_node = Some(nid);
                            if is_consecutive {
                                dom.click_count = (dom.click_count + 1).min(4);
                            } else {
                                dom.click_count = 1;
                            }

                            match dom.click_count {
                                2 => {
                                    let (ws, we) = word_boundaries_in_run(dom, run_root, flat_idx);
                                    dom.set_selection(DomSelection::new(run_root, ws, we));
                                }
                                3 => {
                                    // Select entire text node (line-level)
                                    if let Some((run, entry)) = dom.find_run_entry_for_node(nid) {
                                        dom.set_selection(DomSelection::new(
                                            run.root_id,
                                            entry.flat_start,
                                            entry.flat_start + entry.grapheme_count,
                                        ));
                                    }
                                }
                                4 => {
                                    // Select all text in the run
                                    if let Some(run) = dom
                                        .selectable_text_runs
                                        .iter()
                                        .find(|r| r.root_id == run_root)
                                    {
                                        dom.set_selection(DomSelection::new(
                                            run_root,
                                            0,
                                            run.total_graphemes,
                                        ));
                                    }
                                }
                                _ => {
                                    // Single click: place cursor
                                    dom.set_selection(DomSelection::new(
                                        run_root, flat_idx, flat_idx,
                                    ));
                                }
                            }
                            dom.dragging_view_selection = Some(run_root);
                        }
                    } else {
                        // Clicked on non-selectable area: clear view selection
                        dom.clear_selection();
                    }
                }
            }

            needs_redraw = true;
        }
        ElementState::Released => {
            dom.dispatch_mouse_up(mx, my, mouse_button);
            if let Some(target) = js_target {
                events.push(AppEvent::MouseUp(MouseEventData {
                    window_id: wid,
                    node_id: target,
                    x,
                    y,
                    screen_x: x,
                    screen_y: y,
                    button: button_num,
                    buttons: mouse_buttons,
                }));
            }
            // Click fires if released on the same element that was pressed
            if let Some(active) = dom.hit_state.active_node
                && dom.hit_state.is_hovered(active)
            {
                dom.dispatch_click(mx, my, mouse_button);
                if let Some(target) = js_target {
                    events.push(AppEvent::Click(MouseEventData {
                        window_id: wid,
                        node_id: target,
                        x,
                        y,
                        screen_x: x,
                        screen_y: y,
                        button: button_num,
                        buttons: mouse_buttons,
                    }));
                }
            }
            dom.set_active(None);
            dom.dragging_input = None;
            dom.dragging_view_selection = None;
            needs_redraw = true;
        }
    }

    (needs_redraw, events)
}

/// Build the raw KeyDown/KeyUp event. Returns None for F5 (hot reload) or unmappable keys.
pub fn build_key_event(
    dom: &ElementTree,
    wid: u32,
    key_event: &winit::event::KeyEvent,
    modifiers: u32,
) -> Option<AppEvent> {
    use winit::event::ElementState;
    use winit::keyboard::PhysicalKey;

    // F5 hot reload
    if key_event.state == ElementState::Pressed && key_event.logical_key == Key::Named(NamedKey::F5)
    {
        return Some(AppEvent::HotReload);
    }

    let key_str = match &key_event.logical_key {
        Key::Character(c) => c.to_string(),
        Key::Named(named) => format!("{:?}", named),
        _ => return None,
    };

    let code_str = match key_event.physical_key {
        PhysicalKey::Code(kc) => format!("{:?}", kc),
        _ => String::new(),
    };

    let data = KeyEventData {
        window_id: wid,
        node_id: dom.focused_node,
        key: key_str,
        code: code_str,
        key_code: 0,
        modifiers,
        repeat: key_event.repeat,
    };

    Some(match key_event.state {
        ElementState::Pressed => AppEvent::KeyDown(data),
        ElementState::Released => AppEvent::KeyUp(data),
    })
}

/// Handle keyboard input for the focused input element. Called AFTER the raw key
/// event has been dispatched to JS (so preventDefault can suppress this).
/// Returns (needs_redraw, events_to_dispatch).
pub fn handle_key_for_input(
    dom: &mut ElementTree,
    handle: &mut Window,
    wid: u32,
    key_event: &winit::event::KeyEvent,
    modifiers: u32,
) -> (bool, Vec<AppEvent>) {
    use winit::event::ElementState;

    let mut needs_redraw = false;
    let mut events: Vec<AppEvent> = Vec::new();

    if key_event.state != ElementState::Pressed {
        return (needs_redraw, events);
    }

    let new_focus = dom
        .with_focused_node(|node, focused_id| {
            let mut new_focus = Some(focused_id);

            let shift = modifiers & 4 != 0;

            // Handle ArrowUp/ArrowDown externally (vertical nav)
            let is_vertical_nav = matches!(
                key_event.logical_key,
                Key::Named(NamedKey::ArrowUp) | Key::Named(NamedKey::ArrowDown)
            );

            if is_vertical_nav {
                let is_up = key_event.logical_key == Key::Named(NamedKey::ArrowUp);
                let extend = shift;

                if let Some(is) = node.behavior.as_input_mut() {
                    let (_, cur_col) = is.cursor_rowcol();
                    let sticky = is.sticky_col.unwrap_or(cur_col);
                    if is_up {
                        is.move_up(extend, Some(sticky));
                    } else {
                        is.move_down(extend, Some(sticky));
                    }
                    is.sticky_col = Some(sticky);
                    is.sticky_x = None;
                    is.reset_blink();
                }

                needs_redraw = true;
            } else {
                // Non-vertical key: delegate to InputState::handle_key
                if let Some(input_state) = node.behavior.as_input_mut() {
                    let result = input_state.handle_key(&key_event.logical_key, modifiers);
                    match result {
                        input::KeyResult::Edit(edit) => {
                            let value = input_state.model.text();
                            let input_type = match edit.kind {
                                input::EditKind::Insert => "insertText",
                                input::EditKind::InsertFromPaste => "insertFromPaste",
                                input::EditKind::DeleteBackward => "deleteContentBackward",
                                input::EditKind::DeleteForward => "deleteContentForward",
                                input::EditKind::DeleteWordBackward => "deleteWordBackward",
                                input::EditKind::DeleteWordForward => "deleteWordForward",
                                input::EditKind::DeleteByCut => "deleteByCut",
                            };
                            events.push(AppEvent::Input(InputEventData {
                                window_id: wid,
                                node_id: focused_id,
                                value,
                                input_type: input_type.to_string(),
                                data: edit.inserted,
                            }));
                            needs_redraw = true;
                        }
                        input::KeyResult::Blur => {
                            input_state.focused = false;
                            new_focus = None;
                            events.push(AppEvent::Blur(FocusEventData {
                                window_id: wid,
                                node_id: focused_id,
                            }));
                            needs_redraw = true;
                        }
                        input::KeyResult::Handled => {
                            needs_redraw = true;
                        }
                        input::KeyResult::Ignored => {}
                    }
                }
            }
            new_focus
        })
        .flatten();

    dom.focused_node = new_focus;

    if needs_redraw {
        scroll_input_to_cursor(dom, handle);
    }

    (needs_redraw, events)
}

/// Handle keyboard shortcuts for view text selection (Shift+Arrows, Ctrl+A, etc.)
/// Called after input-level processing, only when there's no focused input.
/// Returns true if a redraw is needed.
pub fn handle_key_for_view_selection(
    dom: &mut ElementTree,
    key_event: &winit::event::KeyEvent,
    modifiers: u32,
) -> bool {
    use winit::event::ElementState;

    if key_event.state != ElementState::Pressed {
        return false;
    }

    let Some(sel) = dom.selection() else {
        return false;
    };

    let root = sel.root;
    let SelectionRange { anchor, active } = sel.range;

    let run_len = dom
        .selectable_text_runs
        .iter()
        .find(|r| r.root_id == root)
        .map(|r| r.total_graphemes)
        .unwrap_or(0);

    if run_len == 0 {
        return false;
    }

    let shift = modifiers & 4 != 0;
    let ctrl = modifiers & 1 != 0;

    match &key_event.logical_key {
        Key::Named(NamedKey::ArrowLeft) if shift && ctrl => {
            // Move active to previous word boundary
            let new_active = prev_word_boundary_in_run(dom, root, active);
            dom.set_selection(DomSelection::new(root, anchor, new_active));
            true
        }
        Key::Named(NamedKey::ArrowRight) if shift && ctrl => {
            let new_active = next_word_boundary_in_run(dom, root, active);
            dom.set_selection(DomSelection::new(root, anchor, new_active));
            true
        }
        Key::Named(NamedKey::ArrowLeft) if shift => {
            let new_active = if active > 0 { active - 1 } else { 0 };
            dom.set_selection(DomSelection::new(root, anchor, new_active));
            true
        }
        Key::Named(NamedKey::ArrowRight) if shift => {
            let new_active = (active + 1).min(run_len);
            dom.set_selection(DomSelection::new(root, anchor, new_active));
            true
        }
        Key::Named(NamedKey::Home) if shift => {
            dom.set_selection(DomSelection::new(root, anchor, 0));
            true
        }
        Key::Named(NamedKey::End) if shift => {
            dom.set_selection(DomSelection::new(root, anchor, run_len));
            true
        }
        Key::Character(c) if ctrl && (c.as_ref() == "a" || c.as_ref() == "A") => {
            dom.set_selection(DomSelection::new(root, 0, run_len));
            true
        }
        _ => false,
    }
}

// ── Clipboard command pipeline ──────────────────────────────────────

/// Identifies the target of a clipboard operation.
pub enum ClipboardTarget {
    /// Focused input node.
    Input(NodeId),
    /// Non-input text selection root.
    ViewSelection(NodeId),
}

/// A resolved clipboard command ready for event dispatch and default action.
pub enum ClipboardCommand {
    Copy {
        target: Option<NodeId>,
        selection_text: String,
    },
    Cut {
        target: Option<NodeId>,
        selection_text: String,
        is_input: bool,
    },
    Paste {
        target: Option<NodeId>,
        clipboard_text: Option<String>,
        is_input: bool,
    },
}

/// Resolve the current clipboard target from DOM state.
fn resolve_clipboard_target(dom: &ElementTree) -> Option<ClipboardTarget> {
    if let Some(focused_id) = dom.focused_node
        && let Some(node) = dom.nodes.get(focused_id)
        && node.behavior.as_input().is_some()
    {
        return Some(ClipboardTarget::Input(focused_id));
    }
    if let Some(sel) = dom.selection()
        && !sel.is_collapsed()
    {
        return Some(ClipboardTarget::ViewSelection(sel.root));
    }
    None
}

/// Detect whether a key event is a clipboard shortcut and build the corresponding
/// command. Returns `None` if the key is not a clipboard shortcut.
pub fn build_clipboard_command(
    dom: &ElementTree,
    key_event: &winit::event::KeyEvent,
    modifiers: u32,
    clipboard: &mut SystemClipboard,
) -> Option<ClipboardCommand> {
    use winit::event::ElementState;

    if key_event.state != ElementState::Pressed {
        return None;
    }

    let ctrl = modifiers & 1 != 0;
    if !ctrl {
        return None;
    }

    let ch = match &key_event.logical_key {
        Key::Character(c) => c.as_ref(),
        _ => return None,
    };

    match ch {
        "c" | "C" => {
            let target = resolve_clipboard_target(dom);
            let selection_text = match &target {
                Some(ClipboardTarget::Input(nid)) => {
                    let node = dom.nodes.get(*nid)?;
                    let is = node.behavior.as_input()?;
                    if is.secure {
                        return None; // Block copy on secure inputs
                    }
                    let text = is.selected_text();
                    if text.is_empty() {
                        return None;
                    }
                    text
                }
                Some(ClipboardTarget::ViewSelection(_)) => {
                    let text = dom.selected_text();
                    if text.is_empty() {
                        return None;
                    }
                    text
                }
                None => return None,
            };
            let target_id = match &target {
                Some(ClipboardTarget::Input(nid)) => Some(*nid),
                Some(ClipboardTarget::ViewSelection(nid)) => Some(*nid),
                None => None,
            };
            Some(ClipboardCommand::Copy {
                target: target_id,
                selection_text,
            })
        }
        "x" | "X" => {
            let target = resolve_clipboard_target(dom);
            let (target_id, is_input) = match &target {
                Some(ClipboardTarget::Input(nid)) => {
                    let node = dom.nodes.get(*nid)?;
                    let is = node.behavior.as_input()?;
                    if is.secure {
                        return None; // Block cut on secure inputs
                    }
                    (Some(*nid), true)
                }
                Some(ClipboardTarget::ViewSelection(nid)) => (Some(*nid), false),
                None => return None,
            };
            let selection_text = match &target {
                Some(ClipboardTarget::Input(nid)) => {
                    let node = dom.nodes.get(*nid)?;
                    let is = node.behavior.as_input()?;
                    let text = is.selected_text();
                    if text.is_empty() {
                        return None;
                    }
                    text
                }
                Some(ClipboardTarget::ViewSelection(_)) => {
                    let text = dom.selected_text();
                    if text.is_empty() {
                        return None;
                    }
                    text
                }
                None => return None,
            };
            Some(ClipboardCommand::Cut {
                target: target_id,
                selection_text,
                is_input,
            })
        }
        "v" | "V" => {
            let target = resolve_clipboard_target(dom);
            let (target_id, is_input) = match &target {
                Some(ClipboardTarget::Input(nid)) => (Some(*nid), true),
                Some(ClipboardTarget::ViewSelection(nid)) => (Some(*nid), false),
                None => return None,
            };
            let clipboard_text = clipboard.read_text().unwrap_or(None);
            Some(ClipboardCommand::Paste {
                target: target_id,
                clipboard_text,
                is_input,
            })
        }
        _ => None,
    }
}

/// Build the AppEvent for dispatching a clipboard command to JS.
pub fn clipboard_command_to_event(cmd: &ClipboardCommand, wid: u32) -> AppEvent {
    match cmd {
        ClipboardCommand::Copy {
            target,
            selection_text,
        } => AppEvent::Copy(ClipboardEventData {
            window_id: wid,
            node_id: *target,
            selection_text: Some(selection_text.clone()),
            clipboard_text: None,
        }),
        ClipboardCommand::Cut {
            target,
            selection_text,
            ..
        } => AppEvent::Cut(ClipboardEventData {
            window_id: wid,
            node_id: *target,
            selection_text: Some(selection_text.clone()),
            clipboard_text: None,
        }),
        ClipboardCommand::Paste {
            target,
            clipboard_text,
            ..
        } => AppEvent::Paste(ClipboardEventData {
            window_id: wid,
            node_id: *target,
            selection_text: None,
            clipboard_text: clipboard_text.clone(),
        }),
    }
}

/// Apply the default clipboard action. Returns (needs_redraw, follow_up_events).
pub fn apply_clipboard_command(
    cmd: ClipboardCommand,
    dom: &mut ElementTree,
    wid: u32,
    clipboard: &mut SystemClipboard,
) -> (bool, Vec<AppEvent>) {
    let mut events = Vec::new();
    let mut needs_redraw = false;

    match cmd {
        ClipboardCommand::Copy { selection_text, .. } => {
            if let Err(e) = clipboard.write_text(&selection_text) {
                eprintln!("[uzumaki] clipboard write error: {e}");
            }
        }
        ClipboardCommand::Cut {
            target,
            selection_text,
            is_input,
        } => {
            if let Err(e) = clipboard.write_text(&selection_text) {
                eprintln!("[uzumaki] clipboard write error: {e}");
            }
            if is_input
                && let Some(target_id) = target
                && let Some(node) = dom.nodes.get_mut(target_id)
                && let Some(is) = node.behavior.as_input_mut()
                && let Some((_cut_text, _edit)) = is.cut_selected_text()
            {
                let value = is.model.text();
                events.push(AppEvent::Input(InputEventData {
                    window_id: wid,
                    node_id: target_id,
                    value,
                    input_type: "deleteByCut".to_string(),
                    data: None,
                }));
                needs_redraw = true;
            }
            // For view selections, cut is a no-op on the content
        }
        ClipboardCommand::Paste {
            target,
            clipboard_text,
            is_input,
        } => {
            if is_input
                && let (Some(target_id), Some(text)) = (target, clipboard_text)
                && let Some(node) = dom.nodes.get_mut(target_id)
                && let Some(is) = node.behavior.as_input_mut()
                && let Some(_edit) = is.paste_text(&text)
            {
                let value = is.model.text();
                events.push(AppEvent::Input(InputEventData {
                    window_id: wid,
                    node_id: target_id,
                    value,
                    input_type: "insertFromPaste".to_string(),
                    data: Some(text),
                }));
                needs_redraw = true;
            }
            // For view selections, paste is a no-op
        }
    }

    (needs_redraw, events)
}

/// Find the previous word boundary from a flat grapheme index in a text select run.
fn prev_word_boundary_in_run(
    dom: &ElementTree,
    root_id: crate::element::NodeId,
    flat_idx: usize,
) -> usize {
    let Some(run) = dom
        .selectable_text_runs
        .iter()
        .find(|r| r.root_id == root_id)
    else {
        return flat_idx;
    };
    let graphemes: Vec<&str> =
        unicode_segmentation::UnicodeSegmentation::graphemes(run.flat_text.as_str(), true)
            .collect();
    if flat_idx == 0 {
        return 0;
    }
    let is_word = |g: &str| {
        g.chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
    };
    let mut i = flat_idx;
    // Skip whitespace/non-word backwards
    while i > 0 && !is_word(graphemes[i - 1]) {
        i -= 1;
    }
    // Skip word chars backwards
    while i > 0 && is_word(graphemes[i - 1]) {
        i -= 1;
    }
    i
}

/// Find the next word boundary from a flat grapheme index in a text select run.
fn next_word_boundary_in_run(
    dom: &ElementTree,
    root_id: crate::element::NodeId,
    flat_idx: usize,
) -> usize {
    let Some(run) = dom
        .selectable_text_runs
        .iter()
        .find(|r| r.root_id == root_id)
    else {
        return flat_idx;
    };
    let graphemes: Vec<&str> =
        unicode_segmentation::UnicodeSegmentation::graphemes(run.flat_text.as_str(), true)
            .collect();
    let len = graphemes.len();
    if flat_idx >= len {
        return len;
    }
    let is_word = |g: &str| {
        g.chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
    };
    let mut i = flat_idx;
    // Skip word chars forward
    while i < len && is_word(graphemes[i]) {
        i += 1;
    }
    // Skip whitespace/non-word forward
    while i < len && !is_word(graphemes[i]) {
        i += 1;
    }
    i
}

pub fn handle_mouse_wheel(dom: &mut ElementTree, scroll_delta_y: f64) -> bool {
    let mut needs_redraw = false;
    let Some((mx, my)) = dom.hit_state.mouse_position else {
        return false;
    };

    const SCROLL_LOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(150);

    let locked_target = dom.scroll_lock.and_then(|(nid, t)| {
        if t.elapsed() < SCROLL_LOCK_TIMEOUT {
            dom.scroll_thumbs
                .iter()
                .find(|tr| tr.node_id == nid && tr.view_bounds.contains(mx, my))
                .map(|_| nid)
        } else {
            None
        }
    });

    let target = if let Some(nid) = locked_target {
        dom.scroll_lock = Some((nid, std::time::Instant::now()));
        Some(nid)
    } else {
        let mut found: Option<crate::element::NodeId> = None;
        for thumb_rect in dom.scroll_thumbs.iter() {
            if thumb_rect.view_bounds.contains(mx, my) {
                found = Some(thumb_rect.node_id);
                break;
            }
        }
        if let Some(nid) = found {
            dom.scroll_lock = Some((nid, std::time::Instant::now()));
        }
        found
    };

    if let Some(nid) = target {
        let scroll_info = dom
            .scroll_thumbs
            .iter()
            .find(|t| t.node_id == nid)
            .map(|t| (t.content_height, t.visible_height));
        if let Some((content_h, visible_h)) = scroll_info {
            let max_scroll = (content_h - visible_h).max(0.0);
            if let Some(node) = dom.nodes.get_mut(nid) {
                if let Some(ss) = &mut node.scroll_state {
                    ss.scroll_offset_y =
                        (ss.scroll_offset_y - scroll_delta_y as f32).clamp(0.0, max_scroll);
                } else if let Some(is) = node.behavior.as_input_mut() {
                    is.scroll_offset_y =
                        (is.scroll_offset_y - scroll_delta_y as f32).clamp(0.0, max_scroll);
                }
            }
            needs_redraw = true;
        }
    }

    needs_redraw
}
