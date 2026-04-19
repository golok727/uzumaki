use std::collections::HashMap;

use vello::Scene;
use vello::kurbo::{Affine, Rect, RoundedRect, RoundedRectRadii};
use vello::peniko::{Color as VelloColor, Fill};

use crate::element::checkbox::CheckboxRenderInfo;
use crate::element::input::InputRenderInfo;
use crate::element::{InheritedProperties, NodeContext, ScrollThumbRect, UzNodeId};
use crate::style::{Bounds, Color, UzStyle, Visibility};
use crate::text::{GlyphPos2D, TextRenderer};
use crate::ui::UIState;

fn compute_selection_rects(
    positions: &[GlyphPos2D],
    sel_start: usize,
    sel_end: usize,
    _text_w: f64,
    line_height: f64,
) -> Vec<[f64; 4]> {
    if sel_start >= sel_end || positions.is_empty() {
        return vec![];
    }
    let sel_end_idx = sel_end.min(positions.len() - 1);
    let start_x = positions[sel_start].x as f64;
    let end_x = positions[sel_end_idx].x as f64;
    let mut line_ys: Vec<f32> = Vec::new();
    for pos in &positions[sel_start..=sel_end_idx] {
        let y = pos.y;
        if line_ys.last().is_none_or(|&ly| (y - ly).abs() > 1.0) {
            line_ys.push(y);
        }
    }
    let num_lines = line_ys.len();
    let mut rects = Vec::new();
    for (idx, &ly) in line_ys.iter().enumerate() {
        let y = ly as f64;
        let line_end_x = positions
            .iter()
            .filter(|pos| (pos.y - ly).abs() < 1.0)
            .map(|pos| pos.x as f64)
            .fold(0.0, f64::max);
        let (x1, x2) = if num_lines == 1 {
            (start_x, end_x)
        } else if idx == 0 {
            (start_x, line_end_x)
        } else if idx == num_lines - 1 {
            if end_x < 1.0 {
                (0.0, 8.0)
            } else {
                (0.0, end_x)
            }
        } else if line_end_x > 1.0 {
            (0.0, line_end_x)
        } else {
            (0.0, 8.0)
        };

        if x2 > x1 {
            rects.push([x1, y, x2, y + line_height]);
        }
    }
    rects
}

/// Renders an `ElementTree` into a Vello `Scene`. Also rebuilds hitboxes and
/// scroll thumbs as a side effect of walking the tree.
pub struct Painter<'a> {
    dom: &'a mut UIState,
    scene: &'a mut Scene,
    text_renderer: &'a mut TextRenderer,
    scale: f64,
}

impl<'a> Painter<'a> {
    pub fn new(
        dom: &'a mut UIState,
        scene: &'a mut Scene,
        text_renderer: &'a mut TextRenderer,
        scale: f64,
    ) -> Self {
        Self {
            dom,
            scene,
            text_renderer,
            scale,
        }
    }

    pub fn paint(mut self) {
        self.dom.hitbox_store.clear();
        self.dom.scroll_thumbs.clear();
        self.dom.build_text_select_runs();

        if let Some(root) = self.dom.root {
            self.render_tree(root);
        }
    }

