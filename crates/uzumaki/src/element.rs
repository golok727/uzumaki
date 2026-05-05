use crate::cursor::UzCursorIcon;
use crate::input::InputState;
use crate::interactivity::Interactivity;
use crate::style::{Bounds, TextSelectable, UzStyle};
use crate::text::TextBrush;
use parley::Layout as ParleyLayout;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use vello::peniko::Blob;

pub mod checkbox;
pub mod image;
pub mod input;
pub mod render;
pub mod scroll;
pub mod selection;
pub mod svg;
pub mod view;

use vello::kurbo::Affine;

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
}

/// Active scroll-thumb drag. Stored on the dom (only one drag at a time).
pub struct ScrollDragState {
    pub node_id: UzNodeId,
    pub axis: ScrollAxis,
    /// Mouse coordinate on `axis` at drag start (logical px).
    pub start_mouse_pos: f64,
    pub start_scroll_offset: f32,
    /// Distance the thumb can travel along `axis` (track length minus thumb
    /// length).
    pub track_range: f64,
    /// Maximum scroll offset along `axis` (content_size - visible_size).
    pub max_scroll: f32,
}

/// Rendered thumb rect, rebuilt each paint pass for hit testing.
pub struct ScrollThumbRect {
    pub node_id: UzNodeId,
    pub axis: ScrollAxis,
    pub thumb_bounds: Bounds,
    pub view_bounds: Bounds,
    pub content_size: f32,
    pub visible_size: f32,
}

#[derive(Clone, Debug)]
pub struct TextContent {
    pub content: String,
}

impl TextContent {
    pub fn new(content: String) -> Self {
        Self { content }
    }
}

#[derive(Clone, Debug)]
pub struct ImageMeasureInfo {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RasterImageData {
    pub width: u32,
    pub height: u32,
    pub data: Blob<u8>,
}

impl RasterImageData {
    pub fn new(width: u32, height: u32, data: Arc<Vec<u8>>) -> Self {
        Self {
            width,
            height,
            data: Blob::new(data),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum ImageData {
    Raster(RasterImageData),
    Svg {
        tree: Arc<usvg::Tree>,
        uses_current_color: bool,
    },
    #[default]
    None,
}

impl ImageData {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn natural_size(&self) -> Option<(f32, f32)> {
        match self {
            Self::Raster(r) => Some((r.width as f32, r.height as f32)),
            Self::Svg { tree, .. } => {
                let s = tree.size();
                Some((s.width(), s.height()))
            }
            Self::None => None,
        }
    }
}

impl From<RasterImageData> for ImageData {
    fn from(value: RasterImageData) -> Self {
        Self::Raster(value)
    }
}

impl From<usvg::Tree> for ImageData {
    fn from(value: usvg::Tree) -> Self {
        Self::Svg {
            tree: Arc::new(value),
            uses_current_color: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ImageNode {
    pub data: ImageData,
}

impl ImageNode {
    pub fn clear(&mut self) {
        self.data = ImageData::None;
    }
}

/// One text node's contribution to a textSelect run.
pub struct TextRunEntry {
    pub node_id: UzNodeId,
    /// Start grapheme index of this node in the flat run.
    pub flat_start: usize,
    pub grapheme_count: usize,
}

/// The complete text run for a textSelect subtree.
/// Built each frame; maps between flat grapheme offsets and per-node positions.
pub struct TextSelectRun {
    pub root_id: UzNodeId,
    pub entries: Vec<TextRunEntry>,
    pub total_graphemes: usize,
}

pub struct ElementNode {
    pub is_focussable: bool,
    pub data: ElementData,
}

impl ElementNode {
    pub fn new(data: ElementData) -> Self {
        Self {
            is_focussable: false,
            data,
        }
    }

    /**
     * Inline text element (for styling inline text) Hello <text> Something <text>
     *  Hello                    <text>Something</text>
     *   |                          |---------------|
     *NodeData::TextNode()   NodeData::ElementNode(ElementData::Text())
     */
    pub fn new_text(text: TextContent) -> Self {
        Self::new(ElementData::Text(text))
    }

    pub fn new_text_input(state: InputState) -> Self {
        Self::new(ElementData::TextInput(Box::new(state)))
    }

    pub fn new_checkbox_input(checked: bool) -> Self {
        Self::new(ElementData::CheckboxInput(checked))
    }

    pub fn new_image(state: ImageNode) -> Self {
        Self::new(ElementData::Image(Box::new(state)))
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

    pub fn is_focussable(&self) -> bool {
        self.is_focussable
    }

    pub fn set_focussable(&mut self, focussable: bool) {
        self.is_focussable = focussable;
    }
}

#[derive(Default)]
pub enum ElementData {
    // this is text Element <text>
    Text(TextContent),
    TextInput(Box<InputState>),
    CheckboxInput(bool),
    Image(Box<ImageNode>),
    // for view nodes
    #[default]
    None,
}

impl ElementData {
    pub fn default_cursor(&self) -> Option<UzCursorIcon> {
        match self {
            Self::TextInput(_) => Some(UzCursorIcon::Text),
            Self::CheckboxInput(_) => Some(UzCursorIcon::Pointer),
            _ => None,
        }
    }

    pub fn is_text_input(&self) -> bool {
        matches!(self, Self::TextInput(_))
    }

    pub fn is_checkbox_input(&self) -> bool {
        matches!(self, Self::CheckboxInput(_))
    }

    pub fn is_image(&self) -> bool {
        matches!(self, Self::Image(_))
    }

    pub fn get_text_content(&self) -> Option<&TextContent> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn text_content_mut(&mut self) -> Option<&mut TextContent> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_text_input(&self) -> Option<&InputState> {
        match self {
            Self::TextInput(state) => Some(state),
            _ => None,
        }
    }

    pub fn as_text_input_mut(&mut self) -> Option<&mut InputState> {
        match self {
            Self::TextInput(state) => Some(state),
            _ => None,
        }
    }

    pub fn as_checkbox_input(&self) -> Option<&bool> {
        match self {
            Self::CheckboxInput(checked) => Some(checked),
            _ => None,
        }
    }

    pub fn as_checkbox_input_mut(&mut self) -> Option<&mut bool> {
        match self {
            Self::CheckboxInput(checked) => Some(checked),
            _ => None,
        }
    }

    pub fn as_image(&self) -> Option<&ImageNode> {
        match self {
            Self::Image(image) => Some(image),
            _ => None,
        }
    }

    pub fn as_image_mut(&mut self) -> Option<&mut ImageNode> {
        match self {
            Self::Image(image) => Some(image),
            _ => None,
        }
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

pub struct Node {
    pub parent: Option<UzNodeId>,

    pub children: Vec<UzNodeId>,

    pub data: NodeData,

    /// The base style for this element. Converted to taffy for layout.
    pub style: UzStyle,
    /// Interactivity: hover/active style overrides, hitbox, event listeners.
    pub interactivity: Interactivity,
    /// Scroll state, present only when overflow_y == Scroll.
    pub scroll_state: Option<ScrollState>,
    // not used now todo use this :3
    pub transform: Option<Affine>,
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
            interactivity: Interactivity::new(),
            scroll_state: None,
            transform: None,
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
