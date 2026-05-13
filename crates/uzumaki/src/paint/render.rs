use std::collections::HashMap;

use slab::Slab;
use vello::Scene;
use vello::kurbo::{Affine, Rect};
use vello::peniko::{Color as VelloColor, Fill};

use crate::layout::{NodeContext, TaffyLayoutExt};
use crate::node::{Node, ScrollAxis, UzNodeId};
use crate::paint::{
    ScrollThumbRect,
    checkbox::CheckboxRenderInfo,
    image::ImageRenderInfo,
    input::InputRenderInfo,
    scroll::{self, ScrollAxisInfo, ThumbGeometry},
};
use crate::style::{Bounds, ScrollbarStyle, UzStyle, Visibility};
use crate::text::{
    TextRenderer, apply_text_style_to_editor, secure_cursor_geometry, secure_selection_geometry,
};
use crate::ui::UIState;

pub struct Painter<'a> {
    dom: &'a mut UIState,
    text_renderer: &'a mut TextRenderer,
    scale: f64,
}

impl<'a> Painter<'a> {
    pub fn new(dom: &'a mut UIState, text_renderer: &'a mut TextRenderer, scale: f64) -> Self {
        Self {
            dom,
            text_renderer,
            scale,
        }
    }

    pub fn paint(mut self, scene: &mut Scene) {
        self.dom.hitbox_store.clear();
        self.dom.scroll_thumbs.clear();
        for (_, node) in self.dom.nodes.iter_mut() {
            node.hitbox_id = None;
        }
        self.dom.build_text_select_runs();

        let text_selections = self.compute_text_selections_map();

        let Some(root) = self.dom.root else {
            return;
        };

        self.render_node(
            root,
            0.0,
            0.0,
            Affine::scale(self.scale),
            Affine::IDENTITY,
            scene,
            &text_selections,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn render_node(
        &mut self,
        node_id: UzNodeId,
        parent_x: f64,
        parent_y: f64,
        parent_paint_transform: Affine,
        parent_hit_transform: Affine,
        scene: &mut Scene,
        text_selections: &HashMap<UzNodeId, (usize, usize)>,
    ) {
        let Some(node_ref) = self.dom.nodes.get(node_id) else {
            return;
        };
        let layout = node_ref.final_layout;

        let computed_style = node_ref.computed_style().clone();

        if computed_style.visibility == Visibility::Hidden
            || computed_style.display == crate::style::Display::None
        {
            return;
        }

        let border_box = layout.border_box_bounds();
        let content_box = layout.content_box_bounds();
        let x = parent_x + layout.location.x as f64;
        let y = parent_y + layout.location.y as f64;
        let w = border_box.width;
        let h = border_box.height;

        let local_style_transform = computed_style.transform.to_affine(w, h);
        let local_translate =
            Affine::translate((layout.location.x as f64, layout.location.y as f64));
        let transform = parent_paint_transform * local_translate * local_style_transform;
        let hit_transform = parent_hit_transform * local_translate * local_style_transform;

        let hitbox_id =
            self.dom
                .hitbox_store
                .insert_transformed(node_id, border_box, hit_transform);
        self.dom.nodes[node_id].hitbox_id = Some(hitbox_id);

        self.paint_node(
            node_id,
            &layout,
            &computed_style,
            border_box,
            Bounds::new(x, y, w, h),
            transform,
            scene,
            text_selections,
        );

        let view_scroll = self.prepare_view_scroll(
            node_id,
            &computed_style,
            &layout,
            Bounds::new(x, y, w, h),
            transform,
        );

        let needs_clip = view_scroll.is_some()
            || computed_style.overflow_x.clips()
            || computed_style.overflow_y.clips();

        let (offset_x, offset_y, mouse_in_view, view_thumbs) = match view_scroll {
            Some(v) => (v.offset_x, v.offset_y, v.mouse_in_view, v.thumbs),
            None => (0.0, 0.0, false, Vec::new()),
        };

        let scroll_translate = if offset_x != 0.0 || offset_y != 0.0 {
            Affine::translate((-offset_x, -offset_y))
        } else {
            Affine::IDENTITY
        };
        let child_paint_transform = transform * scroll_translate;
        let child_hit_transform = hit_transform * scroll_translate;

        if needs_clip {
            scene.push_clip_layer(Fill::NonZero, transform, &content_box.to_rect());
        }

        let children = self.dom.nodes[node_id].layout_children.borrow().clone();
        for child_id in children {
            self.render_node(
                child_id,
                x - offset_x,
                y - offset_y,
                child_paint_transform,
                child_hit_transform,
                scene,
                text_selections,
            );
        }

        if needs_clip {
            scene.pop_layer();
        }

        if mouse_in_view {
            for thumb in view_thumbs {
                scroll::paint_thumb(
                    scene,
                    thumb.transform,
                    &thumb.geom,
                    thumb.hovered,
                    thumb.active,
                    &thumb.style,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_node(
        &mut self,
        node_id: UzNodeId,
        layout: &taffy::Layout,
        style: &UzStyle,
        bounds: Bounds,      // local (0,0,w,h)
        view_bounds: Bounds, // absolute (x,y,w,h) for scrollbar placement
        transform: Affine,
        scene: &mut Scene,
        text_selections: &HashMap<UzNodeId, (usize, usize)>,
    ) {
        if self.dom.nodes[node_id].is_text_input() {
            // TODO: dont snapshot
            let info = self.build_input_render_info(node_id, style, layout);
            crate::paint::input::paint_input(
                scene,
                self.text_renderer,
                bounds,
                style,
                layout.content_box_bounds(),
                &info,
                transform,
            );
            // Input scrollbar paints outside the input's text clip.
            if let Some(thumb) = self.register_input_scrollbar(
                node_id,
                style,
                layout,
                &info,
                transform,
                view_bounds.x,
                view_bounds.y,
            ) {
                scroll::paint_thumb(
                    scene,
                    thumb.transform,
                    &thumb.geom,
                    thumb.hovered,
                    thumb.active,
                    &thumb.style,
                );
            }
            return;
        }

        let node = &self.dom.nodes[node_id];

        if let Some(&checked) = node.as_checkbox_input() {
            let info = CheckboxRenderInfo {
                checked,
                focused: self.dom.focused_node == Some(node_id),
            };
            crate::paint::checkbox::paint_checkbox(scene, bounds, style, &info, transform);
        } else if let Some(img) = node.as_image() {
            let info = ImageRenderInfo {
                data: img.data.clone(),
            };
            crate::paint::image::paint_image(scene, bounds, style, &info, transform);
        } else if let Some(inline) = node
            .as_element()
            .and_then(|element| element.inline_layout.as_ref())
        {
            let sel = self.compute_inline_selection(node_id);
            if !inline.entries.is_empty() {
                // todo we should properly render inline box not just text
                let colors = inline
                    .entries
                    .iter()
                    .map(|entry| {
                        let color = self
                            .dom
                            .nodes
                            .get(entry.node_id)
                            .map(|node| node.computed_style().text.color.to_vello())
                            .unwrap_or_else(|| style.text.color.to_vello());
                        (entry.node_id, color)
                    })
                    .collect::<HashMap<_, _>>();
                Self::paint_text_node(
                    scene,
                    bounds,
                    style,
                    layout.content_box_bounds(),
                    &inline.layout,
                    inline.text.len(),
                    transform,
                    sel,
                    Some(&colors),
                );
            } else {
                let sel = text_selections.get(&node_id).copied();
                Self::paint_text_node(
                    scene,
                    bounds,
                    style,
                    layout.content_box_bounds(),
                    &inline.layout,
                    inline.text.len(),
                    transform,
                    sel,
                    None,
                );
            }
        } else if let Some(tc) = node.get_text_content() {
            let sel = text_selections.get(&node_id).copied();
            let text_len = tc.content.len();
            // Cached parley layout (built once per frame in refresh_text_layouts).
            // If absent (shouldn't happen for nodes with text content), skip.
            if let Some(text_layout) = node
                .as_element()
                .and_then(|element| element.inline_layout.as_ref())
                .map(|inline| &inline.layout)
            {
                Self::paint_text_node(
                    scene,
                    bounds,
                    style,
                    layout.content_box_bounds(),
                    text_layout,
                    text_len,
                    transform,
                    sel,
                    None,
                );
            }
        } else {
            crate::paint::view::paint_view(scene, bounds, style, transform, |_| {});
        }
    }

    /// Draw a text node from its cached parley layout, optionally with a
    /// selection highlight.
    #[allow(clippy::too_many_arguments)]
    fn paint_text_node(
        scene: &mut Scene,
        bounds: Bounds,
        style: &UzStyle,
        content_box: Bounds,
        layout: &parley::Layout<crate::text::TextBrush>,
        text_len: usize,
        transform: Affine,
        selection: Option<(usize, usize)>,
        colors: Option<&HashMap<usize, VelloColor>>,
    ) {
        style.paint(bounds, scene, transform, |scene| {
            let text_x = content_box.x;
            let text_y = content_box.y;
            if let Some((sel_start, sel_end)) = selection {
                let rects =
                    crate::text::selection_rects_from_layout(layout, text_len, sel_start, sel_end);
                let sel_color = VelloColor::from_rgba8(56, 121, 185, 128);
                for rect in rects {
                    scene.fill(
                        Fill::NonZero,
                        transform,
                        sel_color,
                        None,
                        &Rect::new(
                            rect.x0 + text_x,
                            rect.y0 + text_y,
                            rect.x1 + text_x,
                            rect.y1 + text_y,
                        ),
                    );
                }
            }
            if let Some(colors) = colors {
                crate::text::draw_layout_with_brush(
                    scene,
                    layout,
                    (content_box.x as f32, content_box.y as f32),
                    transform,
                    |brush| {
                        colors
                            .get(&brush.id)
                            .copied()
                            .unwrap_or_else(|| style.text.color.to_vello())
                    },
                );
            } else {
                crate::text::draw_layout(
                    scene,
                    layout,
                    (content_box.x as f32, content_box.y as f32),
                    style.text.color.to_vello(),
                    transform,
                );
            }
        });
    }

    fn build_input_render_info(
        &mut self,
        node_id: UzNodeId,
        style: &UzStyle,
        layout: &taffy::Layout,
    ) -> InputRenderInfo {
        let text_w = layout.content_box_width().max(0.0);
        let text_style = style.text.clone();

        // Grab seed values first so we can drop the immutable borrow.
        let (multiline, secure, has_preedit) = {
            let is = self.dom.nodes[node_id].as_text_input().unwrap();
            (is.multiline, is.secure, is.preedit.is_some())
        };

        let focused = self.dom.focused_node == Some(node_id);

        let node_mut = &mut self.dom.nodes[node_id];
        let is = node_mut.as_text_input_mut().unwrap();

        apply_text_style_to_editor(&mut is.editor, &text_style);
        is.editor
            .set_width(if multiline { Some(text_w) } else { None });
        is.editor.refresh_layout(
            &mut self.text_renderer.font_ctx,
            &mut self.text_renderer.layout_ctx,
        );

        let blink_visible = is.blink_visible(focused, self.dom.window_focused);

        let cursor_rect = if blink_visible || has_preedit {
            if secure {
                secure_cursor_geometry(&is.editor, 1.5, &text_style, self.text_renderer)
            } else {
                is.editor.cursor_geometry(1.5)
            }
        } else {
            None
        };

        let selection_rects = if secure {
            secure_selection_geometry(&is.editor, &text_style, self.text_renderer)
        } else {
            is.editor
                .selection_geometry()
                .into_iter()
                .map(|(bb, _)| bb)
                .collect()
        };

        let layout_height = is.editor.try_layout().map(|l| l.height()).unwrap_or(0.0);

        let preedit = is.preedit.clone().map(|ps| {
            let positions = self
                .text_renderer
                .grapheme_x_positions(&ps.text, &text_style);
            let width = *positions.last().unwrap_or(&0.0);
            crate::paint::input::PreeditRenderInfo {
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

        let display_text = is.display_text();
        let placeholder = is.placeholder.clone();
        let scroll_offset_x = node_mut.scroll_state.scroll_offset_x;
        let scroll_offset_y = node_mut.scroll_state.scroll_offset_y;

        InputRenderInfo {
            display_text,
            placeholder,
            text_style,
            focused,
            cursor_rect,
            selection_rects,
            scroll_offset_x,
            scroll_offset_y,
            blink_visible,
            multiline,
            layout_height,
            preedit,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn register_input_scrollbar(
        &mut self,
        node_id: UzNodeId,
        style: &UzStyle,
        layout: &taffy::Layout,
        info: &InputRenderInfo,
        transform: Affine,
        view_x: f64,
        view_y: f64,
    ) -> Option<ScrollbarPaint> {
        if !info.multiline {
            return None;
        }
        let border_box = layout.border_box_bounds();
        let content_box = layout.content_box_bounds();
        let axis_info = ScrollAxisInfo {
            content_size: info.layout_height as f64,
            visible_size: content_box.height,
            offset: info.scroll_offset_y as f64,
        };
        if !axis_info.overflows() {
            return None;
        }
        let max_y = axis_info.max_scroll() as f32;
        let scroll = &mut self.dom.nodes[node_id].scroll_state;
        if scroll.scroll_offset_y > max_y {
            scroll.scroll_offset_y = max_y;
        }

        let view_local = border_box;
        let geom = scroll::thumb_geometry(ScrollAxis::Y, view_local, axis_info, &style.scrollbar);
        let view_bounds = Bounds::new(view_x, view_y, border_box.width, border_box.height);
        let thumb_bounds = Bounds::new(
            view_x + geom.local_x,
            view_y + geom.local_y,
            geom.thumb_width,
            geom.thumb_height,
        );

        self.dom.scroll_thumbs.push(ScrollThumbRect {
            node_id,
            axis: ScrollAxis::Y,
            thumb_bounds,
            view_bounds,
            content_size: axis_info.content_size as f32,
            visible_size: axis_info.visible_size as f32,
        });

        let mouse_in = self.is_active_drag(node_id)
            || self
                .dom
                .hit_state
                .mouse_position
                .is_some_and(|(mx, my)| view_bounds.contains(mx, my));
        if !mouse_in {
            return None;
        }

        let active = self.is_active_drag_axis(node_id, ScrollAxis::Y);
        let hovered = !active
            && self
                .dom
                .hit_state
                .mouse_position
                .is_some_and(|(mx, my)| thumb_bounds.contains(mx, my));

        Some(ScrollbarPaint {
            transform,
            geom,
            hovered,
            active,
            style: style.scrollbar,
        })
    }

    fn prepare_view_scroll(
        &mut self,
        node_id: UzNodeId,
        style: &UzStyle,
        layout: &taffy::Layout,
        view_bounds: Bounds,
        transform: Affine,
    ) -> Option<ViewScroll> {
        let scroll_x = style.overflow_x.is_scrollable();
        let scroll_y = style.overflow_y.is_scrollable();
        if !scroll_x && !scroll_y {
            return None;
        }

        let visible_w = layout.axis_content_box_size(ScrollAxis::X) as f64;
        let visible_h = layout.axis_content_box_size(ScrollAxis::Y) as f64;
        let content_w = layout.axis_scroll_content_size(ScrollAxis::X) as f64;
        let content_h = layout.axis_scroll_content_size(ScrollAxis::Y) as f64;
        let max_x = layout.axis_scroll_overflow(ScrollAxis::X) as f64;
        let max_y = layout.axis_scroll_overflow(ScrollAxis::Y) as f64;

        let ss = &mut self.dom.nodes[node_id].scroll_state;
        if ss.scroll_offset_x as f64 > max_x {
            ss.scroll_offset_x = max_x as f32;
        }
        if ss.scroll_offset_y as f64 > max_y {
            ss.scroll_offset_y = max_y as f32;
        }
        let offset_x = ss.scroll_offset_x as f64;
        let offset_y = ss.scroll_offset_y as f64;

        let mouse_in_view = self.is_active_drag(node_id)
            || self
                .dom
                .hit_state
                .mouse_position
                .is_some_and(|(mx, my)| view_bounds.contains(mx, my));

        let mut thumbs = Vec::new();
        if scroll_y && content_h > visible_h {
            thumbs.push(self.register_view_thumb(
                node_id,
                ScrollAxis::Y,
                view_bounds,
                content_h,
                visible_h,
                offset_y,
                transform,
                &style.scrollbar,
            ));
        }
        if scroll_x && content_w > visible_w {
            thumbs.push(self.register_view_thumb(
                node_id,
                ScrollAxis::X,
                view_bounds,
                content_w,
                visible_w,
                offset_x,
                transform,
                &style.scrollbar,
            ));
        }

        Some(ViewScroll {
            offset_x,
            offset_y,
            mouse_in_view,
            thumbs,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn register_view_thumb(
        &mut self,
        node_id: UzNodeId,
        axis: ScrollAxis,
        view_bounds: Bounds,
        content: f64,
        visible: f64,
        offset: f64,
        transform: Affine,
        scrollbar: &ScrollbarStyle,
    ) -> ScrollbarPaint {
        let view_local = Bounds::new(0.0, 0.0, view_bounds.width, view_bounds.height);
        let geom = scroll::thumb_geometry(
            axis,
            view_local,
            ScrollAxisInfo {
                content_size: content,
                visible_size: visible,
                offset,
            },
            scrollbar,
        );
        let thumb_bounds = Bounds::new(
            view_bounds.x + geom.local_x,
            view_bounds.y + geom.local_y,
            geom.thumb_width,
            geom.thumb_height,
        );
        self.dom.scroll_thumbs.push(ScrollThumbRect {
            node_id,
            axis,
            thumb_bounds,
            view_bounds,
            content_size: content as f32,
            visible_size: visible as f32,
        });

        let active = self.is_active_drag_axis(node_id, axis);
        let hovered = !active
            && self
                .dom
                .hit_state
                .mouse_position
                .is_some_and(|(mx, my)| thumb_bounds.contains(mx, my));

        ScrollbarPaint {
            transform,
            geom,
            hovered,
            active,
            style: *scrollbar,
        }
    }

    fn is_active_drag(&self, node_id: UzNodeId) -> bool {
        self.dom
            .drag_mode
            .as_scrollbar_thumb()
            .is_some_and(|d| d.node_id == node_id)
    }

    fn is_active_drag_axis(&self, node_id: UzNodeId, axis: ScrollAxis) -> bool {
        self.dom
            .drag_mode
            .as_scrollbar_thumb()
            .is_some_and(|d| d.node_id == node_id && d.axis == axis)
    }

    fn compute_inline_selection(&self, layout_node_id: UzNodeId) -> Option<(usize, usize)> {
        let sel = self.dom.text_selection;
        if sel.is_collapsed() {
            return None;
        }
        let (start, end) = self.dom.ordered_text_selection()?;
        let run = self.dom.find_run_for_node(start.node)?;
        let mut range_start = None;
        let mut range_end = None;
        let mut in_range = false;
        for entry in &run.entries {
            if entry.node_id == start.node {
                in_range = true;
            }
            if !in_range {
                continue;
            }
            if entry.layout_node_id != layout_node_id {
                if entry.node_id == end.node {
                    break;
                }
                continue;
            }
            let local_start = if entry.node_id == start.node {
                start.offset.min(entry.byte_len)
            } else {
                0
            };
            let local_end = if entry.node_id == end.node {
                end.offset.min(entry.byte_len)
            } else {
                entry.byte_len
            };
            let byte_start = entry.flat_byte_start + local_start;
            let byte_end = entry.flat_byte_start + local_end;
            range_start = Some(range_start.map_or(byte_start, |v: usize| v.min(byte_start)));
            range_end = Some(range_end.map_or(byte_end, |v: usize| v.max(byte_end)));
            if entry.node_id == end.node {
                break;
            }
        }
        Some((range_start?, range_end?))
    }

    fn compute_text_selections_map(&self) -> HashMap<UzNodeId, (usize, usize)> {
        let mut map = HashMap::new();
        let sel = self.dom.text_selection;
        if sel.is_collapsed() {
            return map;
        }
        let Some((start, end)) = self.dom.ordered_text_selection() else {
            return map;
        };
        let Some(run) = self.dom.find_run_for_node(start.node) else {
            return map;
        };
        let mut in_range = false;
        for entry in &run.entries {
            if entry.node_id == start.node {
                in_range = true;
            }
            if !in_range {
                continue;
            }
            let Some(text) = self
                .dom
                .nodes
                .get(entry.node_id)
                .and_then(|n| n.get_text_content())
            else {
                continue;
            };
            let local_start = if entry.node_id == start.node {
                start.offset.min(text.content.len())
            } else {
                0
            };
            let local_end = if entry.node_id == end.node {
                end.offset.min(text.content.len())
            } else {
                text.content.len()
            };
            if local_start < local_end {
                map.insert(entry.node_id, (local_start, local_end));
            }
            if entry.node_id == end.node {
                break;
            }
        }
        map
    }
}

struct ViewScroll {
    offset_x: f64,
    offset_y: f64,
    mouse_in_view: bool,
    thumbs: Vec<ScrollbarPaint>,
}

#[derive(Clone)]
struct ScrollbarPaint {
    transform: Affine,
    geom: ThumbGeometry,
    hovered: bool,
    active: bool,
    style: ScrollbarStyle,
}

pub(crate) fn measure(
    text_renderer: &mut TextRenderer,
    nodes: &Slab<Node>,
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
    let Some(node) = nodes.get(ctx.node_id) else {
        return default_size;
    };
    let style = node.computed_style();

    if node.is_text_input() {
        return taffy::Size {
            width: known_dimensions
                .width
                .or_else(|| available_as_option(available_space.width))
                .unwrap_or(200.0),
            height: known_dimensions
                .height
                .unwrap_or((style.text.font_size * style.text.line_height).round()),
        };
    }

    if let Some(text) = node
        .as_element()
        .and_then(|element| element.inline_layout.as_ref())
        .map(|inline| inline.text.as_str())
        .or_else(|| node.get_text_content().map(|text| text.content.as_str()))
    {
        let (measured_width, measured_height) = text_renderer.measure_text(
            text,
            &style.text,
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

    if let Some((width, height)) = node.as_image().and_then(|image| image.data.natural_size()) {
        if width <= 0.0 || height <= 0.0 {
            return default_size;
        }
        let aspect_ratio = width / height;
        let measured_width = known_dimensions.width.unwrap_or({
            if let Some(known_height) = known_dimensions.height {
                known_height * aspect_ratio
            } else {
                width
            }
        });
        let measured_height = known_dimensions.height.unwrap_or_else(|| {
            if let Some(known_width) = known_dimensions.width {
                known_width / aspect_ratio
            } else {
                height
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
    use std::sync::Arc;

    use super::measure;
    use crate::element::{ElementNode, ImageData, ImageNode, RasterImageData};
    use crate::layout::NodeContext;
    use crate::node::Node;
    use crate::style::UzStyle;
    use crate::text::TextRenderer;
    use slab::Slab;

    fn image_nodes(width: u32, height: u32) -> (Slab<Node>, NodeContext) {
        let mut nodes = Slab::new();
        let image = ImageNode {
            data: ImageData::Raster(RasterImageData::new(width, height, Arc::new(Vec::new()))),
        };
        let node_id = nodes.insert(Node::new(UzStyle::default(), ElementNode::new_image(image)));
        (nodes, NodeContext { node_id })
    }

    fn empty_image_nodes() -> (Slab<Node>, NodeContext) {
        let mut nodes = Slab::new();
        let node_id = nodes.insert(Node::new(
            UzStyle::default(),
            ElementNode::new_image(ImageNode::default()),
        ));
        (nodes, NodeContext { node_id })
    }

    #[test]
    fn image_measure_uses_natural_size_when_unconstrained() {
        let mut renderer = TextRenderer::new();
        let (nodes, mut ctx) = image_nodes(320, 180);
        let size = measure(
            &mut renderer,
            &nodes,
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
        let (nodes, mut ctx) = image_nodes(400, 200);
        let size = measure(
            &mut renderer,
            &nodes,
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
        let (nodes, mut ctx) = image_nodes(200, 400);
        let size = measure(
            &mut renderer,
            &nodes,
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
        let (nodes, mut ctx) = image_nodes(320, 180);
        let size = measure(
            &mut renderer,
            &nodes,
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
        let (nodes, mut ctx) = empty_image_nodes();
        let size = measure(
            &mut renderer,
            &nodes,
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
