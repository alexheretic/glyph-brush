use super::owned_section::*;
use super::*;
use std::f32;

/// An object that contains all the info to render a varied section of text. That is one including
/// many parts with differing fonts/scales/colors bowing to a single layout.
///
/// For single font/scale/color sections it may be simpler to use
/// [`Section`](struct.Section.html).
///
/// # Example
///
/// ```
/// use gfx_glyph::{SectionText, VariedSection};
///
/// let section = VariedSection {
///     text: vec![
///         SectionText {
///             text: "I looked around and it was ",
///             ..SectionText::default()
///         },
///         SectionText {
///             text: "RED",
///             color: [1.0, 0.0, 0.0, 1.0],
///             ..SectionText::default()
///         },
///     ],
///     ..VariedSection::default()
/// };
/// # let _ = section;
/// ```
#[derive(Debug, Clone)]
pub struct VariedSection<'a> {
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
    pub text: Vec<SectionText<'a>>,
}

impl Default for VariedSection<'static> {
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

impl<'a> From<VariedSection<'a>> for Cow<'a, VariedSection<'a>> {
    fn from(owned: VariedSection<'a>) -> Self {
        Cow::Owned(owned)
    }
}

impl<'a, 'b> From<&'b VariedSection<'a>> for Cow<'b, VariedSection<'a>> {
    fn from(owned: &'b VariedSection<'a>) -> Self {
        Cow::Borrowed(owned)
    }
}

impl<'a> Hash for VariedSection<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        use ordered_float::OrderedFloat;

        let VariedSection {
            screen_position: (screen_x, screen_y),
            bounds: (bound_w, bound_h),
            z,
            layout,
            ref text,
        } = *self;

        let ord_floats: &[OrderedFloat<_>] = &[
            screen_x.into(),
            screen_y.into(),
            bound_w.into(),
            bound_h.into(),
            z.into(),
        ];

        (layout, text, ord_floats).hash(state);
    }
}

impl<'a> VariedSection<'a> {
    pub fn to_owned(&self) -> OwnedVariedSection {
        OwnedVariedSection {
            screen_position: self.screen_position,
            bounds: self.bounds,
            z: self.z,
            layout: self.layout,
            text: self.text.iter().map(|t| t.to_owned()).collect(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SectionText<'a> {
    /// Text to render
    pub text: &'a str,
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
}

impl Default for SectionText<'static> {
    fn default() -> Self {
        Self {
            text: "",
            scale: Scale::uniform(16.0),
            color: [0.0, 0.0, 0.0, 1.0],
            font_id: FontId::default(),
        }
    }
}

impl<'a> Hash for SectionText<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        use ordered_float::OrderedFloat;

        let SectionText {
            text,
            scale,
            color,
            font_id,
        } = *self;

        let ord_floats: &[OrderedFloat<_>] = &[
            scale.x.into(),
            scale.y.into(),
            color[0].into(),
            color[1].into(),
            color[2].into(),
            color[3].into(),
        ];

        (text, font_id, ord_floats).hash(state);
    }
}

impl<'a> SectionText<'a> {
    pub fn to_owned(&self) -> OwnedSectionText {
        OwnedSectionText {
            text: self.text.to_owned(),
            scale: self.scale,
            color: self.color,
            font_id: self.font_id,
        }
    }
}

/// Id for a font, the default `FontId(0)` will always be present in a `GlyphBrush`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct FontId(pub usize);

/// An object that contains all the info to render a section of text.
///
/// For varied font/scale/color sections see [`VariedSection`](struct.VariedSection.html).
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
pub struct Section<'a> {
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
    /// Font id to use for this section.
    ///
    /// It must be known to the `GlyphBrush` it is being used with,
    /// either `FontId::default()` or the return of
    /// [`add_font`](struct.GlyphBrushBuilder.html#method.add_font).
    pub font_id: FontId,
}

impl Default for Section<'static> {
    fn default() -> Self {
        Self {
            text: "",
            screen_position: (0.0, 0.0),
            bounds: (f32::INFINITY, f32::INFINITY),
            scale: Scale::uniform(16.0),
            color: [0.0, 0.0, 0.0, 1.0],
            z: 0.0,
            layout: Layout::default(),
            font_id: FontId::default(),
        }
    }
}

impl<'a, 'b> From<&'b Section<'a>> for VariedSection<'a> {
    fn from(s: &'b Section<'a>) -> Self {
        let Section {
            text,
            scale,
            color,
            screen_position,
            bounds,
            z,
            layout,
            font_id,
        } = *s;

        VariedSection {
            text: vec![SectionText {
                text,
                scale,
                color,
                font_id,
            }],
            screen_position,
            bounds,
            z,
            layout,
        }
    }
}

impl<'a> From<Section<'a>> for VariedSection<'a> {
    fn from(s: Section<'a>) -> Self {
        VariedSection::from(&s)
    }
}

impl<'a> From<Section<'a>> for Cow<'a, VariedSection<'a>> {
    fn from(section: Section<'a>) -> Self {
        Cow::Owned(VariedSection::from(section))
    }
}

impl<'a, 'b> From<&'b Section<'a>> for Cow<'a, VariedSection<'a>> {
    fn from(section: &'b Section<'a>) -> Self {
        Cow::Owned(VariedSection::from(section))
    }
}
