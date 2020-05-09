use super::*;
use std::{borrow::Cow, f32};

#[derive(Debug, Clone, PartialEq)]
pub struct OwnedVariedSection<X = Extra> {
    /// Position on screen to render text, in pixels from top-left. Defaults to (0, 0).
    pub screen_position: (f32, f32),
    /// Max (width, height) bounds, in pixels from top-left. Defaults to unbounded.
    pub bounds: (f32, f32),
    /// Built in layout, can be overridden with custom layout logic
    /// see [`queue_custom_layout`](struct.GlyphBrush.html#method.queue_custom_layout)
    pub layout: Layout<BuiltInLineBreaker>,
    /// Text to render, rendered next to one another according the layout.
    pub text: Vec<OwnedText<X>>,
}

impl<X: Default> Default for OwnedVariedSection<X> {
    fn default() -> Self {
        Self {
            screen_position: (0.0, 0.0),
            bounds: (f32::INFINITY, f32::INFINITY),
            layout: Layout::default(),
            text: vec![],
        }
    }
}

impl<X: Clone> OwnedVariedSection<X> {
    pub fn to_borrowed(&self) -> VariedSection<'_, X> {
        VariedSection {
            screen_position: self.screen_position,
            bounds: self.bounds,
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
pub struct OwnedText<X = Extra> {
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
    // Extra stuff for vertex generation.
    pub extra: X,
}

// impl OwnedText {
//     pub fn from_text_and_color(st: &SectionText<'_>, color: Color) -> Self {
//         Self {
//             text: st.text.into(),
//             scale: st.scale,
//             font_id: st.font_id,
//             color,
//         }
//     }
// }

impl<X: Default> Default for OwnedText<X> {
    fn default() -> Self {
        Self {
            text: String::new(),
            scale: PxScale::from(16.0),
            font_id: <_>::default(),
            extra: <_>::default(),
        }
    }
}

impl<'a, X: Clone> From<&'a OwnedText<X>> for Text<'a, X> {
    fn from(owned: &'a OwnedText<X>) -> Self {
        Self {
            text: owned.text.as_str(),
            scale: owned.scale,
            font_id: owned.font_id,
            extra: owned.extra.clone(),
        }
    }
}

impl<X: Clone> From<&Text<'_, X>> for OwnedText<X> {
    fn from(s: &Text<'_, X>) -> Self {
        Self {
            text: s.text.into(),
            scale: s.scale,
            font_id: s.font_id,
            extra: s.extra.clone(),
        }
    }
}
