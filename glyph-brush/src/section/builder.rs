use crate::{Section, Text};
use glyph_brush_layout::{BuiltInLineBreaker, Layout};

/// [`Section`] builder.
///
/// Usage can avoid generic `X` type issues as it's not mentioned until text is involved.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SectionBuilder {
    /// Position on screen to render text, in pixels from top-left. Defaults to (0, 0).
    pub screen_position: (f32, f32),
    /// Max (width, height) bounds, in pixels from top-left. Defaults to unbounded.
    pub bounds: (f32, f32),
    /// Built in layout, can be overridden with custom layout logic
    /// see [`queue_custom_layout`](struct.GlyphBrush.html#method.queue_custom_layout)
    pub layout: Layout<BuiltInLineBreaker>,
}

impl Default for SectionBuilder {
    fn default() -> Self {
        Self {
            screen_position: (0.0, 0.0),
            bounds: (f32::INFINITY, f32::INFINITY),
            layout: Layout::default(),
        }
    }
}

impl SectionBuilder {
    #[inline]
    pub fn with_screen_position<P: Into<(f32, f32)>>(mut self, position: P) -> Self {
        self.screen_position = position.into();
        self
    }

    #[inline]
    pub fn with_bounds<P: Into<(f32, f32)>>(mut self, bounds: P) -> Self {
        self.bounds = bounds.into();
        self
    }

    #[inline]
    pub fn with_layout<L: Into<Layout<BuiltInLineBreaker>>>(mut self, layout: L) -> Self {
        self.layout = layout.into();
        self
    }

    #[inline]
    pub fn add_text<X>(self, text: Text<'_, X>) -> Section<'_, X> {
        self.with_text(vec![text])
    }

    #[inline]
    pub fn with_text<X>(self, text: Vec<Text<'_, X>>) -> Section<'_, X> {
        Section {
            text,
            screen_position: self.screen_position,
            bounds: self.bounds,
            layout: self.layout,
        }
    }
}