    fn render_tree(&mut self, root_id: UzNodeId) {
        let scale = self.scale;

        // Pre-compute per-node selection ranges for text selection painting
        let text_sel_map = self.compute_text_selections_map();

        let mut render_list: Vec<RenderCommand> = Vec::new();
        let mut stack: Vec<StackItem> = vec![StackItem::Visit(
            root_id,
            0.0,
            0.0,
            InheritedProperties::default(),
        )];

        while let Some(item) = stack.pop() {
            match item {
                StackItem::PushClip(rect, s) => {
                    render_list.push(RenderCommand::PushClip(rect, s));
                    continue;
                }
                StackItem::PopClip => {
                    render_list.push(RenderCommand::PopClip);
                    continue;
                }
                StackItem::PaintThumb(info) => {
                    render_list.push(RenderCommand::PaintThumb(info));
                    continue;
                }
                StackItem::Visit(node_id, parent_x, parent_y, parent_inherited) => {
                    // Extract all needed data from the node (immutable borrow scope)
                    let (
                        taffy_node,
                        computed_style,
                        text,
                        input_snapshot,
                        checkbox_snapshot,
                        needs_hitbox,
                        is_scrollable,
                        first_child,
                        inherited,
                    ) = {
                        let node = &self.dom.nodes[node_id];

                        // Resolve inherited properties
                        let mut inherited = parent_inherited.clone();

                        // if not inherited property set the value
                        if let Some(text_selectable) = node.text_selectable().as_value() {
                            inherited.text_selectable = text_selectable;
                        }

                        let taffy_node = node.taffy_node;
                        let computed_style = node.interactivity.compute_style(
                            &node.style,
                            node_id,
                            &self.dom.hit_state,
                        );

                        let text = node.as_text_node().map(|tc| {
                            (
                                tc.content.clone(),
                                computed_style.text.font_size,
                                computed_style.text.color,
                            )
                        });

                        let is_input = node.is_text_input();
                        let input_snapshot = if is_input {
                            let is = node.as_text_input().unwrap();
                            Some((
                                is.display_text(),
                                is.placeholder.clone(),
                                is.focused,
                                is.scroll_offset,
                                is.scroll_offset_y,
                                is.blink_visible(self.dom.window_focused),
                                is.multiline,
                                is.preedit.clone(),
                            ))
                        } else {
                            None
                        };
                        let checkbox_snapshot = if node.is_checkbox_input() {
                            node.as_checkbox_input()
                                .copied()
                                .map(|checked| CheckboxRenderInfo {
                                    checked,
                                    focused: self.dom.focused_node == Some(node_id),
                                })
                        } else {
                            None
                        };
                        // Text nodes inside textSelect views need hitboxes for click-to-select
                        let selectable_text = inherited.text_selectable && node.is_text_node();
                        let needs_hitbox = node.interactivity.needs_hitbox() || selectable_text;
                        let is_scrollable = node.scroll_state.is_some();
                        let first_child = node.first_child;

                        (
                            taffy_node,
                            computed_style,
                            text,
                            input_snapshot,
                            checkbox_snapshot,
                            needs_hitbox,
                            is_scrollable,
                            first_child,
                            inherited,
                        )
                    };

                    if computed_style.visibility == Visibility::Hidden
                        || computed_style.display == crate::style::Display::None
                    {
                        continue;
                    }

                    let Ok(layout) = self.dom.taffy.layout(taffy_node) else {
                        continue;
                    };

                    // Populate InputRenderInfo with geometry from PlainEditor (needs mut access)
                    let input = if let Some((
                        display_text,
                        placeholder,
                        focused,
                        scroll_offset,
                        scroll_offset_y,
                        blink_visible,
                        multiline,
                        preedit_state,
                    )) = input_snapshot
                    {
                        let padding: f32 = 8.0;
                        let text_w = (layout.size.width - padding * 2.0).max(0.0);
                        let node_mut = &mut self.dom.nodes[node_id];
                        let is = node_mut.as_text_input_mut().unwrap();
                        is.set_font_size(computed_style.text.font_size);
                        if multiline {
                            is.set_width(Some(text_w));
                        } else {
                            is.set_width(None);
                        }
                        is.refresh_layout(self.text_renderer);
                        let cursor_rect = if blink_visible || preedit_state.is_some() {
                            is.display_cursor_geometry(1.5, self.text_renderer)
                        } else {
                            None
                        };
                        let selection_rects = is.display_selection_geometry(self.text_renderer);
                        let layout_height =
                            is.editor.try_layout().map(|l| l.height()).unwrap_or(0.0);
                        let preedit = preedit_state.map(|ps| {
                            let font_size = computed_style.text.font_size;
                            let positions =
                                self.text_renderer.grapheme_x_positions(&ps.text, font_size);
                            let width = *positions.last().unwrap_or(&0.0);
                            crate::element::input::PreeditRenderInfo {
                                text: ps.text,
                                cursor_x: ps
                                    .cursor
                                    .map(|(start, _)| {
                                        if start < positions.len() {
                                            positions[start]
                                        } else {
                                            width
                                        }
                                    })
                                    .unwrap_or(width),
                                width,
                            }
                        });
                        Some(InputRenderInfo {
                            display_text,
                            placeholder,
                            font_size: computed_style.text.font_size,
                            text_color: computed_style.text.color,
                            focused,
                            cursor_rect,
                            selection_rects,
                            scroll_offset,
                            scroll_offset_y,
                            blink_visible,
                            multiline,
                            layout_height,
                            preedit,
                        })
                    } else {
                        None
                    };

                    let x = parent_x + layout.location.x as f64;
                    let y = parent_y + layout.location.y as f64;
                    let w = layout.size.width as f64;
                    let h = layout.size.height as f64;

                    // Compute scroll info and clamp offset (mutable borrow is safe now)
                    let scroll_info = if is_scrollable {
                        let content_height = layout.content_size.height;
                        let visible_height = layout.size.height;
                        let max_scroll = (content_height - visible_height).max(0.0);
                        if let Some(ss) = self.dom.nodes[node_id].scroll_state.as_mut()
                            && ss.scroll_offset_y > max_scroll
                        {
                            ss.scroll_offset_y = max_scroll;
                        }
                        let clamped_offset = self.dom.nodes[node_id]
                            .scroll_state
                            .as_ref()
                            .map(|ss| ss.scroll_offset_y)
                            .unwrap_or(0.0);
                        Some((content_height, visible_height, clamped_offset))
                    } else {
                        None
                    };

                    // Collect children in order
                    let mut children = Vec::new();
                    let mut child = first_child;
                    while let Some(child_id) = child {
                        children.push(child_id);
                        child = self.dom.nodes[child_id].next_sibling;
                    }

                    if let Some((content_height, visible_height, clamped_offset)) = scroll_info {
                        let overflows = content_height > visible_height;
                        let thumb_hovered = self
                            .dom
                            .scroll_drag
                            .as_ref()
                            .is_some_and(|d| d.node_id == node_id)
                            || self.dom.scroll_thumbs.iter().any(|t| {
                                t.node_id == node_id
                                    && self
                                        .dom
                                        .hit_state
                                        .mouse_position
                                        .is_some_and(|(mx, my)| t.thumb_bounds.contains(mx, my))
                            });

                        let mouse_in_view = self
                            .dom
                            .scroll_drag
                            .as_ref()
                            .is_some_and(|d| d.node_id == node_id)
                            || self.dom.hit_state.mouse_position.is_some_and(|(mx, my)| {
                                mx >= x && mx <= x + w && my >= y && my <= y + h
                            });

                        // Push in reverse order for LIFO stack:
                        // 6. PaintThumb (last to execute)
                        if overflows {
                            stack.push(StackItem::PaintThumb(ThumbInfo {
                                node_id,
                                view_x: x,
                                view_y: y,
                                view_w: w,
                                view_h: h,
                                scroll_offset_y: clamped_offset,
                                content_height,
                                visible_height,
                                thumb_hovered,
                                mouse_in_view,
                                scale,
                            }));
                        }
                        // 5. PopClip
                        stack.push(StackItem::PopClip);
                        // 4-3. Children (reversed for correct order)
                        for &child_id in children.iter().rev() {
                            stack.push(StackItem::Visit(
                                child_id,
                                x,
                                y - clamped_offset as f64,
                                inherited.clone(),
                            ));
                        }
                        // 2. PushClip
                        let clip_rect = Rect::new(x, y, x + w, y + h);
                        stack.push(StackItem::PushClip(clip_rect, scale));
                    } else {
                        // Normal (non-scrollable) node: push children
                        for &child_id in children.iter().rev() {
                            stack.push(StackItem::Visit(child_id, x, y, inherited.clone()));
                        }
                    }

                    // 1. PaintNode (always first — the node's own bg/borders)
                    render_list.push(RenderCommand::PaintNode(RenderInfo {
                        node_id,
                        x,
                        y,
                        w,
                        h,
                        style: Box::new(computed_style),
                        text,
                        needs_hitbox,
                        input,
                        checkbox: checkbox_snapshot,
                    }));
                }
            }
        }

        // Paint all commands in order
        for cmd in &render_list {
            match cmd {
                RenderCommand::PaintNode(info) => self.paint_node(info, &text_sel_map),
                RenderCommand::PushClip(rect, s) => {
                    self.scene
                        .push_clip_layer(Fill::NonZero, Affine::scale(*s), rect);
                }
                RenderCommand::PopClip => {
                    self.scene.pop_layer();
                }
                RenderCommand::PaintThumb(thumb) => self.paint_thumb(thumb),
            }
        }
    }

