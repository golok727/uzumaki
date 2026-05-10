use std::ops::{Deref, DerefMut};

use crate::cursor::UzCursorIcon;
use crate::element::{ElementNode, ImageNode, TextContent};
use crate::input::InputState;
use crate::interactivity::{HitboxId, StyleVariants};
use crate::style::{TextSelectable, UzStyle};
use crate::text::TextBrush;
use parley::Layout as ParleyLayout;

pub type UzNodeId = usize;

/// Which axis a scroll operation targets. Used by drag/wheel routing and by
/// the unified scrollbar geometry helpers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScrollAxis {
    X,
    Y,
}

#[derive(Default)]
pub struct ScrollState {
    pub scroll_offset_x: f32,
    pub scroll_offset_y: f32,
}

impl ScrollState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn offset(&self, axis: ScrollAxis) -> f32 {
        match axis {
            ScrollAxis::X => self.scroll_offset_x,
            ScrollAxis::Y => self.scroll_offset_y,
        }
    }

    pub fn set_offset(&mut self, axis: ScrollAxis, value: f32) {
        match axis {
            ScrollAxis::X => self.scroll_offset_x = value,
            ScrollAxis::Y => self.scroll_offset_y = value,
        }
    }

    pub fn scroll_input_x(
        &mut self,
        cursor_left: f32,
        cursor_right: f32,
        natural_w: f32,
        visible_width: f32,
    ) {
        if visible_width <= 0.0 {
            return;
        }
        if cursor_left - self.scroll_offset_x < 0.0 {
            self.scroll_offset_x = cursor_left;
        } else if cursor_right - self.scroll_offset_x > visible_width {
            self.scroll_offset_x = cursor_right - visible_width;
        }
        let max_scroll = (natural_w - visible_width).max(0.0);
        self.scroll_offset_x = self.scroll_offset_x.clamp(0.0, max_scroll);
    }

    pub fn scroll_single_line_input_end(&mut self, natural_w: f32, visible_width: f32) {
        if visible_width <= 0.0 {
            return;
        }
        let max_scroll = (natural_w - visible_width).max(0.0);
        self.scroll_offset_x = max_scroll;
    }

    pub fn scroll_input_y(&mut self, cursor_y: f32, line_height: f32, visible_height: f32) {
        if visible_height <= 0.0 {
            return;
        }
        let cursor_bottom = cursor_y + line_height;
        if cursor_y < self.scroll_offset_y {
            self.scroll_offset_y = cursor_y;
        } else if cursor_bottom > self.scroll_offset_y + visible_height {
            self.scroll_offset_y = cursor_bottom - visible_height;
        }
        self.scroll_offset_y = self.scroll_offset_y.max(0.0);
    }
}

pub struct Node {
    pub parent: Option<UzNodeId>,

    pub children: Vec<UzNodeId>,

    pub data: NodeData,

    /// The base style for this element. Converted to taffy for layout.
    pub style: UzStyle,
    /// Hover/active/focus style refinements.
    pub style_variants: StyleVariants,
    /// Hitbox assigned during the latest paint pass. None if not painted yet.
    pub hitbox_id: Option<HitboxId>,
    /// Per-node scroll offsets for content that can scroll on either axis.
    pub scroll_state: ScrollState,
    /// Cached parley layout for text-bearing nodes (text node or `<text>`
    /// element). Refreshed once per frame after taffy compute, then reused by
    /// paint, selection geometry and hit-testing instead of rebuilding parley
    /// layouts on every read.
    pub text_layout: Option<ParleyLayout<TextBrush>>,
    /// Cached taffy layout for this node, copied here after `compute_layout`
    /// runs. Reading `node.final_layout` avoids the
    /// `layout_engine.layout(node_id)` two-level lookup on the paint hot path.
    pub final_layout: taffy::Layout,
}

impl Node {
    pub fn new(style: UzStyle, data: impl Into<NodeData>) -> Self {
        Self {
            parent: None,
            children: Vec::new(),
            data: data.into(),
            style,
            style_variants: StyleVariants::new(),
            hitbox_id: None,
            scroll_state: ScrollState::new(),
            text_layout: None,
            final_layout: taffy::Layout::new(),
        }
    }
}

impl Node {
    #[inline]
    pub fn text_selectable(&self) -> TextSelectable {
        self.style.text_selectable
    }

    pub fn is_text_selectable(&self) -> bool {
        self.style.text_selectable.selectable()
    }

    pub fn set_text_selectable(&mut self, text_selectable: TextSelectable) {
        self.style.text_selectable = text_selectable
    }

