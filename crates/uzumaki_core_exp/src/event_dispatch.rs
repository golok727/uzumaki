use serde::Serialize;
use winit::keyboard::{Key, NamedKey};

use crate::element::{Dom, NodeId, ScrollDragState};
use crate::input;
use crate::window::Window;

// ── Event data structs ───────────────────────────────────────────────

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
    pub key: String,
    pub code: String,
    pub key_code: u32,
    pub modifiers: u32,
    pub repeat: bool,
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
    HotReload,
}

pub fn handle_redraw(
    dom: &mut Dom,
    handle: &mut Window,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) {
    // Update scroll offset for focused input before paint
    if let Some(focused_id) = dom.focused_node {
        let scroll_info = dom.nodes.get(focused_id).and_then(|node| {
            node.behavior.as_input().map(|is| {
                let display_text = is.display_text();
                let font_size = node.style.text.font_size;
                let padding = node.style.padding.left;
                let input_padding = if padding > 0.0 { padding } else { 8.0 };
                let pt = node.style.padding.top;
                let top_pad = if pt > 0.0 { pt } else { 4.0 };
                let cursor_pos = is.selection.active;
                let taffy_node = node.taffy_node;
                (
                    display_text,
                    font_size,
                    input_padding,
                    top_pad,
                    cursor_pos,
                    taffy_node,
                )
            })
        });
        if let Some((display_text, font_size, input_padding, top_pad, cursor_pos, taffy_node)) =
            scroll_info
        {
            let is_multiline = dom
                .nodes
                .get(focused_id)
                .and_then(|n| n.behavior.as_input())
                .map(|is| is.multiline)
                .unwrap_or(false);

            let (input_width, input_height) = dom
                .taffy
                .layout(taffy_node)
                .map(|l| {
                    (
                        l.size.width as f32 - input_padding * 2.0,
                        l.size.height as f32,
                    )
                })
                .unwrap_or((200.0, 100.0));

            if is_multiline {
                let positions = handle.text_renderer.grapheme_positions_2d(
                    &display_text,
                    font_size,
                    Some(input_width),
                );
                let cursor_y = if cursor_pos < positions.len() {
                    positions[cursor_pos].y
                } else {
                    positions.last().map(|p| p.y).unwrap_or(0.0)
                };
                let line_height = (font_size * 1.2).round();
                if let Some(node) = dom.nodes.get_mut(focused_id) {
                    if let Some(is) = node.behavior.as_input_mut() {
                        is.update_scroll_y(cursor_y, line_height, input_height - top_pad * 2.0);
                    }
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
                if let Some(node) = dom.nodes.get_mut(focused_id) {
                    if let Some(is) = node.behavior.as_input_mut() {
                        is.update_scroll(cursor_x, input_width);
                    }
                }
            }
        }
    }

    handle.paint_and_present(device, queue, dom);
}

// ── Cursor moved ─────────────────────────────────────────────────────

pub fn handle_cursor_moved(
    dom: &mut Dom,
    handle: &mut Window,
    position: winit::dpi::PhysicalPosition<f64>,
    mouse_buttons: u8,
) -> bool {
    let mut needs_redraw = false;
    let scale = handle.winit_window.scale_factor();
    let logical_x = position.x / scale;
    let logical_y = position.y / scale;
    let old_top = dom.hit_state.top_hit;
    dom.update_hit_test(logical_x, logical_y);
    if old_top != dom.hit_state.top_hit {
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
                            .map(|l| l.size.width as f32 - input_padding as f32 * 2.0)
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

                if let Some(node) = dom.nodes.get_mut(drag_nid) {
                    if let Some(is) = node.behavior.as_input_mut() {
                        is.selection.active = grapheme_idx;
                        is.reset_blink();
                    }
                }
                needs_redraw = true;
            }
        }
    }

    needs_redraw
}

// ── Mouse input ──────────────────────────────────────────────────────