    fn paint_node(&mut self, info: &RenderInfo, text_sel_map: &HashMap<UzNodeId, (usize, usize)>) {
        let scale = self.scale;
        let bounds = Bounds::new(info.x, info.y, info.w, info.h);

        // Register hitbox if needed
        if info.needs_hitbox {
            let hitbox_id = self.dom.hitbox_store.insert(info.node_id, bounds);
            self.dom.nodes[info.node_id].interactivity.hitbox_id = Some(hitbox_id);
        }

        if let Some(input_info) = &info.input {
            let content_info = crate::element::input::paint_input(
                self.scene,
                self.text_renderer,
                bounds,
                &info.style,
                input_info,
                scale,
            );

            // Paint scrollbar for multiline inputs with overflow
            if let Some(ci) = content_info
                && ci.content_height > ci.visible_height
            {
                let mouse_in = self
                    .dom
                    .scroll_drag
                    .as_ref()
                    .is_some_and(|d| d.node_id == info.node_id)
                    || self
                        .dom
                        .hit_state
                        .mouse_position
                        .is_some_and(|(mx, my)| bounds.contains(mx, my));

                let thumb_width = 4.0;
                let thumb_margin = 4.0;
                let ratio = ci.visible_height / ci.content_height;
                let thumb_height = (bounds.height * ratio).max(24.0);
                let max_scroll = (ci.content_height - ci.visible_height).max(0.0);
                let scroll_ratio = if max_scroll > 0.0 {
                    ci.scroll_offset_y / max_scroll
                } else {
                    0.0
                };
                let thumb_y = bounds.y + scroll_ratio * (bounds.height - thumb_height);
                let thumb_x = bounds.x + bounds.width - thumb_width - thumb_margin;

                let thumb_bounds = Bounds::new(thumb_x, thumb_y, thumb_width, thumb_height);

                // Register for hit testing (drag + wheel)
                self.dom.scroll_thumbs.push(ScrollThumbRect {
                    node_id: info.node_id,
                    thumb_bounds,
                    view_bounds: bounds,
                    content_height: ci.content_height as f32,
                    visible_height: ci.visible_height as f32,
                });

                if mouse_in {
                    let thumb_hovered = self
                        .dom
                        .scroll_drag
                        .as_ref()
                        .is_some_and(|d| d.node_id == info.node_id)
                        || self
                            .dom
                            .hit_state
                            .mouse_position
                            .is_some_and(|(mx, my)| thumb_bounds.contains(mx, my));
                    let alpha = if thumb_hovered { 140u8 } else { 90u8 };
                    let color = VelloColor::from_rgba8(255, 255, 255, alpha);
                    let radius = thumb_width / 2.0;
                    let rect = Rect::new(
                        thumb_x,
                        thumb_y,
                        thumb_x + thumb_width,
                        thumb_y + thumb_height,
                    );
                    let rounded =
                        RoundedRect::from_rect(rect, RoundedRectRadii::from_single_radius(radius));
                    // Clip to input bounds
                    let clip = Rect::new(
                        bounds.x,
                        bounds.y,
                        bounds.x + bounds.width,
                        bounds.y + bounds.height,
                    );
                    self.scene
                        .push_clip_layer(Fill::NonZero, Affine::scale(scale), &clip);
                    self.scene
                        .fill(Fill::NonZero, Affine::scale(scale), color, None, &rounded);
                    self.scene.pop_layer();
                }
            }
        } else if let Some(checkbox_info) = &info.checkbox {
            crate::element::checkbox::paint_checkbox(
                self.scene,
                bounds,
                &info.style,
                checkbox_info,
                scale,
            );
        } else if let Some((content, font_size, color)) = &info.text {
            let sel_range = text_sel_map.get(&info.node_id).copied();
            if sel_range.is_some() {
                // Text node with active selection: paint selection
                // highlight between background and text glyphs.
                let scene = &mut *self.scene;
                let text_renderer = &mut *self.text_renderer;
                info.style.paint(bounds, scene, scale, |scene| {
                    if let Some((sel_start, sel_end)) = sel_range {
                        let positions = text_renderer.grapheme_positions_2d(
                            content,
                            *font_size,
                            Some(bounds.width as f32),
                        );
                        let line_height = (*font_size * 1.2).round();
                        let rects = compute_selection_rects(
                            &positions,
                            sel_start,
                            sel_end,
                            bounds.width,
                            line_height as f64,
                        );
                        let sel_color = VelloColor::from_rgba8(56, 121, 185, 128);
                        for [x1, y1, x2, y2] in rects {
                            scene.fill(
                                Fill::NonZero,
                                Affine::scale(scale),
                                sel_color,
                                None,
                                &Rect::new(
                                    bounds.x + x1,
                                    bounds.y + y1,
                                    bounds.x + x2,
                                    bounds.y + y2,
                                ),
                            );
                        }
                    }
                    text_renderer.draw_text(
                        scene,
                        content,
                        *font_size,
                        bounds.width as f32,
                        bounds.height as f32,
                        (bounds.x as f32, bounds.y as f32),
                        color.to_vello(),
                        scale,
                    );
                });
            } else {
                crate::element::text::paint_text(
                    self.scene,
                    self.text_renderer,
                    bounds,
                    &info.style,
                    content,
                    *font_size,
                    *color,
                    scale,
                );
            }
        } else {
            crate::element::view::paint_view(self.scene, bounds, &info.style, scale, |_| {});
        }
    }

