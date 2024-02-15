use crate::FontId;
use ab_glyph::*;
use std::f32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SectionGeometry {
    /// Position on screen to render text, in pixels from top-left. Defaults to (0, 0).
    pub screen_position: (f32, f32),
    /// Max (width, height) bounds, in pixels from top-left. Defaults to unbounded.
    pub bounds: (f32, f32),
}

impl Default for SectionGeometry {
    #[inline]
    fn default() -> Self {
        Self {
            screen_position: (0.0, 0.0),
            bounds: (f32::INFINITY, f32::INFINITY),
        }
    }
}

/// Text to layout together using a font & scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SectionText<'a> {
    /// Text to render
    pub text: &'a str,
    /// Pixel scale of text. Defaults to 16.
    pub scale: PxScale,
    /// Font id to use for this section.
    ///
    /// It must be a valid id in the `FontMap` used for layout calls.
    /// The default `FontId(0)` should always be valid.
    pub font_id: FontId,
}

impl Default for SectionText<'static> {
    #[inline]
    fn default() -> Self {
        Self {
            text: "",
            scale: PxScale::from(16.0),
            font_id: FontId::default(),
        }
    }
}

pub trait ToSectionText {
    fn to_section_text(&self) -> SectionText<'_>;
}

impl ToSectionText for SectionText<'_> {
    #[inline]
    fn to_section_text(&self) -> SectionText<'_> {
        *self
    }
}

impl ToSectionText for &SectionText<'_> {
    #[inline]
    fn to_section_text(&self) -> SectionText<'_> {
        **self
    }
}

/// A positioned glyph with info relating to the [`SectionText`] (or glyph_brush `Section::text`)
/// from which it was derived.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct SectionGlyph {
    /// The index of the [`SectionText`] source for this glyph.
    pub section_index: usize,
    /// The exact character byte index from the [`SectionText::text`] source for this glyph.
    pub byte_index: usize,
    /// A positioned glyph.
    pub glyph: Glyph,
    /// Font id.
    pub font_id: FontId,
}
