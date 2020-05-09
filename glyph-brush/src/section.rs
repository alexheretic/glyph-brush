use super::{owned_section::*, *};
use ordered_float::OrderedFloat;
use std::{borrow::Cow, f32, hash::*};

pub type Color = [f32; 4];

/// An object that contains all the info to render a varied section of text. That is one including
/// many parts with differing fonts/scales/colors bowing to a single layout.
///
/// # Example
/// ```
/// use glyph_brush::{HorizontalAlign, Layout, Text, VariedSection};
///
/// let section = VariedSection::default()
///     .add_text(Text::new("The last word was ").with_color([0.0, 0.0, 0.0, 1.0]))
///     .add_text(Text::new("RED").with_color([1.0, 0.0, 0.0, 1.0]))
///     .with_layout(Layout::default().h_align(HorizontalAlign::Center));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct VariedSection<'a, X = Extra> {
    /// Position on screen to render text, in pixels from top-left. Defaults to (0, 0).
    pub screen_position: (f32, f32),
    /// Max (width, height) bounds, in pixels from top-left. Defaults to unbounded.
    pub bounds: (f32, f32),
    /// Built in layout, can be overridden with custom layout logic
    /// see [`queue_custom_layout`](struct.GlyphBrush.html#method.queue_custom_layout)
    pub layout: Layout<BuiltInLineBreaker>,
    /// Text to render, rendered next to one another according the layout.
    pub text: Vec<Text<'a, X>>,
}

impl<X: Clone> VariedSection<'_, X> {
    #[inline]
    pub(crate) fn clone_extras(&self) -> Vec<X> {
        self.text.iter().map(|t| &t.extra).cloned().collect()
    }
}

impl<X> Default for VariedSection<'static, X> {
    #[inline]
    fn default() -> Self {
        Self {
            screen_position: (0.0, 0.0),
            bounds: (f32::INFINITY, f32::INFINITY),
            layout: Layout::default(),
            text: vec![],
        }
    }
}

impl<'a, X> VariedSection<'a, X> {
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
    pub fn add_text<T: Into<Text<'a, X>>>(mut self, text: T) -> Self {
        self.text.push(text.into());
        self
    }

    #[inline]
    pub fn with_text<'b, X2>(self, text: Vec<Text<'b, X2>>) -> VariedSection<'b, X2> {
        VariedSection {
            text,
            screen_position: self.screen_position,
            bounds: self.bounds,
            layout: self.layout,
        }
    }
}

impl<'a, X: Clone> From<VariedSection<'a, X>> for Cow<'a, VariedSection<'a, X>> {
    fn from(owned: VariedSection<'a, X>) -> Self {
        Cow::Owned(owned)
    }
}

impl<'a, 'b, X: Clone> From<&'b VariedSection<'a, X>> for Cow<'b, VariedSection<'a, X>> {
    fn from(owned: &'b VariedSection<'a, X>) -> Self {
        Cow::Borrowed(owned)
    }
}

impl<X: Hash> Hash for VariedSection<'_, X> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let VariedSection {
            screen_position: (screen_x, screen_y),
            bounds: (bound_w, bound_h),
            layout,
            ref text,
        } = *self;

        let ord_floats: &[OrderedFloat<_>] = &[
            screen_x.into(),
            screen_y.into(),
            bound_w.into(),
            bound_h.into(),
        ];

        layout.hash(state);

        hash_section_text(state, text);

        ord_floats.hash(state);
    }
}

/// `SectionText` + extra.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Text<'a, X = Extra> {
    /// Text to render
    pub text: &'a str,
    /// Position on screen to render text, in pixels from top-left. Defaults to (0, 0).
    pub scale: PxScale,
    /// Font id to use for this section.
    ///
    /// It must be a valid id in the `FontMap` used for layout calls.
    /// The default `FontId(0)` should always be valid.
    pub font_id: FontId,
    /// Extra stuff for vertex generation.
    pub extra: X,
}

impl<X: Default> Default for Text<'static, X> {
    #[inline]
    fn default() -> Self {
        Self {
            text: "",
            scale: PxScale::from(16.0),
            font_id: <_>::default(),
            extra: <_>::default(),
        }
    }
}

impl<'a, X> Text<'a, X> {
    #[inline]
    pub fn with_text<'b>(self, text: &'b str) -> Text<'b, X> {
        Text {
            text,
            scale: self.scale,
            font_id: self.font_id,
            extra: self.extra,
        }
    }

    #[inline]
    pub fn with_scale<S: Into<PxScale>>(mut self, scale: S) -> Self {
        self.scale = scale.into();
        self
    }

    #[inline]
    pub fn with_font_id<F: Into<FontId>>(mut self, font_id: F) -> Self {
        self.font_id = font_id.into();
        self
    }

    #[inline]
    pub fn with_extra<X2>(self, extra: X2) -> Text<'a, X2> {
        Text {
            text: self.text,
            scale: self.scale,
            font_id: self.font_id,
            extra,
        }
    }
}