    fn paint_thumb(&mut self, thumb: &ThumbInfo) {
        // Only show scrollbar when mouse is inside the scrollable node
        if !thumb.mouse_in_view {
            return;
        }

        // Scrollbar thumb: 4px wide, 4px margin from right edge
        let thumb_width = 4.0;
        let thumb_margin = 4.0;

        let ratio = thumb.visible_height as f64 / thumb.content_height as f64;
        let thumb_height = (thumb.view_h * ratio).max(24.0);
        let track_height = thumb.view_h;
        let max_scroll = (thumb.content_height - thumb.visible_height).max(0.0) as f64;
        let scroll_ratio = if max_scroll > 0.0 {
            thumb.scroll_offset_y as f64 / max_scroll
        } else {
            0.0
        };
        let thumb_y = thumb.view_y + scroll_ratio * (track_height - thumb_height);
        let thumb_x = thumb.view_x + thumb.view_w - thumb_width - thumb_margin;

        let thumb_bounds = Bounds::new(thumb_x, thumb_y, thumb_width, thumb_height);

        // Register for hit testing
        self.dom.scroll_thumbs.push(ScrollThumbRect {
            node_id: thumb.node_id,
            thumb_bounds,
            view_bounds: Bounds::new(thumb.view_x, thumb.view_y, thumb.view_w, thumb.view_h),
            content_height: thumb.content_height,
            visible_height: thumb.visible_height,
        });

        // Paint the thumb as a filled rounded rect
        let alpha = if thumb.thumb_hovered { 140u8 } else { 90u8 };
        let color = VelloColor::from_rgba8(255, 255, 255, alpha);
        let radius = thumb_width / 2.0;
        let rect = Rect::new(
            thumb_x,
            thumb_y,
            thumb_x + thumb_width,
            thumb_y + thumb_height,
        );
        let rounded = RoundedRect::from_rect(rect, RoundedRectRadii::from_single_radius(radius));
        self.scene.fill(
            Fill::NonZero,
            Affine::scale(thumb.scale),
            color,
            None,
            &rounded,
        );
    }

