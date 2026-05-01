use serde::Serialize;
use winit::keyboard::{Key, NamedKey};

use crate::clipboard::SystemClipboard;
use crate::element::{ScrollDragState, UzNodeId};
use crate::input::{self, KeyResult};
use crate::selection::{Affinity, SelectionEndpoint, TextSelection};
use crate::style::TextStyle;
use crate::text::{apply_text_style_to_editor, secure_cursor_geometry};
use crate::ui::UIState;
use crate::window::Window;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseEventData {
    pub window_id: u32,
    pub node_id: UzNodeId,
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
    pub node_id: Option<UzNodeId>,
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
    pub node_id: UzNodeId,
    pub input_type: String,
    pub data: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusEventData {
    pub window_id: u32,
    pub node_id: UzNodeId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardEventData {
    pub window_id: u32,
    pub node_id: Option<UzNodeId>,
    pub selection_text: Option<String>,
    pub clipboard_text: Option<String>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AppEvent {
    // move all this to UzUIEvent ?
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
    #[serde(rename = "windowClose")]
    WindowClose(WindowLoadEventData),
    HotReload,
}

pub fn handle_redraw(
    dom: &mut UIState,
    handle: &mut Window,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) {
    handle.paint_and_present(device, queue, dom);
}

pub struct FocusedInputLayoutMeta {
    pub taffy_x: f64,
    pub taffy_y: f64,
    pub input_padding: f32,
    pub top_pad: f32,
    pub multiline: bool,
    pub text_style: TextStyle,
    pub input_width: f32,
    pub input_height: f32,
}

pub fn input_layout_meta(dom: &UIState, focused_id: UzNodeId) -> Option<FocusedInputLayoutMeta> {
    let node = dom.nodes.get(focused_id)?;
    let is = node.as_text_input()?;
    let input_padding = node.style.padding.left;
    let top_pad = node.style.padding.top;
    let pad_h = node.style.padding.left + node.style.padding.right;
    let text_style = node.style.text.clone();
    let hb = node
        .interactivity
        .hitbox_id
        .and_then(|hid| dom.hitbox_store.get(hid))?;
    let layout = dom.taffy.layout(node.taffy_node).ok()?;
    Some(FocusedInputLayoutMeta {
        taffy_x: hb.bounds.x,
        taffy_y: hb.bounds.y,
        input_padding,
        top_pad,
        multiline: is.multiline,
        text_style,
        input_width: (layout.size.width - pad_h).max(0.0),
        input_height: layout.size.height,
    })
}

fn sync_focused_input_cursor(
    dom: &mut UIState,
    handle: &mut Window,
    focused_id: UzNodeId,
    meta: &FocusedInputLayoutMeta,
) -> Option<(parley::BoundingBox, f32, f32)> {
    let node = dom.nodes.get_mut(focused_id)?;
    let is = node.as_text_input_mut()?;
    apply_text_style_to_editor(&mut is.editor, &meta.text_style);
    is.editor.set_width(if meta.multiline {
        Some(meta.input_width)
    } else {
        None
    });
    is.editor.refresh_layout(
        &mut handle.text_renderer.font_ctx,
        &mut handle.text_renderer.layout_ctx,
    );
    let cursor_rect = if is.secure {
        secure_cursor_geometry(&is.editor, 1.5, &meta.text_style, &mut handle.text_renderer)
    } else {
        is.editor.cursor_geometry(1.5)
    }?;
    Some((cursor_rect, is.scroll_offset, is.scroll_offset_y))
}

fn set_ime_cursor_area(
    handle: &mut Window,
    meta: &FocusedInputLayoutMeta,
    ime_area: &parley::BoundingBox,
    _scroll_offset_x: f32,
    scroll_offset_y: f32,
) {
    let line_height = (meta.text_style.font_size * meta.text_style.line_height).round() as f64;
    let text_origin_x = meta.taffy_x + meta.input_padding as f64;
    let text_origin_y = if meta.multiline {
        meta.taffy_y + meta.top_pad as f64 - scroll_offset_y as f64
    } else {
        meta.taffy_y + ((meta.input_height as f64 - line_height) / 2.0).max(0.0)
    };
    let position =
        winit::dpi::LogicalPosition::new(text_origin_x + ime_area.x0, text_origin_y + ime_area.y0);
    let size = winit::dpi::LogicalSize::new(
        (ime_area.x1 - ime_area.x0).max(24.0) as f32,
        (ime_area.y1 - ime_area.y0).max(1.0) as f32,
    );
    handle.winit_window.set_ime_cursor_area(position, size);
}

pub fn update_ime_cursor_area(dom: &mut UIState, handle: &mut Window) {
    let Some(focused_id) = dom.focused_node else {
        return;
    };
    let Some(meta) = input_layout_meta(dom, focused_id) else {
        return;
    };
    let Some((_cursor_rect, scroll_offset_x, scroll_offset_y)) =
        sync_focused_input_cursor(dom, handle, focused_id, &meta)
    else {
        return;
    };
    let Some(node) = dom.nodes.get(focused_id) else {
        return;
    };
    let Some(is) = node.as_text_input() else {
        return;
    };
    let ime_area = is.editor.ime_cursor_area();
    set_ime_cursor_area(handle, &meta, &ime_area, scroll_offset_x, scroll_offset_y);
}

/// Scroll the focused input so the cursor stays visible.
/// Call this after any action that moves the cursor (key press, click, drag).
pub fn scroll_input_to_cursor(dom: &mut UIState, handle: &mut Window) {
    let Some(focused_id) = dom.focused_node else {
        return;
    };
    let Some(meta) = input_layout_meta(dom, focused_id) else {
        return;
    };

    if let Some(node) = dom.nodes.get_mut(focused_id)
        && let Some(is) = node.as_text_input_mut()
    {
        apply_text_style_to_editor(&mut is.editor, &meta.text_style);
        is.editor.set_width(if meta.multiline {
            Some(meta.input_width)
        } else {
            None
        });
        is.editor.refresh_layout(
            &mut handle.text_renderer.font_ctx,
            &mut handle.text_renderer.layout_ctx,
        );
        let cursor_rect = if is.secure {
            secure_cursor_geometry(&is.editor, 1.5, &meta.text_style, &mut handle.text_renderer)
        } else {
            is.editor.cursor_geometry(1.5)
        };
        if let Some(rect) = cursor_rect {
            if meta.multiline {
                let line_height = (meta.text_style.font_size * meta.text_style.line_height).round();
                is.update_scroll_y(
                    rect.y0 as f32,
                    line_height,
                    meta.input_height - meta.top_pad * 2.0,
                );
            } else {
                is.update_scroll(rect.x0 as f32, meta.input_width);
            }
        }
    }

    if let Some((_cursor_rect, scroll_offset_x, scroll_offset_y)) =
        sync_focused_input_cursor(dom, handle, focused_id, &meta)
        && let Some(node) = dom.nodes.get(focused_id)
        && let Some(is) = node.as_text_input()
    {
        let ime_area = is.editor.ime_cursor_area();
        set_ime_cursor_area(handle, &meta, &ime_area, scroll_offset_x, scroll_offset_y);
    }
}

pub fn handle_cursor_moved(
    dom: &mut UIState,
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
            } else if let Some(is) = node.as_text_input_mut() {
                is.scroll_offset_y = new_offset.clamp(0.0, max);
            }
        }
        needs_redraw = true;
    }

    // Input drag selection
    if mouse_buttons & 1 != 0 {
        if let Some(drag_nid) = dom.dragging_input {
            let hit_info = dom.nodes.get(drag_nid).and_then(|node| {
                let is = node.as_text_input()?;
                let input_padding = node.style.padding.left as f64;
                let top_pad = node.style.padding.top;
                let hb = node
                    .interactivity
                    .hitbox_id
                    .and_then(|hid| dom.hitbox_store.get(hid))?
                    .bounds;
                Some((
                    is.scroll_offset,
                    is.scroll_offset_y,
                    is.multiline,
                    input_padding,
                    top_pad,
                    hb,
                ))
            });

            if let Some((
                scroll_offset,
                scroll_offset_y,
                is_multiline,
                input_padding,
                top_pad,
                hb,
            )) = hit_info
            {
                let relative_x = if is_multiline {
                    (logical_x - hb.x - input_padding) as f32
                } else {
                    (logical_x - hb.x - input_padding) as f32 + scroll_offset
                };
                let relative_y = (logical_y - hb.y) as f32 + scroll_offset_y - top_pad;

                // Apply styles/width so the driver's layout accounts for wrapping
                if let Some(meta) = input_layout_meta(dom, drag_nid)
                    && let Some(node) = dom.nodes.get_mut(drag_nid)
                    && let Some(is) = node.as_text_input_mut()
                {
                    apply_text_style_to_editor(&mut is.editor, &meta.text_style);
                    is.editor.set_width(if meta.multiline {
                        Some(meta.input_width)
                    } else {
                        None
                    });
                }

                if let Some(node) = dom.nodes.get_mut(drag_nid)
                    && let Some(is) = node.as_text_input_mut()
                {
                    is.extend_selection_to_point(relative_x, relative_y, &mut handle.text_renderer);
                }

                scroll_input_to_cursor(dom, handle);
                needs_redraw = true;
            }
        }

        // View text selection drag
        if let Some(root_id) = dom.dragging_view_selection
            && let Some(hit) = hit_text_in_run(
                dom,
                &mut handle.text_renderer,
                root_id,
                logical_x,
                logical_y,
            )
        {
            if dom.selection_root(&dom.text_selection) == Some(root_id) {
                dom.text_selection.focus = Some(hit.endpoint);
            }
            needs_redraw = true;
        }
    }

    let cursor = dom
        .hit_state
        .top_node
        .map(|id| dom.resolve_cursor(id))
        .unwrap_or(crate::cursor::UzCursorIcon::Default);
    handle.set_cursor(cursor);

    needs_redraw
}