impl<'a> Text<'a, Extra> {
    #[inline]
    pub fn new(text: &'a str) -> Self {
        Text::default().with_text(text)
    }

    #[inline]
    pub fn with_color<C: Into<Color>>(mut self, color: C) -> Self {
        self.extra.color = color.into();
        self
    }

    #[inline]
    pub fn with_z<Z: Into<f32>>(mut self, z: Z) -> Self {
        self.extra.z = z.into();
        self
    }
}

impl<X> ToSectionText for Text<'_, X> {
    #[inline]
    fn to_section_text(&self) -> SectionText<'_> {
        SectionText {
            text: self.text,
            scale: self.scale,
            font_id: self.font_id,
        }
    }
}

#[inline]
fn hash_section_text<X: Hash, H: Hasher>(state: &mut H, text: &[Text<'_, X>]) {
    for t in text {
        let Text {
            text,
            scale,
            font_id,
            ref extra,
        } = *t;

        let ord_floats: [OrderedFloat<_>; 2] = [scale.x.into(), scale.y.into()];

        (text, font_id, extra, ord_floats).hash(state);
    }
}

impl<'text, X: Clone> VariedSection<'text, X> {
    pub fn to_owned(&self) -> OwnedVariedSection<X> {
        OwnedVariedSection {
            screen_position: self.screen_position,
            bounds: self.bounds,
            layout: self.layout,
            text: self.text.iter().map(OwnedText::from).collect(),
        }
    }

    pub(crate) fn to_hashable_parts(&self) -> HashableVariedSectionParts<'_, X> {
        let VariedSection {
            screen_position: (screen_x, screen_y),
            bounds: (bound_w, bound_h),
            ref text,
            layout: _,
        } = *self;

        let geometry = [
            screen_x.into(),
            screen_y.into(),
            bound_w.into(),
            bound_h.into(),
        ];

        HashableVariedSectionParts { geometry, text }
    }
}

impl<X> From<&VariedSection<'_, X>> for SectionGeometry {
    fn from(section: &VariedSection<'_, X>) -> Self {
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
/// let section = Section {
///     text: "Hello glyph_brush",
///     ..Section::default()
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Section<'a> {
    /// Text to render
    pub text: &'a str,
    /// Position on screen to render text, in pixels from top-left. Defaults to (0, 0).
    pub screen_position: (f32, f32),
    /// Max (width, height) bounds, in pixels from top-left. Defaults to unbounded.
    pub bounds: (f32, f32),
    /// Font scale. Defaults to 16
    pub scale: PxScale,
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
    #[inline]
    fn default() -> Self {
        Self {
            text: "",
            screen_position: (0.0, 0.0),
            bounds: (f32::INFINITY, f32::INFINITY),
            scale: PxScale::from(16.0),
            color: [0.0, 0.0, 0.0, 1.0],
            z: 0.0,
            layout: Layout::default(),
            font_id: FontId::default(),
        }
    }
}

impl<'a> From<&Section<'a>> for VariedSection<'a> {
    fn from(s: &Section<'a>) -> Self {
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
            text: vec![Text {
                text,
                scale,
                font_id,
                extra: Extra { color, z },
            }],
            screen_position,
            bounds,
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

impl<'a> From<&Section<'a>> for Cow<'a, VariedSection<'a>> {
    fn from(section: &Section<'a>) -> Self {
        Cow::Owned(VariedSection::from(section))
    }
}

pub(crate) struct HashableVariedSectionParts<'a, X> {
    geometry: [OrderedFloat<f32>; 4],
    text: &'a [Text<'a, X>],
}

impl<X: Hash> HashableVariedSectionParts<'_, X> {
    #[inline]
    pub fn hash_geometry<H: Hasher>(&self, state: &mut H) {
        self.geometry.hash(state);
    }

    #[inline]
    pub fn hash_text_no_extra<H: Hasher>(&self, state: &mut H) {
        for t in self.text {
            let Text {
                text,
                scale,
                font_id,
                ..
            } = *t;

            let ord_floats: &[OrderedFloat<_>] = &[scale.x.into(), scale.y.into()];

            (text, font_id, ord_floats).hash(state);
        }
    }

    #[inline]
    pub fn hash_extra<H: Hasher>(&self, state: &mut H) {
        self.text.iter().for_each(|t| t.extra.hash(state));
    }
}