    pub fn as_text_input(&self) -> Option<&InputState> {
        self.data.as_text_input()
    }

    pub fn as_text_input_mut(&mut self) -> Option<&mut InputState> {
        self.data.as_text_input_mut()
    }

    pub fn as_checkbox_input(&self) -> Option<&bool> {
        self.data.as_checkbox_input()
    }

    pub fn as_checkbox_input_mut(&mut self) -> Option<&mut bool> {
        self.data.as_checkbox_input_mut()
    }

    pub fn as_element(&self) -> Option<&ElementNode> {
        self.data.as_element()
    }

    pub fn as_element_mut(&mut self) -> Option<&mut ElementNode> {
        self.data.as_element_mut()
    }

    pub fn get_text_content(&self) -> Option<&TextContent> {
        self.data.get_text_content()
    }

    pub fn text_content_mut(&mut self) -> Option<&mut TextContent> {
        self.data.text_content_mut()
    }

    pub fn as_image(&self) -> Option<&ImageNode> {
        self.data.as_image()
    }

    pub fn as_image_mut(&mut self) -> Option<&mut ImageNode> {
        self.data.as_image_mut()
    }

    pub fn is_text_input(&self) -> bool {
        self.data.is_text_input()
    }

    pub fn is_checkbox_input(&self) -> bool {
        self.data.is_checkbox_input()
    }

    pub fn is_image(&self) -> bool {
        self.data.is_image()
    }

    pub fn is_button(&self) -> bool {
        self.data.is_button()
    }

    pub fn is_keyboard_activatable(&self) -> bool {
        self.data.is_keyboard_activatable()
    }

    pub fn is_text_node(&self) -> bool {
        self.data.is_text_node()
    }

    pub fn default_cursor(&self) -> Option<UzCursorIcon> {
        self.data.default_cursor()
    }

    /// Whether this node can receive keyboard focus.
    pub fn is_focusable(&self) -> bool {
        self.as_element()
            .map(|e| e.is_focussable())
            .unwrap_or(false)
    }
}

pub struct TextNode(TextContent);

impl TextNode {
    pub fn new(content: TextContent) -> Self {
        Self(content)
    }
}

impl Deref for TextNode {
    type Target = TextContent;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TextNode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<TextNode> for NodeData {
    fn from(value: TextNode) -> Self {
        Self::Text(value)
    }
}

pub enum NodeData {
    Root,
    // normal text nodes (cant add event listners etc just plain text)
    Text(TextNode),
    // element node
    Element(ElementNode),
}

impl From<ElementNode> for NodeData {
    fn from(value: ElementNode) -> Self {
        Self::Element(value)
    }
}

impl NodeData {
    pub fn default_cursor(&self) -> Option<UzCursorIcon> {
        match self {
            Self::Element(element) => element.data.default_cursor(),
            // Plain text labels should inherit the cursor from their container.
            // Text cursor is handled separately for inputs and textSelect content.
            Self::Text(_) => None,
            _ => None,
        }
    }

    pub fn create_root() -> Self {
        Self::Root
    }

    pub fn get_text_content(&self) -> Option<&TextContent> {
        match self {
            Self::Text(text) => Some(&text.0),
            Self::Element(element) => element.data.get_text_content(),
            _ => None,
        }
    }

    pub fn text_content_mut(&mut self) -> Option<&mut TextContent> {
        match self {
            Self::Text(text) => Some(text),
            Self::Element(element) => element.data.text_content_mut(),
            _ => None,
        }
    }

    pub fn as_text_input(&self) -> Option<&InputState> {
        match self {
            Self::Element(element) => element.data.as_text_input(),
            _ => None,
        }
    }

    pub fn as_text_input_mut(&mut self) -> Option<&mut InputState> {
        match self {
            Self::Element(element) => element.data.as_text_input_mut(),
            _ => None,
        }
    }

    pub fn as_checkbox_input(&self) -> Option<&bool> {
        match self {
            Self::Element(element) => element.data.as_checkbox_input(),
            _ => None,
        }
    }

    pub fn as_checkbox_input_mut(&mut self) -> Option<&mut bool> {
        match self {
            Self::Element(element) => element.data.as_checkbox_input_mut(),
            _ => None,
        }
    }

    pub fn as_image(&self) -> Option<&ImageNode> {
        match self {
            Self::Element(element) => element.data.as_image(),
            _ => None,
        }
    }

    pub fn as_image_mut(&mut self) -> Option<&mut ImageNode> {
        match self {
            Self::Element(element) => element.data.as_image_mut(),
            _ => None,
        }
    }