pub fn handle_mouse_input(
    dom: &mut Dom,
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
                .and_then(|n| n.scroll_state.as_ref())
                .map(|ss| ss.scroll_offset_y)
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
    let js_target = dom
        .hit_state
        .top_hit
        .and_then(|hid| dom.hitbox_store.get(hid))
        .map(|hb| hb.node_id);

    match btn_state {
        ElementState::Pressed => {
            let top = dom.hit_state.top_hit;
            dom.set_active(top);
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

                    // TODO thriple click
                    // Double-click detection
                    let now = std::time::Instant::now();
                    let is_double_click = dom.last_click_node == Some(nid)
                        && dom
                            .last_click_time
                            .map_or(false, |t| now.duration_since(t).as_millis() < 400);
                    dom.last_click_time = Some(now);
                    dom.last_click_node = Some(nid);

                    // Focus if not already focused
                    if old_focus != Some(nid) {
                        if let Some(old_id) = old_focus {
                            if let Some(old_node) = dom.nodes.get_mut(old_id) {
                                if let Some(is) = old_node.behavior.as_input_mut() {
                                    is.focused = false;
                                }
                            }
                            events.push(AppEvent::Blur(FocusEventData {
                                window_id: wid,
                                node_id: old_id,
                            }));
                        }
                        dom.focused_node = Some(nid);
                        if let Some(node) = dom.nodes.get_mut(nid) {
                            if let Some(is) = node.behavior.as_input_mut() {
                                is.focused = true;
                                is.reset_blink();
                            }
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
                                    .map(|l| l.size.width as f32 - input_padding as f32 * 2.0)
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

                        if let Some(node) = dom.nodes.get_mut(nid) {
                            if let Some(is) = node.behavior.as_input_mut() {
                                if is_double_click {
                                    let (ws, we) = is.word_at(grapheme_idx);
                                    is.selection.anchor = ws;
                                    is.selection.active = we;
                                } else {
                                    is.selection.set_cursor(grapheme_idx);
                                }
                                is.reset_blink();
                            }
                        }
                    }

                    dom.dragging_input = Some(nid);
                } else {
                    // Clicked non-input: blur focused
                    if let Some(old_id) = old_focus {
                        if let Some(old_node) = dom.nodes.get_mut(old_id) {
                            if let Some(is) = old_node.behavior.as_input_mut() {
                                is.focused = false;
                            }
                        }
                        dom.focused_node = None;
                        events.push(AppEvent::Blur(FocusEventData {
                            window_id: wid,
                            node_id: old_id,
                        }));
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
            if let Some(active) = dom.hit_state.active_hitbox {
                if dom.hit_state.is_hovered(active) {
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
            }
            dom.set_active(None);
            dom.dragging_input = None;
            needs_redraw = true;
        }
    }

    (needs_redraw, events)
}

// handles the keyboard input and returns if redraw is needed and events to dispatch (todo this is bad we should sync emit from js side)
pub fn handle_keyboard_input(
    dom: &mut Dom,
    _handle: &mut Window,
    wid: u32,
    key_event: &winit::event::KeyEvent,
    modifiers: u32,
) -> (bool, Vec<AppEvent>) {
    // todo split this function TT
    use winit::event::ElementState;
    use winit::keyboard::PhysicalKey;

    let mut needs_redraw = false;
    let mut events: Vec<AppEvent> = Vec::new();

    // F5 hot reload
    // todo we should do this after the event listners are done also enable only in dev
    if key_event.state == ElementState::Pressed && key_event.logical_key == Key::Named(NamedKey::F5)
    {
        events.push(AppEvent::HotReload);
        return (needs_redraw, events);
    }

    // Route keyboard to focused input if present
    let mut handled_by_input = false;

    if key_event.state == ElementState::Pressed {
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
                    handled_by_input = true;
                } else {
                    // Non-vertical key: delegate to InputState::handle_key
                    if let Some(input_state) = node.behavior.as_input_mut() {
                        let result = input_state.handle_key(&key_event.logical_key, modifiers);
                        match result {
                            input::KeyResult::Edit(edit) => {
                                let value = input_state.model.text();
                                let input_type = match edit.kind {
                                    input::EditKind::Insert => "insertText",
                                    input::EditKind::DeleteBackward => "deleteContentBackward",
                                    input::EditKind::DeleteForward => "deleteContentForward",
                                    input::EditKind::DeleteWordBackward => "deleteWordBackward",
                                    input::EditKind::DeleteWordForward => "deleteWordForward",
                                };
                                events.push(AppEvent::Input(InputEventData {
                                    window_id: wid,
                                    node_id: focused_id,
                                    value,
                                    input_type: input_type.to_string(),
                                    data: edit.inserted,
                                }));
                                needs_redraw = true;
                                handled_by_input = true;
                            }
                            input::KeyResult::Blur => {
                                input_state.focused = false;
                                new_focus = None;
                                // dom.focused_node = None;
                                events.push(AppEvent::Blur(FocusEventData {
                                    window_id: wid,
                                    node_id: focused_id,
                                }));
                                needs_redraw = true;
                                handled_by_input = true;
                            }
                            input::KeyResult::Handled => {
                                needs_redraw = true;
                                handled_by_input = true;
                            }
                            input::KeyResult::Ignored => {}
                        }
                    }
                }
                new_focus
            })
            .flatten();

        dom.focused_node = new_focus;
    }

    if !handled_by_input {
        let key_str = match &key_event.logical_key {
            Key::Character(c) => c.to_string(),
            Key::Named(named) => format!("{:?}", named),
            _ => return (needs_redraw, events),
        };

        let code_str = match key_event.physical_key {
            PhysicalKey::Code(kc) => format!("{:?}", kc),
            _ => String::new(),
        };

        let data = KeyEventData {
            window_id: wid,
            key: key_str,
            code: code_str,
            key_code: 0,
            modifiers,
            repeat: key_event.repeat,
        };

        match key_event.state {
            ElementState::Pressed => events.push(AppEvent::KeyDown(data)),
            ElementState::Released => events.push(AppEvent::KeyUp(data)),
        }
    }

    (needs_redraw, events)
}

pub fn handle_mouse_wheel(dom: &mut Dom, scroll_delta_y: f64) -> bool {
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
                }
            }
            needs_redraw = true;
        }
    }

    needs_redraw
}
