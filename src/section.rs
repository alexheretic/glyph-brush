use super::*;
use std::f32;

#[derive(Debug, Clone)]
pub struct Section2<'a> {
    /// Position on screen to render text, in pixels from top-left. Defaults to (0, 0).
    pub screen_position: (f32, f32),
    /// Max (width, height) bounds, in pixels from top-left. Defaults to unbounded.
    pub bounds: (f32, f32),
    /// Z values for use in depth testing. Defaults to 0.0
    pub z: f32,
    /// Built in layout, can be overridden with custom layout logic
    /// see [`queue_custom_layout`](struct.GlyphBrush.html#method.queue_custom_layout)
    pub layout: Layout<BuiltInLineBreaker>,

    pub text: Vec<SectionText<'a>>,
}

impl Default for Section2<'static> {
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

impl<'a> Hash for Section2<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        use ordered_float::OrderedFloat;

        let Section2 {
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

#[derive(Debug, Clone, Copy)]
pub struct SectionText<'a> {
    /// Text to render
    pub text: &'a str,
    /// Position on screen to render text, in pixels from top-left. Defaults to (0, 0).
    pub scale: Scale,
    /// Rgba color of rendered text. Defaults to black.
    pub color: [f32; 4],

    pub font_id: usize,
}

impl Default for SectionText<'static> {
    fn default() -> Self {
        Self {
            text: "",
            scale: Scale::uniform(16.0),
            color: [0.0, 0.0, 0.0, 1.0],
            font_id: 0,
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
            font_id
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
