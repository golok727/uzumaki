use std::collections::HashMap;

use vello::Scene;
use vello::kurbo::{Affine, Rect, RoundedRect, RoundedRectRadii};
use vello::peniko::{Color as VelloColor, Fill};

use crate::element::checkbox::CheckboxRenderInfo;
use crate::element::image::ImageRenderInfo;
use crate::element::input::InputRenderInfo;
use crate::element::{ImageMeasureInfo, NodeContext, ScrollThumbRect, UzNodeId};
use crate::style::{Bounds, Overflow, TextStyle, UzStyle, Visibility};
use crate::text::{
    TextRenderer, apply_text_style_to_editor, secure_cursor_geometry, secure_selection_geometry,
};
use crate::ui::UIState;

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

    fn paint_translation(&self, x: f64, y: f64) -> Affine {
        Affine::translate((x, y))
    }

    fn render_tree(&mut self, root_id: UzNodeId) {
        // Pre-compute per-node selection ranges for text selection painting
        let text_sel_map = self.compute_text_selections_map();

        let mut render_list: Vec<RenderCommand> = Vec::new();
        let mut stack: Vec<StackItem> = vec![StackItem::Visit(
            root_id,
            0.0,
            0.0,
            Affine::scale(self.scale),
            Affine::IDENTITY,
            None,
        )];

        while let Some(item) = stack.pop() {
            match item {
                StackItem::PushClip(rect, transform) => {
                    render_list.push(RenderCommand::PushClip(rect, transform));
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
                StackItem::Visit(
                    node_id,
                    parent_x,
                    parent_y,
                    parent_paint_transform,
                    parent_hit_transform,
                    parent_style,
                ) => {
                    // Extract all needed data from the node (immutable borrow scope)
                    let (
                        taffy_node,
                        computed_style,
                        text,
                        input_snapshot,
                        checkbox_snapshot,
                        image_snapshot,
                        needs_hitbox,
                        is_scrollable,
                        first_child,
                    ) = {
                        let node = &self.dom.nodes[node_id];

                        let taffy_node = node.taffy_node;
                        let computed_style =
                            self.dom.computed_style(node_id, parent_style.as_deref());

                        let text = node
                            .as_text_node()
                            .map(|tc| (tc.content.clone(), computed_style.text.clone()));

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
                        let image_snapshot = node.as_image().map(|image| ImageRenderInfo {
                            data: image.data.clone(),
                        });
                        // Visible boxes participate in hit testing by default. This lets
                        // non-listener overlays consume pointer targeting instead of leaking
                        // hover/active state to lower siblings.
                        let needs_hitbox = true;
                        let is_scrollable = matches!(computed_style.overflow_y, Overflow::Scroll);
                        let first_child = node.first_child;

                        (
                            taffy_node,
                            computed_style,
                            text,
                            input_snapshot,
                            checkbox_snapshot,
                            image_snapshot,
                            needs_hitbox,
                            is_scrollable,
                            first_child,
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
                        let pad_h = computed_style.padding.left + computed_style.padding.right;
                        let text_w = (layout.size.width - pad_h).max(0.0);
                        let text_style = computed_style.text.clone();

                        let node_mut = &mut self.dom.nodes[node_id];
                        let is = node_mut.as_text_input_mut().unwrap();

                        // Apply styles to editor
                        apply_text_style_to_editor(&mut is.editor, &text_style);
                        is.editor
                            .set_width(if multiline { Some(text_w) } else { None });
                        is.editor.refresh_layout(
                            &mut self.text_renderer.font_ctx,
                            &mut self.text_renderer.layout_ctx,
                        );

                        let cursor_rect = if blink_visible || preedit_state.is_some() {
                            if is.secure {
                                secure_cursor_geometry(
                                    &is.editor,
                                    1.5,
                                    &text_style,
                                    self.text_renderer,
                                )
                            } else {
                                is.editor.cursor_geometry(1.5)
                            }
                        } else {
                            None
                        };

                        let selection_rects = if is.secure {
                            secure_selection_geometry(&is.editor, &text_style, self.text_renderer)
                        } else {
                            is.editor
                                .selection_geometry()
                                .into_iter()
                                .map(|(bb, _)| bb)
                                .collect()
                        };

                        let layout_height =
                            is.editor.try_layout().map(|l| l.height()).unwrap_or(0.0);

                        let preedit = preedit_state.map(|ps| {
                            let positions = self
                                .text_renderer
                                .grapheme_x_positions(&ps.text, &text_style);
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
                            text_style,
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
                    let local_style_transform = computed_style.transform.to_affine(w, h);
                    let transform = parent_paint_transform
                        * self
                            .paint_translation(layout.location.x as f64, layout.location.y as f64)
                        * local_style_transform;
                    let hit_transform = parent_hit_transform
                        * Affine::translate((layout.location.x as f64, layout.location.y as f64))
                        * local_style_transform;

                    // Compute scroll info and clamp offset (mutable borrow is safe now)
                    let scroll_info = if is_scrollable {
                        let content_height = layout.content_size.height;
                        let visible_height = layout.size.height;
                        let max_scroll = (content_height - visible_height).max(0.0);
                        if self.dom.nodes[node_id].scroll_state.is_none() {
                            self.dom.nodes[node_id].scroll_state =
                                Some(crate::element::ScrollState::new());
                        }
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
                                view_bounds: Bounds::new(x, y, w, h),
                                transform,
                                view_w: w,
                                view_h: h,
                                scroll_offset_y: clamped_offset,
                                content_height,
                                visible_height,
                                thumb_hovered,
                                mouse_in_view,
                            }));
                        }
                        // 5. PopClip
                        stack.push(StackItem::PopClip);
                        // 4-3. Children (reversed for correct order)
                        for &child_id in children.iter().rev() {
                            let child_paint_parent =
                                transform * Affine::translate((0.0, -clamped_offset as f64));
                            let child_hit_parent =
                                hit_transform * Affine::translate((0.0, -clamped_offset as f64));
                            stack.push(StackItem::Visit(
                                child_id,
                                x,
                                y - clamped_offset as f64,
                                child_paint_parent,
                                child_hit_parent,
                                Some(Box::new(computed_style.clone())),
                            ));
                        }
                        // 2. PushClip
                        let clip_rect = Rect::new(0.0, 0.0, w, h);
                        stack.push(StackItem::PushClip(clip_rect, transform));
                    } else {
                        // Normal (non-scrollable) node: push children
                        for &child_id in children.iter().rev() {
                            stack.push(StackItem::Visit(
                                child_id,
                                x,
                                y,
                                transform,
                                hit_transform,
                                Some(Box::new(computed_style.clone())),
                            ));
                        }
                    }

                    // 1. PaintNode (always first — the node's own bg/borders)
                    render_list.push(RenderCommand::PaintNode(Box::new(RenderInfo {
                        node_id,
                        x,
                        y,
                        w,
                        h,
                        transform,
                        hit_transform,
                        style: Box::new(computed_style),
                        text,
                        needs_hitbox,
                        input,
                        checkbox: checkbox_snapshot,
                        image: image_snapshot,
                    })));
                }
            }
        }

        // Paint all commands in order
        for cmd in &render_list {
            match cmd {
                RenderCommand::PaintNode(info) => self.paint_node(info, &text_sel_map),
                RenderCommand::PushClip(rect, transform) => {
                    self.scene.push_clip_layer(Fill::NonZero, *transform, rect);
                }
                RenderCommand::PopClip => {
                    self.scene.pop_layer();
                }
                RenderCommand::PaintThumb(thumb) => self.paint_thumb(thumb),
            }
        }
    }

    fn paint_node(&mut self, info: &RenderInfo, text_sel_map: &HashMap<UzNodeId, (usize, usize)>) {
        let local_bounds = Bounds::new(0.0, 0.0, info.w, info.h);
        let hit_bounds = Bounds::new(info.x, info.y, info.w, info.h);

        // Register hitbox if needed
        if info.needs_hitbox {
            let hitbox_id = self.dom.hitbox_store.insert_transformed(
                info.node_id,
                local_bounds,
                info.hit_transform,
            );
            self.dom.nodes[info.node_id].interactivity.hitbox_id = Some(hitbox_id);
        }

        if let Some(input_info) = &info.input {
            let content_info = crate::element::input::paint_input(
                self.scene,
                self.text_renderer,
                local_bounds,
                &info.style,
                input_info,
                info.transform,
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
                        .is_some_and(|(mx, my)| hit_bounds.contains(mx, my));

                let thumb_width = 4.0;
                let thumb_margin = 4.0;
                let ratio = ci.visible_height / ci.content_height;
                let thumb_height = (hit_bounds.height * ratio).max(24.0);
                let max_scroll = (ci.content_height - ci.visible_height).max(0.0);
                let scroll_ratio = if max_scroll > 0.0 {
                    ci.scroll_offset_y / max_scroll
                } else {
                    0.0
                };
                let local_thumb_y = scroll_ratio * (hit_bounds.height - thumb_height);
                let local_thumb_x = hit_bounds.width - thumb_width - thumb_margin;
                let thumb_y = hit_bounds.y + local_thumb_y;
                let thumb_x = hit_bounds.x + local_thumb_x;

                let thumb_bounds = Bounds::new(thumb_x, thumb_y, thumb_width, thumb_height);

                // Register for hit testing (drag + wheel)
                self.dom.scroll_thumbs.push(ScrollThumbRect {
                    node_id: info.node_id,
                    thumb_bounds,
                    view_bounds: hit_bounds,
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
                        local_thumb_x,
                        local_thumb_y,
                        local_thumb_x + thumb_width,
                        local_thumb_y + thumb_height,
                    );
                    let rounded =
                        RoundedRect::from_rect(rect, RoundedRectRadii::from_single_radius(radius));
                    // Clip to input bounds
                    let clip = Rect::new(0.0, 0.0, hit_bounds.width, hit_bounds.height);
                    self.scene
                        .push_clip_layer(Fill::NonZero, info.transform, &clip);
                    self.scene
                        .fill(Fill::NonZero, info.transform, color, None, &rounded);
                    self.scene.pop_layer();
                }
            }
        } else if let Some(checkbox_info) = &info.checkbox {
            crate::element::checkbox::paint_checkbox(
                self.scene,
                local_bounds,
                &info.style,
                checkbox_info,
                info.transform,
            );
        } else if let Some(image_info) = &info.image {
            crate::element::image::paint_image(
                self.scene,
                local_bounds,
                &info.style,
                image_info,
                info.transform,
            );
        } else if let Some((content, text_style)) = &info.text {
            let sel_range = text_sel_map.get(&info.node_id).copied();
            if sel_range.is_some() {
                let scene = &mut *self.scene;
                let text_renderer = &mut *self.text_renderer;
                info.style
                    .paint(local_bounds, scene, info.transform, |scene| {
                        if let Some((sel_start, sel_end)) = sel_range {
                            let rects = text_renderer.selection_rects(
                                content,
                                text_style,
                                Some(local_bounds.width as f32),
                                sel_start,
                                sel_end,
                            );
                            let sel_color = VelloColor::from_rgba8(56, 121, 185, 128);
                            for rect in rects {
                                scene.fill(
                                    Fill::NonZero,
                                    info.transform,
                                    sel_color,
                                    None,
                                    &Rect::new(rect.x0, rect.y0, rect.x1, rect.y1),
                                );
                            }
                        }
                        text_renderer.draw_text(
                            scene,
                            content,
                            text_style,
                            local_bounds.width as f32,
                            local_bounds.height as f32,
                            (0.0, 0.0),
                            text_style.color.to_vello(),
                            info.transform,
                        );
                    });
            } else {
                crate::element::text::paint_text(
                    self.scene,
                    self.text_renderer,
                    local_bounds,
                    &info.style,
                    content,
                    text_style,
                    text_style.color,
                    info.transform,
                );
            }
        } else {
            crate::element::view::paint_view(
                self.scene,
                local_bounds,
                &info.style,
                info.transform,
                |_| {},
            );
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
        let local_thumb_y = scroll_ratio * (track_height - thumb_height);
        let local_thumb_x = thumb.view_w - thumb_width - thumb_margin;
        let thumb_y = thumb.view_bounds.y + local_thumb_y;
        let thumb_x = thumb.view_bounds.x + local_thumb_x;

        let thumb_bounds = Bounds::new(thumb_x, thumb_y, thumb_width, thumb_height);

        // Register for hit testing
        self.dom.scroll_thumbs.push(ScrollThumbRect {
            node_id: thumb.node_id,
            thumb_bounds,
            view_bounds: thumb.view_bounds,
            content_height: thumb.content_height,
            visible_height: thumb.visible_height,
        });

        // Paint the thumb as a filled rounded rect
        let alpha = if thumb.thumb_hovered { 140u8 } else { 90u8 };
        let color = VelloColor::from_rgba8(255, 255, 255, alpha);
        let radius = thumb_width / 2.0;
        let rect = Rect::new(
            local_thumb_x,
            local_thumb_y,
            local_thumb_x + thumb_width,
            local_thumb_y + thumb_height,
        );
        let rounded = RoundedRect::from_rect(rect, RoundedRectRadii::from_single_radius(radius));
        self.scene
            .fill(Fill::NonZero, thumb.transform, color, None, &rounded);
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
    transform: Affine,
    hit_transform: Affine,
    style: Box<UzStyle>,
    text: Option<(String, TextStyle)>,
    needs_hitbox: bool,
    input: Option<InputRenderInfo>,
    checkbox: Option<CheckboxRenderInfo>,
    image: Option<ImageRenderInfo>,
}

struct ThumbInfo {
    node_id: UzNodeId,
    view_bounds: Bounds,
    transform: Affine,
    view_w: f64,
    view_h: f64,
    scroll_offset_y: f32,
    content_height: f32,
    visible_height: f32,
    thumb_hovered: bool,
    mouse_in_view: bool,
}

enum RenderCommand {
    PaintNode(Box<RenderInfo>),
    PushClip(Rect, Affine),
    PopClip,
    PaintThumb(ThumbInfo),
}

enum StackItem {
    Visit(UzNodeId, f64, f64, Affine, Affine, Option<Box<UzStyle>>),
    PushClip(Rect, Affine),
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
                .unwrap_or((ctx.text_style.font_size * ctx.text_style.line_height).round()),
        };
    }

    if let Some(text) = &ctx.text {
        let (measured_width, measured_height) = text_renderer.measure_text(
            &text.content,
            &ctx.text_style,
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

    if let Some(ImageMeasureInfo { width, height }) = &ctx.image {
        if *width <= 0.0 || *height <= 0.0 {
            return default_size;
        }

        let aspect_ratio = *width / *height;
        let measured_width = known_dimensions.width.unwrap_or({
            if let Some(known_height) = known_dimensions.height {
                known_height * aspect_ratio
            } else {
                *width
            }
        });
        let measured_height = known_dimensions.height.unwrap_or_else(|| {
            if let Some(known_width) = known_dimensions.width {
                known_width / aspect_ratio
            } else {
                *height
            }
        });

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
        taffy::AvailableSpace::MinContent => Some(0.0),
        taffy::AvailableSpace::MaxContent => None,
    }
}

#[cfg(test)]
mod tests {
    use super::measure;
    use crate::element::{ImageMeasureInfo, NodeContext};
    use crate::style::TextStyle;
    use crate::text::TextRenderer;

    fn image_context(width: f32, height: f32) -> NodeContext {
        NodeContext {
            dom_id: 0,
            text: None,
            text_style: TextStyle::default(),
            is_input: false,
            image: Some(ImageMeasureInfo { width, height }),
        }
    }

    #[test]
    fn image_measure_uses_natural_size_when_unconstrained() {
        let mut renderer = TextRenderer::new();
        let mut ctx = image_context(320.0, 180.0);
        let size = measure(
            &mut renderer,
            taffy::Size {
                width: None,
                height: None,
            },
            taffy::Size {
                width: taffy::AvailableSpace::MaxContent,
                height: taffy::AvailableSpace::MaxContent,
            },
            Some(&mut ctx),
        );
        assert_eq!(size.width, 320.0);
        assert_eq!(size.height, 180.0);
    }

    #[test]
    fn image_measure_preserves_aspect_ratio_with_width_only() {
        let mut renderer = TextRenderer::new();
        let mut ctx = image_context(400.0, 200.0);
        let size = measure(
            &mut renderer,
            taffy::Size {
                width: Some(160.0),
                height: None,
            },
            taffy::Size {
                width: taffy::AvailableSpace::MaxContent,
                height: taffy::AvailableSpace::MaxContent,
            },
            Some(&mut ctx),
        );
        assert_eq!(size.width, 160.0);
        assert_eq!(size.height, 80.0);
    }

    #[test]
    fn image_measure_preserves_aspect_ratio_with_height_only() {
        let mut renderer = TextRenderer::new();
        let mut ctx = image_context(200.0, 400.0);
        let size = measure(
            &mut renderer,
            taffy::Size {
                width: None,
                height: Some(100.0),
            },
            taffy::Size {
                width: taffy::AvailableSpace::MaxContent,
                height: taffy::AvailableSpace::MaxContent,
            },
            Some(&mut ctx),
        );
        assert_eq!(size.width, 50.0);
        assert_eq!(size.height, 100.0);
    }

    #[test]
    fn image_measure_uses_explicit_box_when_both_dimensions_are_known() {
        let mut renderer = TextRenderer::new();
        let mut ctx = image_context(320.0, 180.0);
        let size = measure(
            &mut renderer,
            taffy::Size {
                width: Some(512.0),
                height: Some(128.0),
            },
            taffy::Size {
                width: taffy::AvailableSpace::MaxContent,
                height: taffy::AvailableSpace::MaxContent,
            },
            Some(&mut ctx),
        );
        assert_eq!(size.width, 512.0);
        assert_eq!(size.height, 128.0);
    }

    #[test]
    fn image_measure_without_bitmap_returns_default_size() {
        let mut renderer = TextRenderer::new();
        let mut ctx = NodeContext {
            dom_id: 0,
            text: None,
            text_style: TextStyle::default(),
            is_input: false,
            image: None,
        };
        let size = measure(
            &mut renderer,
            taffy::Size {
                width: None,
                height: None,
            },
            taffy::Size {
                width: taffy::AvailableSpace::MaxContent,
                height: taffy::AvailableSpace::MaxContent,
            },
            Some(&mut ctx),
        );
        assert_eq!(size.width, 0.0);
        assert_eq!(size.height, 0.0);
    }
}