    pub fn is_text_node(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    pub fn is_text_input(&self) -> bool {
        match self {
            Self::Element(element) => element.data.is_text_input(),
            _ => false,
        }
    }

    pub fn is_checkbox_input(&self) -> bool {
        match self {
            Self::Element(element) => element.data.is_checkbox_input(),
            _ => false,
        }
    }

    pub fn is_image(&self) -> bool {
        match self {
            Self::Element(element) => element.data.is_image(),
            _ => false,
        }
    }

    pub fn is_button(&self) -> bool {
        match self {
            Self::Element(element) => element.is_button(),
            _ => false,
        }
    }

    pub fn is_keyboard_activatable(&self) -> bool {
        match self {
            Self::Element(element) => element.is_keyboard_activatable(),
            _ => false,
        }
    }

    pub fn is_element(&self) -> bool {
        matches!(self, Self::Element(_))
    }

    pub fn is_root(&self) -> bool {
        matches!(self, Self::Root)
    }

    pub fn as_element(&self) -> Option<&ElementNode> {
        match self {
            Self::Element(element) => Some(element),
            _ => None,
        }
    }

    pub fn as_element_mut(&mut self) -> Option<&mut ElementNode> {
        match self {
            Self::Element(element) => Some(element),
            _ => None,
        }
    }

    pub fn as_element_kind(&self) -> Option<&ElementNode> {
        match self {
            Self::Element(element) => Some(element),
            _ => None,
        }
    }

    pub fn as_element_kind_mut(&mut self) -> Option<&mut ElementNode> {
        match self {
            Self::Element(element) => Some(element),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ScrollState;

    #[test]
    fn input_scroll_scrolls_right() {
        let mut scroll = ScrollState::new();
        scroll.scroll_input_x(250.0, 251.5, 300.0, 200.0);
        assert_eq!(scroll.scroll_offset_x, 51.5);
    }

    #[test]
    fn input_scroll_scrolls_left() {
        let mut scroll = ScrollState {
            scroll_offset_x: 100.0,
            scroll_offset_y: 0.0,
        };
        scroll.scroll_input_x(50.0, 51.5, 300.0, 200.0);
        assert_eq!(scroll.scroll_offset_x, 50.0);
    }

    #[test]
    fn input_scroll_no_negative() {
        let mut scroll = ScrollState {
            scroll_offset_x: -10.0,
            scroll_offset_y: 0.0,
        };
        scroll.scroll_input_x(50.0, 51.5, 300.0, 200.0);
        assert!(scroll.scroll_offset_x >= 0.0);
    }

    #[test]
    fn input_scroll_clamps_to_natural_width() {
        let mut scroll = ScrollState {
            scroll_offset_x: 200.0,
            scroll_offset_y: 0.0,
        };
        scroll.scroll_input_x(30.0, 31.5, 30.0, 200.0);
        assert_eq!(scroll.scroll_offset_x, 0.0);
    }

    #[test]
    fn input_scroll_keeps_full_caret_visible() {
        let mut scroll = ScrollState::new();
        scroll.scroll_input_x(198.5, 200.0, 200.0, 200.0);
        assert_eq!(scroll.scroll_offset_x, 0.0);
    }

    #[test]
    fn input_scroll_does_not_scroll_past_natural_width_for_caret_width() {
        let mut scroll = ScrollState::new();
        scroll.scroll_input_x(200.0, 201.5, 200.0, 200.0);
        assert_eq!(scroll.scroll_offset_x, 0.0);
    }

    #[test]
    fn single_line_input_end_scrolls_to_natural_width() {
        let mut scroll = ScrollState::new();
        scroll.scroll_single_line_input_end(300.0, 200.0);
        assert_eq!(scroll.scroll_offset_x, 100.0);
    }

    #[test]
    fn single_line_input_end_does_not_scroll_when_text_fits() {
        let mut scroll = ScrollState {
            scroll_offset_x: 20.0,
            scroll_offset_y: 0.0,
        };
        scroll.scroll_single_line_input_end(180.0, 200.0);
        assert_eq!(scroll.scroll_offset_x, 0.0);
    }

    #[test]
    fn input_scroll_y_scrolls_down() {
        let mut scroll = ScrollState::new();
        scroll.scroll_input_y(250.0, 20.0, 200.0);
        assert_eq!(scroll.scroll_offset_y, 70.0);
    }

    #[test]
    fn input_scroll_y_scrolls_up() {
        let mut scroll = ScrollState {
            scroll_offset_x: 0.0,
            scroll_offset_y: 100.0,
        };
        scroll.scroll_input_y(50.0, 20.0, 200.0);
        assert_eq!(scroll.scroll_offset_y, 50.0);
    }
}