    /// Pre-compute per-text-node selection ranges for the current frame.
    /// Returns a map from NodeId → (local_sel_start, local_sel_end) in grapheme units.
    fn compute_text_selections_map(&self) -> HashMap<UzNodeId, (usize, usize)> {
        let mut map = HashMap::new();
        let sel = self.dom.text_selection;
        let Some(root) = sel.root else {
            return map;
        };
        if sel.is_collapsed() {
            return map;
        }
        let Some(run) = self
            .dom
            .selectable_text_runs
            .iter()
            .find(|r| r.root_id == root)
        else {
            return map;
        };
        let sel_start = sel.start();
        let sel_end = sel.end();
        for entry in &run.entries {
            let entry_end = entry.flat_start + entry.grapheme_count;
            let local_start = sel_start.max(entry.flat_start);
            let local_end = sel_end.min(entry_end);
            if local_start < local_end {
                map.insert(
                    entry.node_id,
                    (local_start - entry.flat_start, local_end - entry.flat_start),
                );
            }
        }
        map
    }
}

struct RenderInfo {
    node_id: UzNodeId,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    style: Box<UzStyle>,
    text: Option<(String, f32, Color)>,
    needs_hitbox: bool,
    input: Option<InputRenderInfo>,
    checkbox: Option<CheckboxRenderInfo>,
}

