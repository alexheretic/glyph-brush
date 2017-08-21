use super::*;
use std::f32;

#[derive(Debug, Clone)]
pub struct Section<'a> {
    /// text to render
    pub text: &'a str,
    /// position on screen to render text
    pub screen_position: (f32, f32),
    /// max (width, height) bounds
    pub bounds: (f32, f32),
    /// font scale
    pub scale: Scale,
    /// color of rendered text
    pub color: [f32; 4],
    /// Layout style of text within bounds
    pub layout: Layout,
}

impl<'a> Default for Section<'a> {
    fn default() -> Self {
        StaticSection::default().into()
    }
}

impl<'a> Hash for Section<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        use ordered_float::OrderedFloat;

        let Section {
            text,
            screen_position: (screen_x, screen_y),
            bounds: (bound_w, bound_h),
            scale,
            color,
            layout,
        } = *self;

        let ord_floats: [OrderedFloat<f32>; 10] = [
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
        ];

        (text, ord_floats, layout).hash(state);
    }
}

impl<'a> Section<'a> {
    pub fn to_owned_section(&self) -> OwnedSection {
        OwnedSection {
            text: self.text.to_owned(),
            screen_position: self.screen_position,
            bounds: self.bounds,
            scale: self.scale,
            color: self.color,
            layout: self.layout,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OwnedSection {
    pub text: String,
    pub screen_position: (f32, f32),
    pub bounds: (f32, f32),
    pub scale: Scale,
    pub color: [f32; 4],
    pub layout: Layout,
}

impl Default for OwnedSection {
    fn default() -> Self {
        Section::default().to_owned_section()
    }
}

impl<'a> From<&'a OwnedSection> for Section<'a> {
    fn from(section: &'a OwnedSection) -> Self {
        let &OwnedSection { ref text, screen_position, bounds, scale, color, layout } = section;
        Self {
            text: text,
            screen_position,
            bounds,
            scale,
            color,
            layout
        }
    }
}

#[derive(Debug, Clone)]
pub struct StaticSection {
    pub text: &'static str,
    pub screen_position: (f32, f32),
    pub bounds: (f32, f32),
    pub scale: Scale,
    pub color: [f32; 4],
    pub layout: Layout,
}

impl Default for StaticSection {
    fn default() -> Self {
        Self {
            text: "",
            screen_position: (0.0, 0.0),
            bounds: (f32::INFINITY, f32::INFINITY),
            scale: Scale::uniform(16.0),
            color: [0.0, 0.0, 0.0, 1.0],
            layout: Layout::default(),
        }
    }
}

impl<'a> From<&'a StaticSection> for Section<'static> {
    fn from(section: &'a StaticSection) -> Self {
        let &StaticSection { text, screen_position, bounds, scale, color, layout } = section;
        Self {
            text: text,
            screen_position,
            bounds,
            scale,
            color,
            layout,
        }
    }
}

impl From<StaticSection> for Section<'static> {
    fn from(section: StaticSection) -> Self {
        Section::from(&section)
    }
}
