use std::sync::Arc;

use crate::cursor::UzCursorIcon;
use crate::input::InputState;
use crate::node::UzNodeId;
use vello::peniko::Blob;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ElementKind {
    #[default]
    View,
    Text,
    Button,
    Input,
    Checkbox,
    Image,
}

impl ElementKind {
    pub fn is_keyboard_activatable(self) -> bool {
        matches!(self, Self::Button | Self::Checkbox)
    }
}

pub struct ElementNode {
    pub kind: ElementKind,
    pub is_focussable: bool,
    pub data: ElementData,
}

impl ElementNode {
    pub fn new(kind: ElementKind, data: ElementData) -> Self {
        Self {
            kind,
            is_focussable: false,
            data,
        }
    }

    pub fn new_view() -> Self {
        Self::new(ElementKind::View, ElementData::None)
    }

    pub fn new_button() -> Self {
        let mut element = Self::new(ElementKind::Button, ElementData::None);
        element.set_focussable(true);
        element
    }

    /**
     * Inline text element (for styling inline text) Hello <text> Something <text>
     *  Hello                    <text>Something</text>
     *   |                          |---------------|
     *NodeData::TextNode()   NodeData::ElementNode(ElementData::Text())
     */
    pub fn new_text(text: TextContent) -> Self {
        Self::new(ElementKind::Text, ElementData::Text(text))
    }

    pub fn new_text_input(state: InputState) -> Self {
        Self::new(ElementKind::Input, ElementData::TextInput(Box::new(state)))
    }

    pub fn new_checkbox_input(checked: bool) -> Self {
        Self::new(ElementKind::Checkbox, ElementData::CheckboxInput(checked))
    }

    pub fn new_image(state: ImageNode) -> Self {
        Self::new(ElementKind::Image, ElementData::Image(Box::new(state)))
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
        self.kind == ElementKind::Button
    }

    pub fn is_keyboard_activatable(&self) -> bool {
        self.kind.is_keyboard_activatable()
    }

    pub fn is_focussable(&self) -> bool {
        self.is_focussable
    }

    pub fn set_focussable(&mut self, focussable: bool) {
        self.is_focussable = focussable;
    }
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