/// Hit-test a mouse position against all text nodes in a textSelect run.
/// Returns the matched text node and flat grapheme index if a suitable text node is found.
struct TextRunHit {
    node_id: UzNodeId,
    endpoint: SelectionEndpoint,
}

fn hit_text_in_run(
    dom: &UIState,
    text_renderer: &mut crate::text::TextRenderer,
    root_id: crate::element::UzNodeId,
    mx: f64,
    my: f64,
) -> Option<TextRunHit> {
    use crate::style::Bounds;

    let run = dom
        .selectable_text_runs
        .iter()
        .find(|r| r.root_id == root_id)?;

    // Find the text node closest to mouse position
    let mut best: Option<(crate::element::UzNodeId, f64, Bounds)> = None;
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
    let node = dom.nodes.get(node_id)?;
    let text = node.as_text_node()?;

    if text.content.is_empty() {
        return Some(TextRunHit {
            node_id,
            endpoint: SelectionEndpoint::new(node_id, 0, Affinity::Downstream),
        });
    }

    let relative_x = (mx - bounds.x) as f32;
    let relative_y = (my - bounds.y) as f32;
    let (offset, affinity) = text_renderer.hit_to_text_position(
        &text.content,
        &node.style.text,
        Some(bounds.width as f32),
        relative_x,
        relative_y,
    );

    Some(TextRunHit {
        node_id,
        endpoint: SelectionEndpoint::new(node_id, offset, affinity),
    })
}