struct ThumbInfo {
    node_id: UzNodeId,
    view_x: f64,
    view_y: f64,
    view_w: f64,
    view_h: f64,
    scroll_offset_y: f32,
    content_height: f32,
    visible_height: f32,
    thumb_hovered: bool,
    mouse_in_view: bool,
    scale: f64,
}

enum RenderCommand {
    PaintNode(RenderInfo),
    PushClip(Rect, f64),
    PopClip,
    PaintThumb(ThumbInfo),
}

enum StackItem {
    Visit(UzNodeId, f64, f64, InheritedProperties),
    PushClip(Rect, f64),
    PopClip,
    PaintThumb(ThumbInfo),
}

pub(crate) fn measure(
    text_renderer: &mut TextRenderer,
    known_dimensions: taffy::Size<Option<f32>>,
    available_space: taffy::Size<taffy::AvailableSpace>,
    node_context: Option<&mut NodeContext>,
) -> taffy::Size<f32> {
    let default_size = taffy::Size {
        width: known_dimensions.width.unwrap_or(0.0),
        height: known_dimensions.height.unwrap_or(0.0),
    };

    let Some(ctx) = node_context else {
        return default_size;
    };

    if ctx.is_input {
        return taffy::Size {
            width: known_dimensions
                .width
                .or_else(|| available_as_option(available_space.width))
                .unwrap_or(200.0),
            height: known_dimensions
                .height
                .unwrap_or(ctx.font_size * 1.2 + 16.0),
        };
    }

    if let Some(text) = &ctx.text {
        let (measured_width, measured_height) = text_renderer.measure_text(
            &text.content,
            ctx.font_size,
            known_dimensions
                .width
                .or_else(|| available_as_option(available_space.width)),
            known_dimensions
                .height
                .or_else(|| available_as_option(available_space.height)),
        );

        return taffy::Size {
            width: measured_width,
            height: measured_height,
        };
    }

    default_size
}

