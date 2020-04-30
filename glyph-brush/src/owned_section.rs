use super::*;
use std::{borrow::Cow, f32};

#[derive(Debug, Clone, PartialEq)]
pub struct OwnedVariedSection<C> {
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
    pub text: Vec<OwnedSectionText<C>>,
}

impl<C> Default for OwnedVariedSection<C> {
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

impl<C: Clone> OwnedVariedSection<C> {
    pub fn to_borrowed(&self) -> VariedSection<'_, C> {
        VariedSection {
            screen_position: self.screen_position,
            bounds: self.bounds,
            z: self.z,
            layout: self.layout,
            text: self.text.iter().map(|t| t.into()).collect(),
        }
    }
}

impl<'a, C: Clone> From<&'a OwnedVariedSection<C>> for VariedSection<'a, C> {
    fn from(owned: &'a OwnedVariedSection<C>) -> Self {
        owned.to_borrowed()
    }
}

impl<'a, C: Clone> From<&'a OwnedVariedSection<C>> for Cow<'a, VariedSection<'a, C>> {
    fn from(owned: &'a OwnedVariedSection<C>) -> Self {
        Cow::Owned(owned.to_borrowed())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OwnedSectionText<C> {
    /// Text to render
    pub text: String,
    /// Position on screen to render text, in pixels from top-left. Defaults to (0, 0).
    pub scale: Scale,
    /// Rgba color of rendered text. Defaults to black.
    pub color: [f32; 4],
    /// Font id to use for this section.
    ///
    /// It must be known to the `GlyphBrush` it is being used with,
    /// either `FontId::default()` or the return of
    /// [`add_font`](struct.GlyphBrushBuilder.html#method.add_font).
    pub font_id: FontId,
    pub custom: C
}

impl<C: Default> Default for OwnedSectionText<C> {
    fn default() -> Self {
        Self {
            text: String::new(),
            scale: Scale::uniform(16.0),
            color: [0.0, 0.0, 0.0, 1.0],
            font_id: FontId::default(),
            custom: C::default()
        }
    }
}

impl<'a, C: Clone> From<&'a OwnedSectionText<C>> for SectionText<'a, C> {
    fn from(owned: &'a OwnedSectionText<C>) -> Self {
        Self {
            text: owned.text.as_str(),
            scale: owned.scale,
            color: owned.color,
            font_id: owned.font_id,
            custom: owned.custom.clone(),
        }
    }
}

impl<C: Clone> From<&SectionText<'_, C>> for OwnedSectionText<C> {
    fn from(st: &SectionText<'_, C>) -> Self {
        Self {
            text: st.text.into(),
            scale: st.scale,
            color: st.color,
            font_id: st.font_id,
            custom: st.custom.clone(),
        }
    }
}
