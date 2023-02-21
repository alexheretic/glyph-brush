use crate::Color;
use ordered_float::OrderedFloat;
use std::hash::{Hash, Hasher};

/// Default `extra` field type. Non-layout data for vertex generation.
#[derive(Debug, Clone, Copy)]
pub struct Extra {
    pub color: Color,
    pub outline_color: Color,
    pub z: f32,
}

impl Hash for Extra {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        [
            OrderedFloat::from(self.color[0]),
            OrderedFloat::from(self.color[1]),
            OrderedFloat::from(self.color[2]),
            OrderedFloat::from(self.color[3]),
            OrderedFloat::from(self.outline_color[0]),
            OrderedFloat::from(self.outline_color[1]),
            OrderedFloat::from(self.outline_color[2]),
            OrderedFloat::from(self.outline_color[3]),
            OrderedFloat::from(self.z),
        ]
        .hash(state)
    }
}

impl PartialEq for Extra {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.color == other.color && self.outline_color == other.outline_color && self.z == other.z
    }
}

impl Default for Extra {
    #[inline]
    fn default() -> Self {
        Self {
            color: [0.0, 0.0, 0.0, 1.0],
            outline_color: [0.0, 0.0, 0.0, 1.0],
            z: 0.0,
        }
    }
}
