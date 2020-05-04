use super::*;
use std::{borrow::Cow, f32};

#[derive(Debug, Clone, PartialEq)]
pub struct OwnedVariedSection {
    /// Position on screen to render text, in pixels from top-left. Defaults to (0, 0).
    pub screen_position: (f32, f32),
    /// Max (width, height) bounds, in pixels from top-left. Defaults to unbounded.
    pub bounds: (f32, f32),
    /// Z values for use in depth testing. Defaults to 0.0
    pub z: f32,
    /// Built in layout, can be overridden with custom layout logic
    /// see [`queue_custom_layout`](struct.GlyphBrush.html#method.queue_custom_layout)
    pub layout: Layout<BuiltInLineBreaker>,
    /// Text to render, rendered next to one another according the layout.
    pub text: Vec<OwnedVariedSectionText>,
}

impl Default for OwnedVariedSection {
    fn default() -> Self {
        Self {
            screen_position: (0.0, 0.0),
            bounds: (f32::INFINITY, f32::INFINITY),
            z: 0.0,
            layout: Layout::default(),
            text: vec![],
        }
    }
}

impl OwnedVariedSection {
    pub fn to_borrowed(&self) -> VariedSection<'_> {
        VariedSection {
            screen_position: self.screen_position,
            bounds: self.bounds,
            z: self.z,
            layout: self.layout,
            text: self.text.iter().map(|t| t.into()).collect(),
        }
    }
}

impl<'a> From<&'a OwnedVariedSection> for VariedSection<'a> {
    fn from(owned: &'a OwnedVariedSection) -> Self {
        owned.to_borrowed()
    }
}

impl<'a> From<&'a OwnedVariedSection> for Cow<'a, VariedSection<'a>> {
    fn from(owned: &'a OwnedVariedSection) -> Self {
        Cow::Owned(owned.to_borrowed())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OwnedVariedSectionText {
    /// Text to render
    pub text: String,
    /// Position on screen to render text, in pixels from top-left. Defaults to (0, 0).
    pub scale: PxScale,
    /// Font id to use for this section.
    ///
    /// It must be known to the `GlyphBrush` it is being used with,
    /// either `FontId::default()` or the return of
    /// [`add_font`](struct.GlyphBrushBuilder.html#method.add_font).
    pub font_id: FontId,
    /// Color
    pub color: Color,
}

impl OwnedVariedSectionText {
    pub fn from_text_and_color(st: &SectionText<'_>, color: Color) -> Self {
        Self {
            text: st.text.into(),
            scale: st.scale,
            font_id: st.font_id,
            color,
        }
    }
}

impl Default for OwnedVariedSectionText {
    fn default() -> Self {
        Self {
            text: String::new(),
            scale: PxScale::from(16.0),
            font_id: FontId::default(),
            color: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

impl<'a> From<&'a OwnedVariedSectionText> for VariedSectionText<'a> {
    fn from(owned: &'a OwnedVariedSectionText) -> Self {
        Self {
            text: owned.text.as_str(),
            scale: owned.scale,
            font_id: owned.font_id,
            color: owned.color,
        }
    }
}

impl From<&VariedSectionText<'_>> for OwnedVariedSectionText {
    fn from(s: &VariedSectionText<'_>) -> Self {
        Self {
            text: s.text.into(),
            scale: s.scale,
            font_id: s.font_id,
            color: s.color,
        }
    }
}
