use super::*;
use std::f32;
use section::*;

/// An object that, along with the [`GlyphPositioner`](trait.GlyphPositioner.html),
/// contains all the info to render a section of text.
///
/// # Example
///
/// ```
/// use gfx_glyph::Section;
///
/// let section = Section {
///     text: "Hello gfx_glyph",
///     ..Section::default()
/// };
/// # let _ = section;
/// ```
#[derive(Debug, Clone, Copy)]
pub struct SimpleSection<'a> {
    /// Text to render
    pub text: &'a str,
    /// Position on screen to render text, in pixels from top-left. Defaults to (0, 0).
    pub screen_position: (f32, f32),
    /// Max (width, height) bounds, in pixels from top-left. Defaults to unbounded.
    pub bounds: (f32, f32),
    /// Font scale. Defaults to 16
    pub scale: Scale,
    /// Rgba color of rendered text. Defaults to black.
    pub color: [f32; 4],
    /// Z values for use in depth testing. Defaults to 0.0
    pub z: f32,
    /// Built in layout, can overridden with custom layout logic
    /// see [`queue_custom_layout`](struct.GlyphBrush.html#method.queue_custom_layout)
    pub layout: Layout<BuiltInLineBreaker>,
}

impl Default for SimpleSection<'static> {
    fn default() -> Self {
        Self {
            text: "",
            screen_position: (0.0, 0.0),
            bounds: (f32::INFINITY, f32::INFINITY),
            scale: Scale::uniform(16.0),
            color: [0.0, 0.0, 0.0, 1.0],
            z: 0.0,
            layout: Layout::default(),
        }
    }
}

impl<'a, 'b> From<&'b SimpleSection<'a>> for Section2<'a> {
    fn from(s: &'b SimpleSection<'a>) -> Self {
        let SimpleSection {
            text,
            scale,
            color,
            screen_position,
            bounds,
            z,
            layout,
        } = *s;

        Section2 {
            text: vec![SectionText {
                text,
                scale,
                color,
                ..SectionText::default()
            }],
            screen_position,
            bounds,
            z,
            layout,
        }
    }
}

impl<'a> From<SimpleSection<'a>> for Section2<'a> {
    fn from(s: SimpleSection<'a>) -> Self {
        Section2::from(&s)
    }
}
