use crate::cursor::UzCursorIcon;
use crate::input::InputState;
use crate::interactivity::Interactivity;
use crate::style::{Bounds, TextSelectable, UzStyle};

pub mod checkbox;
pub mod input;
pub mod render;
pub mod selection;
pub mod text;
pub mod view;

use vello::kurbo::Affine;

pub type UzNodeId = usize;

pub struct ScrollState {
    pub scroll_offset_y: f32,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollState {
    pub fn new() -> Self {
        Self {
            scroll_offset_y: 0.0,
        }
    }
}

/// Active scroll-thumb drag. Stored on Dom (only one drag at a time).
pub struct ScrollDragState {
    pub node_id: UzNodeId,
    pub start_mouse_y: f64,
    pub start_scroll_offset: f32,
    /// Track length = visible_height - thumb_height (how far thumb can move).
    pub track_range: f64,
    /// Max scroll offset (content_height - visible_height).
    pub max_scroll: f32,
}

/// Rendered thumb rect, rebuilt each paint pass for hit testing.
pub struct ScrollThumbRect {
    pub node_id: UzNodeId,
    pub thumb_bounds: Bounds,
    pub view_bounds: Bounds,
    pub content_height: f32,
    pub visible_height: f32,
}

#[derive(Clone, Debug)]
pub struct TextNode {
    pub content: String,
}

impl TextNode {
    pub fn new(content: String) -> Self {
        Self { content }
    }
}

// General-purpose mechanism for properties that propagate from parent to child
// unless explicitly overridden. Designed for extension — future inheritable
// properties (font color, font size, line height, etc.) go here.

#[derive(Clone, Debug, Default)]
pub struct InheritedProperties {
    pub text_selectable: bool,
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
    pub flat_text: String,
    pub total_graphemes: usize,
}

#[derive(Clone, Debug)]
pub struct NodeContext {
    pub dom_id: UzNodeId,
    pub text: Option<TextNode>,
    pub font_size: f32,
    pub is_input: bool,
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

    pub fn new_text_input(state: InputState) -> Self {
        Self::new(ElementData::TextInput(Box::new(state)))
    }

    pub fn new_checkbox_input(checked: bool) -> Self {
        Self::new(ElementData::CheckboxInput(checked))
    }

    pub fn is_text_input(&self) -> bool {
        self.data.is_text_input()
    }

    pub fn is_checkbox_input(&self) -> bool {
        self.data.is_checkbox_input()
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
    TextInput(Box<InputState>),
    CheckboxInput(bool),
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
}

pub enum NodeData {
    Root,

    Text(TextNode),
    // element node
    Element(ElementNode),
}

impl From<TextNode> for NodeData {
    fn from(value: TextNode) -> Self {
        Self::Text(value)
    }
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

    pub fn create_text(data: TextNode) -> Self {
        Self::Text(data)
    }

    pub fn create_element(data: ElementNode) -> Self {
        Self::Element(data)
    }

    pub fn as_text_node(&self) -> Option<&TextNode> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_text_node_mut(&mut self) -> Option<&mut TextNode> {
        match self {
            Self::Text(text) => Some(text),
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

    pub first_child: Option<UzNodeId>,

    pub last_child: Option<UzNodeId>,

    pub next_sibling: Option<UzNodeId>,

    pub prev_sibling: Option<UzNodeId>,

    pub taffy_node: taffy::NodeId,

    pub data: NodeData,

    /// The base style for this element. Converted to taffy for layout.
    pub style: UzStyle,
    /// Interactivity: hover/active style overrides, hitbox, event listeners.
    pub interactivity: Interactivity,
    /// Scroll state, present only when overflow_y == Scroll.
    pub scroll_state: Option<ScrollState>,
    // not used now todo use this :3
    pub transform: Option<Affine>,
}

impl Node {
    pub fn new(taffy_node: taffy::NodeId, style: UzStyle, data: impl Into<NodeData>) -> Self {
        Self {
            parent: None,
            first_child: None,
            last_child: None,
            next_sibling: None,
            prev_sibling: None,
            taffy_node,
            data: data.into(),
            style,
            interactivity: Interactivity::new(),
            scroll_state: None,
            transform: None,
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

    pub fn as_text_node(&self) -> Option<&TextNode> {
        self.data.as_text_node()
    }

    pub fn as_text_node_mut(&mut self) -> Option<&mut TextNode> {
        self.data.as_text_node_mut()
    }

    pub fn is_text_input(&self) -> bool {
        self.data.is_text_input()
    }

    pub fn is_checkbox_input(&self) -> bool {
        self.data.is_checkbox_input()
    }

    pub fn is_text_node(&self) -> bool {
        self.data.is_text_node()
    }

    pub fn default_cursor(&self) -> Option<UzCursorIcon> {
        self.data.default_cursor()
    }
}