fn point_to_rect_dist(px: f64, py: f64, bounds: &crate::style::Bounds) -> f64 {
    let cx = px.clamp(bounds.x, bounds.x + bounds.width);
    let cy = py.clamp(bounds.y, bounds.y + bounds.height);
    let dx = px - cx;
    let dy = py - cy;
    (dx * dx + dy * dy).sqrt()
}

fn text_range_at_point(
    dom: &UIState,
    text_renderer: &mut crate::text::TextRenderer,
    node_id: UzNodeId,
    mx: f64,
    my: f64,
    select_line: bool,
) -> Option<(SelectionEndpoint, SelectionEndpoint)> {
    let (_run, _entry) = dom.find_run_entry_for_node(node_id)?;
    let node = dom.nodes.get(node_id)?;
    let text = node.as_text_node()?;
    let bounds = node
        .interactivity
        .hitbox_id
        .and_then(|hid| dom.hitbox_store.get(hid))
        .map(|hb| hb.bounds)?;

    if text.content.is_empty() {
        let endpoint = SelectionEndpoint::new(node_id, 0, Affinity::Downstream);
        return Some((endpoint, endpoint));
    }

    let rel_x = (mx - bounds.x) as f32;
    let rel_y = (my - bounds.y) as f32;
    let (local_start, local_end) = if select_line {
        text_renderer.line_byte_range_at_point(
            &text.content,
            &node.style.text,
            Some(bounds.width as f32),
            rel_x,
            rel_y,
        )
    } else {
        text_renderer.word_byte_range_at_point(
            &text.content,
            &node.style.text,
            Some(bounds.width as f32),
            rel_x,
            rel_y,
        )
    };

    Some((
        SelectionEndpoint::new(node_id, local_start, Affinity::Downstream),
        SelectionEndpoint::new(node_id, local_end, Affinity::Upstream),
    ))
}