fn available_as_option(space: taffy::AvailableSpace) -> Option<f32> {
    match space {
        taffy::AvailableSpace::Definite(v) => Some(v),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> GlyphPos2D {
        GlyphPos2D { x, y }
    }

    #[test]
    fn sel_rect_empty_selection() {
        let positions = vec![p(0.0, 0.0), p(10.0, 0.0)];
        let rects = compute_selection_rects(&positions, 1, 1, 200.0, 20.0);
        assert!(rects.is_empty());
    }

    #[test]
    fn sel_rect_empty_positions() {
        let rects = compute_selection_rects(&[], 0, 1, 200.0, 20.0);
        assert!(rects.is_empty());
    }

    #[test]
    fn sel_rect_single_line() {
        let positions = vec![p(0.0, 0.0), p(10.0, 0.0), p(20.0, 0.0), p(30.0, 0.0)];
        let rects = compute_selection_rects(&positions, 1, 3, 200.0, 20.0);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0], [10.0, 0.0, 30.0, 20.0]);
    }

    #[test]
    fn sel_rect_single_line_from_start() {
        let positions = vec![p(0.0, 0.0), p(10.0, 0.0), p(20.0, 0.0)];
        let rects = compute_selection_rects(&positions, 0, 2, 200.0, 20.0);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0], [0.0, 0.0, 20.0, 20.0]);
    }

    #[test]
    fn sel_rect_two_lines() {
        let positions = vec![
            p(0.0, 0.0),
            p(10.0, 0.0),
            p(20.0, 0.0),
            p(0.0, 20.0),
            p(10.0, 20.0),
            p(20.0, 20.0),
        ];
        let rects = compute_selection_rects(&positions, 1, 4, 200.0, 20.0);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0], [10.0, 0.0, 20.0, 20.0]);
        assert_eq!(rects[1], [0.0, 20.0, 10.0, 40.0]);
    }

    #[test]
    fn sel_rect_three_lines_middle_full_width() {
        let positions = vec![
            p(0.0, 0.0),
            p(10.0, 0.0),
            p(0.0, 20.0),
            p(15.0, 20.0),
            p(0.0, 40.0),
            p(10.0, 40.0),
        ];
        let rects = compute_selection_rects(&positions, 0, 5, 200.0, 20.0);
        assert_eq!(rects.len(), 3);
        assert_eq!(rects[0], [0.0, 0.0, 10.0, 20.0]);
        assert_eq!(rects[1], [0.0, 20.0, 15.0, 40.0]);
        assert_eq!(rects[2], [0.0, 40.0, 10.0, 60.0]);
    }

    #[test]
    fn sel_rect_last_line_at_x_zero_gets_stub() {
        let positions = vec![
            p(0.0, 0.0),
            p(10.0, 0.0),
            p(20.0, 0.0),
            p(30.0, 0.0),
            p(0.0, 20.0),
        ];
        let rects = compute_selection_rects(&positions, 0, 4, 200.0, 20.0);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0], [0.0, 0.0, 30.0, 20.0]);
        assert_eq!(rects[1], [0.0, 20.0, 8.0, 40.0]);
    }

    #[test]
    fn sel_rect_empty_middle_line_gets_stub() {
        let positions = vec![
            p(0.0, 0.0),
            p(10.0, 0.0),
            p(0.0, 20.0),
            p(0.0, 40.0),
            p(10.0, 40.0),
        ];
        let rects = compute_selection_rects(&positions, 0, 4, 200.0, 20.0);
        assert_eq!(rects.len(), 3);
        assert_eq!(rects[0], [0.0, 0.0, 10.0, 20.0]);
        assert_eq!(rects[1], [0.0, 20.0, 8.0, 40.0]);
        assert_eq!(rects[2], [0.0, 40.0, 10.0, 60.0]);
    }

    #[test]
    fn sel_rect_clamped_to_positions_len() {
        let positions = vec![p(0.0, 0.0), p(10.0, 0.0), p(20.0, 0.0)];
        let rects = compute_selection_rects(&positions, 0, 100, 200.0, 20.0);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0], [0.0, 0.0, 20.0, 20.0]);
    }
}
