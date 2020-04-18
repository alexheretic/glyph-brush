use super::{owned_section::*, *};
use ordered_float::OrderedFloat;
use std::{borrow::Cow, f32, hash::*};

/// An object that contains all the info to render a varied section of text. That is one including
/// many parts with differing fonts/scales/colors bowing to a single layout.
///
/// For single font/scale/color sections it may be simpler to use
/// [`Section`](struct.Section.html).
///
/// # Example
///
/// ```
/// use glyph_brush::{SectionText, VariedSection};
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
///             custom: (),
///             ..SectionText::default()
///         },
///     ],
///     ..VariedSection::default()
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct VariedSection<'a, C> {
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
    pub text: Vec<SectionText<'a, C>>,
}

impl<C> Default for VariedSection<'static, C> {
    #[inline]
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

impl<'a, C: Clone> From<VariedSection<'a, C>> for Cow<'a, VariedSection<'a, C>> {
    fn from(owned: VariedSection<'a, C>) -> Self {
        Cow::Owned(owned)
    }
}

impl<'a, 'b, C: Clone> From<&'b VariedSection<'a, C>> for Cow<'b, VariedSection<'a, C>> {
    fn from(owned: &'b VariedSection<'a, C>) -> Self {
        Cow::Borrowed(owned)
    }
}

impl<C: Clone + Hash> Hash for VariedSection<'_, C> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let VariedSection {
            screen_position: (screen_x, screen_y),
            bounds: (bound_w, bound_h),
            z,
            layout,
            ref text,
            ..
        } = *self;

        let ord_floats: &[OrderedFloat<_>] = &[
            screen_x.into(),
            screen_y.into(),
            bound_w.into(),
            bound_h.into(),
            z.into(),
        ];

        layout.hash(state);

        hash_section_text(state, text);

        ord_floats.hash(state);
    }
}

#[inline]
fn hash_section_text<H: Hasher, C: Clone + Hash>(state: &mut H, text: &[SectionText<C>]) {
    for t in text {
        let SectionText {
            text,
            scale,
            color,
            font_id,
            ..
        } = *t;

        let custom = t.custom.clone();

        let ord_floats: &[OrderedFloat<_>] = &[
            scale.x.into(),
            scale.y.into(),
            color[0].into(),
            color[1].into(),
            color[2].into(),
            color[3].into(),
        ];

        (text, font_id, ord_floats, custom).hash(state);
    }
}

impl<'text, C: Clone> VariedSection<'text, C> {
    pub fn to_owned(&self) -> OwnedVariedSection<C> {
        OwnedVariedSection {
            screen_position: self.screen_position,
            bounds: self.bounds,
            z: self.z,
            layout: self.layout,
            text: self.text.iter().map(OwnedSectionText::from).collect(),
        }
    }

    pub(crate) fn to_hashable_parts(&self) -> HashableVariedSectionParts<'_, C> {
        let VariedSection {
            screen_position: (screen_x, screen_y),
            bounds: (bound_w, bound_h),
            z,
            ref text,
            ..
        } = *self;

        let geometry = [
            screen_x.into(),
            screen_y.into(),
            bound_w.into(),
            bound_h.into(),
        ];

        HashableVariedSectionParts {
            geometry,
            z: z.into(),
            text,
        }
    }
}

impl<C> From<&VariedSection<'_, C>> for SectionGeometry {
    fn from(section: &VariedSection<'_, C>) -> Self {
        Self {
            bounds: section.bounds,
            screen_position: section.screen_position,
        }
    }
}

/// An object that contains all the info to render a section of text.
///
/// For varied font/scale/color sections see [`VariedSection`](struct.VariedSection.html).
///
/// # Example
///
/// ```
/// use glyph_brush::Section;
///
/// let section: Section<()> = Section {
///     text: "Hello glyph_brush",
///     ..Section::default()
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Section<'a, C> {
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
    pub custom: C
}

impl<C: Default> Default for Section<'static, C> {
    #[inline]
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
            custom: C::default(),
        }
    }
}

impl<'a, C: Clone> From<&Section<'a, C>> for VariedSection<'a, C> {
    fn from(s: &Section<'a, C>) -> Self {
        let Section {
            text,
            scale,
            color,
            screen_position,
            bounds,
            z,
            layout,
            font_id,
            ..
        } = *s;

        VariedSection {
            text: vec![SectionText {
                text,
                scale,
                color,
                font_id,
                custom: s.custom.clone()
            }],
            screen_position,
            bounds,
            z,
            layout,
        }
    }
}

impl<'a, C: Clone> From<Section<'a, C>> for VariedSection<'a, C> {
    fn from(s: Section<'a, C>) -> Self {
        VariedSection::from(&s)
    }
}

impl<'a, C: Clone> From<Section<'a, C>> for Cow<'a, VariedSection<'a, C>> {
    fn from(section: Section<'a, C>) -> Self {
        Cow::Owned(VariedSection::from(section))
    }
}

impl<'a, C: Clone> From<&Section<'a, C>> for Cow<'a, VariedSection<'a, C>> {
    fn from(section: &Section<'a, C>) -> Self {
        Cow::Owned(VariedSection::from(section))
    }
}

pub(crate) struct HashableVariedSectionParts<'a, C> {
    geometry: [OrderedFloat<f32>; 4],
    z: OrderedFloat<f32>,
    text: &'a [SectionText<'a, C>],
}

impl<C: Clone + Hash> HashableVariedSectionParts<'_, C> {
    #[inline]
    pub fn hash_geometry<H: Hasher>(&self, state: &mut H) {
        self.geometry.hash(state);
    }

    #[inline]
    pub fn hash_z<H: Hasher>(&self, state: &mut H) {
        self.z.hash(state);
    }

    #[inline]
    pub fn hash_text_no_color<H: Hasher>(&self, state: &mut H) {
        for t in self.text {
            let SectionText {
                text,
                scale,
                font_id,
                ..
            } = *t;

            let custom = t.custom.clone();

            let ord_floats: &[OrderedFloat<_>] = &[scale.x.into(), scale.y.into()];

            (text, font_id, ord_floats, custom).hash(state);
        }
    }

    #[inline]
    pub fn hash_alpha<H: Hasher>(&self, state: &mut H) {
        for t in self.text {
            OrderedFloat(t.color[3]).hash(state);
        }
    }

    #[inline]
    pub fn hash_color<H: Hasher>(&self, state: &mut H) {
        for t in self.text {
            let color = t.color;

            let ord_floats: &[OrderedFloat<_>] =
                &[color[0].into(), color[1].into(), color[2].into()];

            ord_floats.hash(state);
        }
    }
}