pub fn handle_mouse_input(
    dom: &mut UIState,
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
                        .or_else(|| n.as_text_input().map(|is| is.scroll_offset_y))
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
                    .map(|n| n.is_text_input())
                    .unwrap_or(false);
                let clicked_is_checkbox = js_target
                    .and_then(|nid| dom.nodes.get(nid))
                    .map(|n| n.is_checkbox_input())
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
                    let click_info = {
                        let node = &dom.nodes[nid];
                        let is = node.as_text_input().unwrap();
                        let input_padding = node.style.padding.left as f64;
                        let top_pad = node.style.padding.top;
                        let hb = node
                            .interactivity
                            .hitbox_id
                            .and_then(|hid| dom.hitbox_store.get(hid))
                            .map(|hb| hb.bounds);
                        (
                            is.scroll_offset,
                            is.scroll_offset_y,
                            is.multiline,
                            input_padding,
                            top_pad,
                            hb,
                        )
                    };
                    let (
                        scroll_offset,
                        scroll_offset_y,
                        is_multiline,
                        input_padding,
                        top_pad,
                        hitbox_bounds,
                    ) = click_info;

                    if let Some(hb) = hitbox_bounds {
                        let relative_x = if is_multiline {
                            (mx - hb.x - input_padding) as f32
                        } else {
                            (mx - hb.x - input_padding) as f32 + scroll_offset
                        };
                        let relative_y = (my - hb.y) as f32 + scroll_offset_y - top_pad;

                        dom.focus_element(nid);

                        // Apply styles/width so hit-testing accounts for wrapping
                        if let Some(meta) = input_layout_meta(dom, nid)
                            && let Some(node) = dom.nodes.get_mut(nid)
                            && let Some(is) = node.as_text_input_mut()
                        {
                            apply_text_style_to_editor(&mut is.editor, &meta.text_style);
                            is.editor.set_width(if meta.multiline {
                                Some(meta.input_width)
                            } else {
                                None
                            });
                        }

                        if let Some(node) = dom.nodes.get_mut(nid)
                            && let Some(is) = node.as_text_input_mut()
                        {
                            let renderer = &mut handle.text_renderer;
                            match dom.click_count {
                                2 => is.select_word_at_point(relative_x, relative_y, renderer),
                                3 => is.select_line_at_point(relative_x, relative_y, renderer),
                                4 => is.select_all(renderer),
                                _ => is.move_to_point(relative_x, relative_y, renderer),
                            }
                            is.reset_blink();
                        }
                    }

                    scroll_input_to_cursor(dom, handle);
                    dom.dragging_input = Some(nid);
                } else if clicked_is_checkbox {
                    let nid = js_target.unwrap();

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

                    dom.clear_selection();
                    dom.focused_node = Some(nid);
                } else {
                    // Clicked non-input: blur focused input
                    if let Some(old_id) = old_focus {
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
                            events.push(AppEvent::Blur(FocusEventData {
                                window_id: wid,
                                node_id: old_id,
                            }));
                        }

                        // Find the run this text node belongs to
                        let run_root = dom.find_run_entry_for_node(nid).map(|(run, _)| run.root_id);

                        if let Some(hit) = run_root.and_then(|root_id| {
                            hit_text_in_run(dom, &mut handle.text_renderer, root_id, mx, my)
                        }) {
                            let run_root = run_root.unwrap_or(nid);
                            let endpoint = hit.endpoint;

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
                                    if let Some((start, end)) = text_range_at_point(
                                        dom,
                                        &mut handle.text_renderer,
                                        hit.node_id,
                                        mx,
                                        my,
                                        false,
                                    ) {
                                        dom.set_selection(TextSelection::new(start, end));
                                    }
                                }
                                3 => {
                                    if let Some((start, end)) = text_range_at_point(
                                        dom,
                                        &mut handle.text_renderer,
                                        hit.node_id,
                                        mx,
                                        my,
                                        true,
                                    ) {
                                        dom.set_selection(TextSelection::new(start, end));
                                    }
                                }
                                4 => {
                                    // Select all text in the run
                                    if let Some(run) = dom
                                        .selectable_text_runs
                                        .iter()
                                        .find(|r| r.root_id == run_root)
                                        && let (Some(start), Some(end)) = (
                                            dom.endpoint_from_flat_index(
                                                run_root,
                                                0,
                                                Affinity::Downstream,
                                            ),
                                            dom.endpoint_from_flat_index(
                                                run_root,
                                                run.total_graphemes,
                                                Affinity::Upstream,
                                            ),
                                        )
                                    {
                                        dom.set_selection(TextSelection::new(start, end));
                                    }
                                }
                                _ => {
                                    // Single click: place cursor
                                    dom.set_selection(TextSelection::new(endpoint, endpoint));
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
                if mouse_button == crate::interactivity::MouseButton::Left
                    && let Some(target) = js_target
                    && let Some(node) = dom.nodes.get_mut(target)
                    && let Some(checked) = node.as_checkbox_input_mut()
                {
                    *checked = !*checked;
                    events.push(AppEvent::Input(InputEventData {
                        window_id: wid,
                        node_id: target,
                        input_type: "toggle".to_string(),
                        data: None,
                    }));
                }
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
    dom: &UIState,
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
    dom: &mut UIState,
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

    // Apply text styles and width to the editor BEFORE handling the key,
    // so parley's driver has the correct layout for cursor movement in wrapped text.
    if let Some(meta) = dom.focused_node.and_then(|id| input_layout_meta(dom, id))
        && let Some(node) = dom.focused_node.and_then(|id| dom.nodes.get_mut(id))
        && let Some(is) = node.as_text_input_mut()
    {
        apply_text_style_to_editor(&mut is.editor, &meta.text_style);
        is.editor.set_width(if meta.multiline {
            Some(meta.input_width)
        } else {
            None
        });
    }

    let new_focus = dom
        .with_focused_node(|node, focused_id| {
            let mut new_focus = Some(focused_id);

            if let Some(input_state) = node.as_text_input_mut() {
                let result = input_state.handle_key(
                    &key_event.logical_key,
                    modifiers,
                    &mut handle.text_renderer,
                );
                match result {
                    KeyResult::Edit(edit) => {
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
                            input_type: input_type.to_string(),
                            data: edit.inserted,
                        }));
                        needs_redraw = true;
                    }
                    KeyResult::Blur => {
                        new_focus = None;
                        events.push(AppEvent::Blur(FocusEventData {
                            window_id: wid,
                            node_id: focused_id,
                        }));
                        needs_redraw = true;
                    }
                    KeyResult::Handled => {
                        needs_redraw = true;
                    }
                    KeyResult::Ignored => {}
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

pub fn handle_key_for_checkbox(
    dom: &mut UIState,
    wid: u32,
    key_event: &winit::event::KeyEvent,
) -> (bool, Vec<AppEvent>) {
    use winit::event::ElementState;

    if key_event.state != ElementState::Pressed {
        return (false, Vec::new());
    }

    let should_toggle = matches!(
        &key_event.logical_key,
        Key::Named(NamedKey::Space) | Key::Named(NamedKey::Enter)
    );
    if !should_toggle {
        return (false, Vec::new());
    }

    let Some(focused_id) = dom.focused_node else {
        return (false, Vec::new());
    };
    let Some(node) = dom.nodes.get_mut(focused_id) else {
        return (false, Vec::new());
    };
    let Some(checked) = node.as_checkbox_input_mut() else {
        return (false, Vec::new());
    };

    *checked = !*checked;
    (
        true,
        vec![AppEvent::Input(InputEventData {
            window_id: wid,
            node_id: focused_id,
            input_type: "toggle".to_string(),
            data: None,
        })],
    )
}

/// Handle Enter/Space on a focused button-like element (focusable view that's
/// not a text input or checkbox). Fires a synthetic click, mirroring browser
/// behavior on `<button>`.
pub fn handle_key_for_button(
    dom: &mut UIState,
    wid: u32,
    key_event: &winit::event::KeyEvent,
) -> (bool, Vec<AppEvent>) {
    use winit::event::ElementState;

    if key_event.state != ElementState::Pressed {
        return (false, Vec::new());
    }
    if !matches!(
        &key_event.logical_key,
        Key::Named(NamedKey::Enter) | Key::Named(NamedKey::Space)
    ) {
        return (false, Vec::new());
    }

    let Some(focused_id) = dom.focused_node else {
        return (false, Vec::new());
    };
    let Some(node) = dom.nodes.get(focused_id) else {
        return (false, Vec::new());
    };
    if node.is_text_input() || node.is_checkbox_input() || !node.is_focusable() {
        return (false, Vec::new());
    }

    // Synthetic click: use the element's bounds center if we have a hitbox,
    // otherwise (0, 0). The JS handler usually doesn't depend on coords for
    // keyboard activations.
    let (x, y) = node
        .interactivity
        .hitbox_id
        .and_then(|hid| dom.hitbox_store.get(hid))
        .map(|hb| {
            (
                (hb.bounds.x + hb.bounds.width / 2.0) as f32,
                (hb.bounds.y + hb.bounds.height / 2.0) as f32,
            )
        })
        .unwrap_or((0.0, 0.0));

    dom.dispatch_click(x as f64, y as f64, crate::interactivity::MouseButton::Left);

    (
        true,
        vec![AppEvent::Click(MouseEventData {
            window_id: wid,
            node_id: focused_id,
            x,
            y,
            screen_x: x,
            screen_y: y,
            button: 0,
            buttons: 0,
        })],
    )
}

pub struct TabFocusOutcome {
    pub consumed: bool,
    pub needs_redraw: bool,
    pub events: Vec<AppEvent>,
}

/// Handle Tab/Shift-Tab to advance focus to the next/previous focusable
/// element. Tab is always consumed (never inserts a tab character).
pub fn handle_tab_focus(
    dom: &mut UIState,
    wid: u32,
    key_event: &winit::event::KeyEvent,
    modifiers: u32,
) -> TabFocusOutcome {
    use winit::event::ElementState;

    let mut outcome = TabFocusOutcome {
        consumed: false,
        needs_redraw: false,
        events: Vec::new(),
    };

    if key_event.state != ElementState::Pressed
        || !matches!(&key_event.logical_key, Key::Named(NamedKey::Tab))
    {
        return outcome;
    }

    outcome.consumed = true;

    let shift = modifiers & 4 != 0;
    let change = if shift {
        dom.focus_prev_node()
    } else {
        dom.focus_next_node()
    };
    if let Some(change) = change {
        if let Some(old) = change.old {
            outcome.events.push(AppEvent::Blur(FocusEventData {
                window_id: wid,
                node_id: old,
            }));
        }
        outcome.events.push(AppEvent::Focus(FocusEventData {
            window_id: wid,
            node_id: change.new,
        }));
        outcome.needs_redraw = true;
    }

    outcome
}

/// Handle keyboard shortcuts for view text selection (Shift+Arrows, Ctrl+A, etc.)
/// Called after input-level processing, only when there's no focused input.
/// Returns true if a redraw is needed.
pub fn handle_key_for_view_selection(
    dom: &mut UIState,
    key_event: &winit::event::KeyEvent,
    modifiers: u32,
) -> bool {
    use winit::event::ElementState;

    if key_event.state != ElementState::Pressed {
        return false;
    }

    let Some(sel) = dom.get_text_selection() else {
        return false;
    };

    let Some(root) = dom.selection_root(&sel) else {
        return false;
    };
    let Some(anchor_endpoint) = sel.anchor else {
        return false;
    };
    let Some(focus_endpoint) = sel.focus else {
        return false;
    };
    let Some(active) = dom.flat_index_for_endpoint(focus_endpoint) else {
        return false;
    };

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
            if let Some(focus) =
                dom.endpoint_from_flat_index(root, new_active, Affinity::Downstream)
            {
                dom.set_selection(TextSelection::new(anchor_endpoint, focus));
            }
            true
        }
        Key::Named(NamedKey::ArrowRight) if shift && ctrl => {
            let new_active = next_word_boundary_in_run(dom, root, active);
            if let Some(focus) =
                dom.endpoint_from_flat_index(root, new_active, Affinity::Downstream)
            {
                dom.set_selection(TextSelection::new(anchor_endpoint, focus));
            }
            true
        }
        Key::Named(NamedKey::ArrowLeft) if shift => {
            let new_active = if active > 0 { active - 1 } else { 0 };
            if let Some(focus) =
                dom.endpoint_from_flat_index(root, new_active, Affinity::Downstream)
            {
                dom.set_selection(TextSelection::new(anchor_endpoint, focus));
            }
            true
        }
        Key::Named(NamedKey::ArrowRight) if shift => {
            let new_active = (active + 1).min(run_len);
            if let Some(focus) =
                dom.endpoint_from_flat_index(root, new_active, Affinity::Downstream)
            {
                dom.set_selection(TextSelection::new(anchor_endpoint, focus));
            }
            true
        }
        Key::Named(NamedKey::Home) if shift => {
            if let Some(focus) = dom.endpoint_from_flat_index(root, 0, Affinity::Downstream) {
                dom.set_selection(TextSelection::new(anchor_endpoint, focus));
            }
            true
        }
        Key::Named(NamedKey::End) if shift => {
            if let Some(focus) = dom.endpoint_from_flat_index(root, run_len, Affinity::Upstream) {
                dom.set_selection(TextSelection::new(anchor_endpoint, focus));
            }
            true
        }
        Key::Character(c) if ctrl && (c.as_ref() == "a" || c.as_ref() == "A") => {
            if let (Some(start), Some(end)) = (
                dom.endpoint_from_flat_index(root, 0, Affinity::Downstream),
                dom.endpoint_from_flat_index(root, run_len, Affinity::Upstream),
            ) {
                dom.set_selection(TextSelection::new(start, end));
            }
            true
        }
        _ => false,
    }
}

/// Identifies the target of a clipboard operation.
pub enum ClipboardTarget {
    /// Focused input node.
    Input(UzNodeId),
    /// Non-input text selection root.
    ViewSelection(UzNodeId),
}

/// A resolved clipboard command ready for event dispatch and default action.
pub enum ClipboardCommand {
    Copy {
        target: Option<UzNodeId>,
        selection_text: String,
    },
    Cut {
        target: Option<UzNodeId>,
        selection_text: String,
        is_input: bool,
    },
    Paste {
        target: Option<UzNodeId>,
        clipboard_text: Option<String>,
        is_input: bool,
    },
}

/// Resolve the current clipboard target from DOM state.
fn resolve_clipboard_target(dom: &UIState) -> Option<ClipboardTarget> {
    if let Some(focused_id) = dom.focused_node
        && let Some(node) = dom.nodes.get(focused_id)
        && node.as_text_input().is_some()
    {
        return Some(ClipboardTarget::Input(focused_id));
    }
    if let Some(sel) = dom.get_text_selection()
        && !sel.is_collapsed()
        && let Some(root) = dom.selection_root(&sel)
    {
        return Some(ClipboardTarget::ViewSelection(root));
    }
    None
}

/// Detect whether a key event is a clipboard shortcut and build the corresponding
/// command. Returns `None` if the key is not a clipboard shortcut.
pub fn build_clipboard_command(
    dom: &UIState,
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
                    let is = node.as_text_input()?;
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
                    let is = node.as_text_input()?;
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
                    let is = node.as_text_input()?;
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
    dom: &mut UIState,
    wid: u32,
    clipboard: &mut SystemClipboard,
    text_renderer: &mut crate::text::TextRenderer,
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
                && let Some(is) = node.as_text_input_mut()
                && let Some((_cut_text, _edit)) = is.cut_selected_text(text_renderer)
            {
                events.push(AppEvent::Input(InputEventData {
                    window_id: wid,
                    node_id: target_id,
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
                && let Some(is) = node.as_text_input_mut()
                && let Some(_edit) = is.paste_text(&text, text_renderer)
            {
                events.push(AppEvent::Input(InputEventData {
                    window_id: wid,
                    node_id: target_id,
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
    dom: &UIState,
    root_id: crate::element::UzNodeId,
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
    dom: &UIState,
    root_id: crate::element::UzNodeId,
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

pub fn handle_mouse_wheel(dom: &mut UIState, handle: &mut Window, scroll_delta_y: f64) -> bool {
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
        let mut found: Option<crate::element::UzNodeId> = None;
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
                } else if let Some(is) = node.as_text_input_mut() {
                    is.scroll_offset_y =
                        (is.scroll_offset_y - scroll_delta_y as f32).clamp(0.0, max_scroll);
                }
            }
            needs_redraw = true;
        }
    }

    if needs_redraw {
        update_ime_cursor_area(dom, handle);
    }

    needs_redraw
}
