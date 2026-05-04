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
use crate::style::{Bounds, TextStyle, UzStyle, Visibility};
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

    pub fn prepaint(mut self) -> PaintList {
        self.dom.hitbox_store.clear();
        self.dom.scroll_thumbs.clear();
        for (_, node) in self.dom.nodes.iter_mut() {
            node.interactivity.hitbox_id = None;
        }
        self.dom.build_text_select_runs();

        let text_selections = self.compute_text_selections_map();
        let mut items: Vec<PaintItem> = Vec::new();

        if let Some(root) = self.dom.root {
            let mut stack: Vec<Frame> = vec![Frame::Visit(VisitFrame {
                node_id: root,
                parent_x: 0.0,
                parent_y: 0.0,
                parent_paint_transform: Affine::scale(self.scale),
                parent_hit_transform: Affine::IDENTITY,
                parent_style: None,
            })];

            while let Some(frame) = stack.pop() {
                match frame {
                    Frame::Visit(v) => self.visit(v, &mut stack, &mut items),
                    Frame::PushClip { rect, transform } => {
                        items.push(PaintItem::PushClip { rect, transform })
                    }
                    Frame::PopClip => items.push(PaintItem::PopClip),
                    Frame::Scrollbar(s) => items.push(PaintItem::Scrollbar(s)),
                }
            }
        }

        PaintList {
            items,
            text_selections,
        }
    }

    fn visit(&mut self, frame: VisitFrame, stack: &mut Vec<Frame>, items: &mut Vec<PaintItem>) {
        let VisitFrame {
            node_id,
            parent_x,
            parent_y,
            parent_paint_transform,
            parent_hit_transform,
            parent_style,
        } = frame;

        let snap = match self.snapshot(node_id, parent_style.as_deref()) {
            Some(snap) => snap,
            None => return,
        };

        let layout = match self.dom.layout_engine.layout(node_id) {
            Some(l) => LayoutSnapshot::from(l),
            None => return,
        };

        let x = parent_x + layout.location_x;
        let y = parent_y + layout.location_y;
        let w = layout.size_w;
        let h = layout.size_h;
        let local_style_transform = snap.computed_style.transform.to_affine(w, h);
        let local_translate = Affine::translate((layout.location_x, layout.location_y));
        let transform = parent_paint_transform * local_translate * local_style_transform;
        let hit_transform = parent_hit_transform * local_translate * local_style_transform;

        // Build per-frame input render info (mutates editor) BEFORE we register
        // hitboxes — we want geometry for the scrollbar that follows.
        let input = snap
            .input_seed
            .map(|seed| self.build_input_render_info(node_id, &snap.computed_style, &layout, seed));

        let input_scrollbar = input.as_ref().and_then(|info| {
            self.register_input_scrollbar(
                node_id,
                &snap.computed_style,
                w,
                h,
                info,
                transform,
                x,
                y,
            )
        });

        // Every visible box participates in hit testing.
        let hitbox_id = self.dom.hitbox_store.insert_transformed(
            node_id,
            Bounds::new(0.0, 0.0, w, h),
            hit_transform,
        );
        self.dom.nodes[node_id].interactivity.hitbox_id = Some(hitbox_id);

        let view_scroll = self.prepare_view_scroll(
            node_id,
            &snap.computed_style,
            &layout,
            Bounds::new(x, y, w, h),
            transform,
        );

        let mut children = Vec::new();
        let mut next = snap.first_child;
        while let Some(child_id) = next {
            children.push(child_id);
            next = self.dom.nodes[child_id].next_sibling;
        }

        let content = if let Some(input_info) = input {
            NodeContent::Input(input_info)
        } else if let Some(cb) = snap.checkbox {
            NodeContent::Checkbox(cb)
        } else if let Some(img) = snap.image {
            NodeContent::Image(img)
        } else if let Some((text, style)) = snap.text {
            NodeContent::Text { text, style }
        } else {
            NodeContent::View
        };

        items.push(PaintItem::Node(Box::new(NodePaint {
            node_id,
            bounds: Bounds::new(0.0, 0.0, w, h),
            transform,
            style: Box::new(snap.computed_style.clone()),
            content,
        })));

        // Input scrollbar paints immediately after the input itself, OUTSIDE
        // the input's internal text clip.
        if let Some(thumb) = input_scrollbar {
            items.push(PaintItem::Scrollbar(thumb));
        }

        // Children + clip + view scrollbars. The stack is LIFO, so push in
        // reverse of execution order: PushClip → children → PopClip → thumbs.
        let needs_clip = view_scroll.is_some()
            || snap.computed_style.overflow_x.clips()
            || snap.computed_style.overflow_y.clips();
        let (offset_x, offset_y, mouse_in_view, view_thumbs) = match view_scroll {
            Some(v) => (v.offset_x, v.offset_y, v.mouse_in_view, v.thumbs),
            None => (0.0, 0.0, false, Vec::new()),
        };

        if mouse_in_view {
            for thumb in view_thumbs.into_iter().rev() {
                stack.push(Frame::Scrollbar(thumb));
            }
        }
        if needs_clip {
            stack.push(Frame::PopClip);
        }

        let scroll_translate = if offset_x != 0.0 || offset_y != 0.0 {
            Affine::translate((-offset_x, -offset_y))
        } else {
            Affine::IDENTITY
        };
        let child_paint_transform = transform * scroll_translate;
        let child_hit_transform = hit_transform * scroll_translate;
        for &child_id in children.iter().rev() {
            stack.push(Frame::Visit(VisitFrame {
                node_id: child_id,
                parent_x: x - offset_x,
                parent_y: y - offset_y,
                parent_paint_transform: child_paint_transform,
                parent_hit_transform: child_hit_transform,
                parent_style: Some(Box::new(snap.computed_style.clone())),
            }));
        }
        if needs_clip {
            stack.push(Frame::PushClip {
                rect: Rect::new(0.0, 0.0, w, h),
                transform,
            });
        }
    }

    /// Pull everything we need from the node into local state. Returning `None`
    /// means the node is hidden and should be skipped.
    fn snapshot(&self, node_id: UzNodeId, parent_style: Option<&UzStyle>) -> Option<NodeSnapshot> {
        let computed_style = self.dom.computed_style(node_id, parent_style);
        if computed_style.visibility == Visibility::Hidden
            || computed_style.display == crate::style::Display::None
        {
            return None;
        }

        let node = &self.dom.nodes[node_id];
        let first_child = node.first_child;

        let text = node
            .get_text_content()
            .map(|tc| (tc.content.clone(), computed_style.text.clone()));

        let input_seed = node.is_text_input().then(|| {
            let is = node.as_text_input().unwrap();
            let focused = self.dom.focused_node == Some(node_id);
            InputSeed {
                display_text: is.display_text(),
                placeholder: is.placeholder.clone(),
                focused,
                scroll_offset: is.scroll_offset,
                scroll_offset_y: is.scroll_offset_y,
                blink_visible: is.blink_visible(focused, self.dom.window_focused),
                multiline: is.multiline,
                preedit: is.preedit.clone(),
            }
        });

        let checkbox = node
            .as_checkbox_input()
            .copied()
            .map(|checked| CheckboxRenderInfo {
                checked,
                focused: self.dom.focused_node == Some(node_id),
            });

        let image = node.as_image().map(|image| ImageRenderInfo {
            data: image.data.clone(),
        });

        Some(NodeSnapshot {
            computed_style,
            first_child,
            text,
            input_seed,
            checkbox,
            image,
        })
    }

    fn build_input_render_info(
        &mut self,
        node_id: UzNodeId,
        style: &UzStyle,
        layout: &LayoutSnapshot,
        seed: InputSeed,
    ) -> InputRenderInfo {
        let pad_h = style.padding.left + style.padding.right;
        let text_w = (layout.size_w as f32 - pad_h).max(0.0);
        let text_style = style.text.clone();

        let node_mut = &mut self.dom.nodes[node_id];
        let is = node_mut.as_text_input_mut().unwrap();

        apply_text_style_to_editor(&mut is.editor, &text_style);
        is.editor
            .set_width(if seed.multiline { Some(text_w) } else { None });
        is.editor.refresh_layout(
            &mut self.text_renderer.font_ctx,
            &mut self.text_renderer.layout_ctx,
        );

        let cursor_rect = if seed.blink_visible || seed.preedit.is_some() {
            if is.secure {
                secure_cursor_geometry(&is.editor, 1.5, &text_style, self.text_renderer)
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

        let layout_height = is.editor.try_layout().map(|l| l.height()).unwrap_or(0.0);

        let preedit = seed.preedit.map(|ps| {
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

        InputRenderInfo {
            display_text: seed.display_text,
            placeholder: seed.placeholder,
            text_style,
            focused: seed.focused,
            cursor_rect,
            selection_rects,
            scroll_offset: seed.scroll_offset,
            scroll_offset_y: seed.scroll_offset_y,
            blink_visible: seed.blink_visible,
            multiline: seed.multiline,
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
        let geom = scroll::thumb_geometry(ScrollAxis::Y, view_local, axis_info);
        let view_bounds = Bounds::new(view_x, view_y, w, h);
        let thumb_bounds = Bounds::new(
            view_x + geom.local_x,
            view_y + geom.local_y,
            geom.width,
            geom.height,
        );
        // Inputs always register the thumb hit rect, even when not visible —
        // matches the legacy behaviour and keeps wheel/drag responsive.
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
        })
    }

    /// For scrollable views, ensure scroll state exists, clamp offsets, and
    /// register a hit rect per scrollable axis. Returns the resolved offsets
    /// (so the child walk can translate into scrolled space) plus the per-axis
    /// thumb paint commands and a flag for whether to actually emit them.
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
        );
        let thumb_bounds = Bounds::new(
            view_bounds.x + geom.local_x,
            view_bounds.y + geom.local_y,
            geom.width,
            geom.height,
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
        }
    }

    fn is_active_drag(&self, node_id: UzNodeId) -> bool {
        self.dom
            .scroll_drag
            .as_ref()
            .is_some_and(|d| d.node_id == node_id)
    }

    /// Pre-compute selection ranges for text-select mode. This depends only on
    /// `dom.text_selection`, which is settled before prepaint runs, so a
    /// re-prepaint after a hit-test refresh produces the same map.
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

// ---------------------------------------------------------------------------
// Paint
// ---------------------------------------------------------------------------

pub struct PaintList {
    items: Vec<PaintItem>,
    text_selections: HashMap<UzNodeId, (usize, usize)>,
}

impl PaintList {
    pub fn paint(&self, scene: &mut Scene, text_renderer: &mut TextRenderer) {
        for item in &self.items {
            match item {
                PaintItem::Node(node) => {
                    paint_node(node, scene, text_renderer, &self.text_selections)
                }
                PaintItem::PushClip { rect, transform } => {
                    scene.push_clip_layer(Fill::NonZero, *transform, rect);
                }
                PaintItem::PopClip => scene.pop_layer(),
                PaintItem::Scrollbar(s) => {
                    scroll::paint_thumb(scene, s.transform, &s.geom, s.hovered)
                }
            }
        }
    }
}

fn paint_node(
    node: &NodePaint,
    scene: &mut Scene,
    text_renderer: &mut TextRenderer,
    text_selections: &HashMap<UzNodeId, (usize, usize)>,
) {
    match &node.content {
        NodeContent::Input(input) => {
            crate::element::input::paint_input(
                scene,
                text_renderer,
                node.bounds,
                &node.style,
                input,
                node.transform,
            );
        }
        NodeContent::Checkbox(cb) => {
            crate::element::checkbox::paint_checkbox(
                scene,
                node.bounds,
                &node.style,
                cb,
                node.transform,
            );
        }
        NodeContent::Image(img) => {
            crate::element::image::paint_image(
                scene,
                node.bounds,
                &node.style,
                img,
                node.transform,
            );
        }
        NodeContent::Text { text, style } => {
            if let Some((sel_start, sel_end)) = text_selections.get(&node.node_id).copied() {
                node.style
                    .paint(node.bounds, scene, node.transform, |scene| {
                        let rects = text_renderer.selection_rects(
                            text,
                            style,
                            Some(node.bounds.width as f32),
                            sel_start,
                            sel_end,
                        );
                        let sel_color = VelloColor::from_rgba8(56, 121, 185, 128);
                        for rect in rects {
                            scene.fill(
                                Fill::NonZero,
                                node.transform,
                                sel_color,
                                None,
                                &Rect::new(rect.x0, rect.y0, rect.x1, rect.y1),
                            );
                        }
                        text_renderer.draw_text(
                            scene,
                            text,
                            style,
                            node.bounds.width as f32,
                            node.bounds.height as f32,
                            (0.0, 0.0),
                            style.color.to_vello(),
                            node.transform,
                        );
                    });
            } else {
                crate::element::text::paint_text(
                    scene,
                    text_renderer,
                    node.bounds,
                    &node.style,
                    text,
                    style,
                    style.color,
                    node.transform,
                );
            }
        }
        NodeContent::View => {
            crate::element::view::paint_view(
                scene,
                node.bounds,
                &node.style,
                node.transform,
                |_| {},
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

struct NodeSnapshot {
    computed_style: UzStyle,
    first_child: Option<UzNodeId>,
    text: Option<(String, TextStyle)>,
    input_seed: Option<InputSeed>,
    checkbox: Option<CheckboxRenderInfo>,
    image: Option<ImageRenderInfo>,
}

struct InputSeed {
    display_text: String,
    placeholder: String,
    focused: bool,
    scroll_offset: f32,
    scroll_offset_y: f32,
    blink_visible: bool,
    multiline: bool,
    preedit: Option<crate::input::PreeditState>,
}

/// Layout fields copied out of taffy so we can mutate dom while reading them.
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

struct NodePaint {
    node_id: UzNodeId,
    bounds: Bounds,
    transform: Affine,
    style: Box<UzStyle>,
    content: NodeContent,
}

enum NodeContent {
    View,
    Text { text: String, style: TextStyle },
    Input(InputRenderInfo),
    Checkbox(CheckboxRenderInfo),
    Image(ImageRenderInfo),
}

#[derive(Clone)]
struct ScrollbarPaint {
    transform: Affine,
    geom: ThumbGeometry,
    hovered: bool,
}

enum PaintItem {
    Node(Box<NodePaint>),
    PushClip { rect: Rect, transform: Affine },
    PopClip,
    Scrollbar(ScrollbarPaint),
}

struct VisitFrame {
    node_id: UzNodeId,
    parent_x: f64,
    parent_y: f64,
    parent_paint_transform: Affine,
    parent_hit_transform: Affine,
    parent_style: Option<Box<UzStyle>>,
}

enum Frame {
    Visit(VisitFrame),
    PushClip { rect: Rect, transform: Affine },
    PopClip,
    Scrollbar(ScrollbarPaint),
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
