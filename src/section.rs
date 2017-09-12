use super::*;
use std::f32;

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
pub struct Section<'a, L: LineBreaker> {
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
    pub layout: Layout<L>,
}

impl Default for Section<'static, StandardLineBreaker> {
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

impl<'a, L: LineBreaker> Hash for Section<'a, L> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        use ordered_float::OrderedFloat;

        let Section {
            text,
            screen_position: (screen_x, screen_y),
            bounds: (bound_w, bound_h),
            scale,
            color,
            z,
            layout: _layout_hashed_separately,
        } = *self;

        let ord_floats: &[OrderedFloat<_>] = &[
            screen_x.into(),
            screen_y.into(),
            bound_w.into(),
            bound_h.into(),
            scale.x.into(),
            scale.y.into(),
            color[0].into(),
            color[1].into(),
            color[2].into(),
            color[3].into(),
            z.into(),
        ];

        (text, ord_floats).hash(state);
    }
}
