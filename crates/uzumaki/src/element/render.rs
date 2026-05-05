use std::collections::HashMap;

use vello::Scene;
use vello::kurbo::{Affine, Rect};
use vello::peniko::{Color as VelloColor, Fill};

use crate::element::checkbox::CheckboxRenderInfo;
use crate::element::image::ImageRenderInfo;
use crate::element::input::InputRenderInfo;
use crate::element::scroll::{self, ScrollAxisInfo, ThumbGeometry};
use crate::element::{ImageMeasureInfo, ScrollAxis, ScrollState, ScrollThumbRect, UzNodeId};
use crate::layout::NodeContext;
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
            node.interactivity.hitbox_id = None;
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
            None,
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
        parent_style: Option<&UzStyle>,
        scene: &mut Scene,
        text_selections: &HashMap<UzNodeId, (usize, usize)>,
    ) {
        let Some(node_ref) = self.dom.nodes.get(node_id) else {
            return;
        };
        let layout = LayoutSnapshot::from(&node_ref.final_layout);

        let computed_style = self.dom.computed_style(node_id, parent_style);

        if computed_style.visibility == Visibility::Hidden
            || computed_style.display == crate::style::Display::None
        {
            return;
        }

        let x = parent_x + layout.location_x;
        let y = parent_y + layout.location_y;
        let w = layout.size_w;
        let h = layout.size_h;

        let local_style_transform = computed_style.transform.to_affine(w, h);
        let local_translate = Affine::translate((layout.location_x, layout.location_y));
        let transform = parent_paint_transform * local_translate * local_style_transform;
        let hit_transform = parent_hit_transform * local_translate * local_style_transform;

        let hitbox_id = self.dom.hitbox_store.insert_transformed(
            node_id,
            Bounds::new(0.0, 0.0, w, h),
            hit_transform,
        );
        self.dom.nodes[node_id].interactivity.hitbox_id = Some(hitbox_id);

        self.paint_node(
            node_id,
            &layout,
            &computed_style,
            Bounds::new(0.0, 0.0, w, h),
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
            scene.push_clip_layer(Fill::NonZero, transform, &Rect::new(0.0, 0.0, w, h));
        }

        let children = self.dom.nodes[node_id].children.clone();
        for child_id in children {
            self.render_node(
                child_id,
                x - offset_x,
                y - offset_y,
                child_paint_transform,
                child_hit_transform,
                Some(&computed_style),
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
                    &thumb.style,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_node(
        &mut self,
        node_id: UzNodeId,
        layout: &LayoutSnapshot,
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
            crate::element::input::paint_input(
                scene,
                self.text_renderer,
                bounds,
                style,
                &info,
                transform,
            );
            // Input scrollbar paints outside the input's text clip.
            if let Some(thumb) = self.register_input_scrollbar(
                node_id,
                style,
                layout.size_w,
                layout.size_h,
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
            crate::element::checkbox::paint_checkbox(scene, bounds, style, &info, transform);
        } else if let Some(img) = node.as_image() {
            let info = ImageRenderInfo {
                data: img.data.clone(),
            };
            crate::element::image::paint_image(scene, bounds, style, &info, transform);
        } else if let Some(tc) = node.get_text_content() {
            let sel = text_selections.get(&node_id).copied();
            let text_len = tc.content.len();
            // Cached parley layout (built once per frame in refresh_text_layouts).
            // If absent (shouldn't happen for nodes with text content), skip.
            if let Some(layout) = node.text_layout.as_ref() {
                Self::paint_text_node(scene, bounds, style, layout, text_len, transform, sel);
            }
        } else {
            crate::element::view::paint_view(scene, bounds, style, transform, |_| {});
        }
    }

    /// Draw a text node from its cached parley layout, optionally with a
    /// selection highlight.
    fn paint_text_node(
        scene: &mut Scene,
        bounds: Bounds,
        style: &UzStyle,
        layout: &parley::Layout<crate::text::TextBrush>,
        text_len: usize,
        transform: Affine,
        selection: Option<(usize, usize)>,
    ) {
        style.paint(bounds, scene, transform, |scene| {
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
                        &Rect::new(rect.x0, rect.y0, rect.x1, rect.y1),
                    );
                }
            }
            crate::text::draw_layout(
                scene,
                layout,
                (0.0, 0.0),
                style.text.color.to_vello(),
                transform,
            );
        });
    }

    fn build_input_render_info(
        &mut self,
        node_id: UzNodeId,
        style: &UzStyle,
        layout: &LayoutSnapshot,
    ) -> InputRenderInfo {
        let pad_h = style.padding.left + style.padding.right;
        let text_w = (layout.size_w as f32 - pad_h).max(0.0);
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

        let is = self.dom.nodes[node_id].as_text_input().unwrap();

        InputRenderInfo {
            display_text: is.display_text(),
            placeholder: is.placeholder.clone(),
            text_style,
            focused,
            cursor_rect,
            selection_rects,
            scroll_offset: is.scroll_offset,
            scroll_offset_y: is.scroll_offset_y,
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
        w: f64,
        h: f64,
        info: &InputRenderInfo,
        transform: Affine,
        view_x: f64,
        view_y: f64,
    ) -> Option<ScrollbarPaint> {
        if !info.multiline {
            return None;
        }
        let pad_t = style.padding.top as f64;
        let pad_b = style.padding.bottom as f64;
        let axis_info = ScrollAxisInfo {
            content_size: info.layout_height as f64 + pad_t + pad_b,
            visible_size: h,
            offset: info.scroll_offset_y as f64,
        };
        if !axis_info.overflows() {
            return None;
        }

        let view_local = Bounds::new(0.0, 0.0, w, h);
        let geom = scroll::thumb_geometry(ScrollAxis::Y, view_local, axis_info, &style.scrollbar);
        let view_bounds = Bounds::new(view_x, view_y, w, h);
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

        let hovered = self.is_active_drag(node_id)
            || self
                .dom
                .hit_state
                .mouse_position
                .is_some_and(|(mx, my)| thumb_bounds.contains(mx, my));

        Some(ScrollbarPaint {
            transform,
            geom,
            hovered,
            style: style.scrollbar,
        })
    }

    fn prepare_view_scroll(
        &mut self,
        node_id: UzNodeId,
        style: &UzStyle,
        layout: &LayoutSnapshot,
        view_bounds: Bounds,
        transform: Affine,
    ) -> Option<ViewScroll> {
        let scroll_x = style.overflow_x.is_scrollable();
        let scroll_y = style.overflow_y.is_scrollable();
        if !scroll_x && !scroll_y {
            return None;
        }

        let max_x = (layout.content_w - layout.size_w).max(0.0);
        let max_y = (layout.content_h - layout.size_h).max(0.0);

        if self.dom.nodes[node_id].scroll_state.is_none() {
            self.dom.nodes[node_id].scroll_state = Some(ScrollState::new());
        }
        let ss = self.dom.nodes[node_id].scroll_state.as_mut().unwrap();
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
        if scroll_y && layout.content_h > layout.size_h {
            thumbs.push(self.register_view_thumb(
                node_id,
                ScrollAxis::Y,
                view_bounds,
                layout.content_h,
                layout.size_h,
                offset_y,
                transform,
                &style.scrollbar,
            ));
        }
        if scroll_x && layout.content_w > layout.size_w {
            thumbs.push(self.register_view_thumb(
                node_id,
                ScrollAxis::X,
                view_bounds,
                layout.content_w,
                layout.size_w,
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

        let hovered = self.is_active_drag(node_id)
            || self
                .dom
                .hit_state
                .mouse_position
                .is_some_and(|(mx, my)| thumb_bounds.contains(mx, my));

        ScrollbarPaint {
            transform,
            geom,
            hovered,
            style: *scrollbar,
        }
    }

    fn is_active_drag(&self, node_id: UzNodeId) -> bool {
        self.dom
            .scroll_drag
            .as_ref()
            .is_some_and(|d| d.node_id == node_id)
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

struct LayoutSnapshot {
    location_x: f64,
    location_y: f64,
    size_w: f64,
    size_h: f64,
    content_w: f64,
    content_h: f64,
}

impl From<&taffy::Layout> for LayoutSnapshot {
    fn from(l: &taffy::Layout) -> Self {
        Self {
            location_x: l.location.x as f64,
            location_y: l.location.y as f64,
            size_w: l.size.width as f64,
            size_h: l.size.height as f64,
            content_w: l.content_size.width as f64,
            content_h: l.content_size.height as f64,
        }
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
    style: ScrollbarStyle,
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
    use crate::element::ImageMeasureInfo;
    use crate::layout::NodeContext;
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
