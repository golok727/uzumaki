use crate::node::UzNodeId;

#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SelectionRange {
    /// Anchor point (where selection started), flat grapheme index
    pub anchor: usize,
    /// Active point / cursor position, flat grapheme index
    pub active: usize,
}

impl SelectionRange {
    pub fn new(anchor: usize, active: usize) -> Self {
        Self { anchor, active }
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.active
    }

    pub fn start(&self) -> usize {
        self.anchor.min(self.active)
    }

    pub fn end(&self) -> usize {
        self.anchor.max(self.active)
    }

    pub fn set_cursor(&mut self, pos: usize) {
        self.anchor = pos;
        self.active = pos;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Affinity {
    #[default]
    Downstream,
    Upstream,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SelectionEndpoint {
    pub node: UzNodeId,
    pub offset: usize,
    pub affinity: Affinity,
}

impl SelectionEndpoint {
    pub fn new(node: UzNodeId, offset: usize, affinity: Affinity) -> Self {
        Self {
            node,
            offset,
            affinity,
        }
    }
}

/// Text selection for non-input elements (textSelect views).
///
/// `anchor == None || focus == None` means no active view selection. Input
/// nodes own their own selection internally and do not populate this.
#[derive(Debug, Copy, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TextSelection {
    pub anchor: Option<SelectionEndpoint>,
    pub focus: Option<SelectionEndpoint>,
}

impl TextSelection {
    pub fn new(anchor: SelectionEndpoint, focus: SelectionEndpoint) -> Self {
        Self {
            anchor: Some(anchor),
            focus: Some(focus),
        }
    }

    pub fn is_active(&self) -> bool {
        self.anchor.is_some() && self.focus.is_some() && !self.is_collapsed()
    }

    pub fn is_set(&self) -> bool {
        self.anchor.is_some() && self.focus.is_some()
    }

    pub fn ordered_with(
        &self,
        mut precedes_or_equal: impl FnMut(&SelectionEndpoint, &SelectionEndpoint) -> bool,
    ) -> Option<(&SelectionEndpoint, &SelectionEndpoint)> {
        let anchor = self.anchor.as_ref()?;
        let focus = self.focus.as_ref()?;
        if precedes_or_equal(anchor, focus) {
            Some((anchor, focus))
        } else {
            Some((focus, anchor))
        }
    }

    pub fn clear(&mut self) {
        self.anchor = None;
        self.focus = None;
    }

    #[inline]
    pub fn anchor_offset(&self) -> Option<usize> {
        self.anchor.map(|endpoint| endpoint.offset)
    }

    #[inline]
    pub fn focus_offset(&self) -> Option<usize> {
        self.focus.map(|endpoint| endpoint.offset)
    }

    pub fn is_collapsed(&self) -> bool {
        match (self.anchor, self.focus) {
            (Some(anchor), Some(focus)) => {
                anchor.node == focus.node && anchor.offset == focus.offset
            }
            _ => false,
        }
    }
}
