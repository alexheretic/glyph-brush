use core::ops;

/// A rectangle, with top-left corner at min, and bottom-right corner at max.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Rectangle<N> {
    pub min: [N; 2],
    pub max: [N; 2],
}

impl<N: ops::Sub<Output = N> + Copy> Rectangle<N> {
    #[inline]
    pub fn width(&self) -> N {
        self.max[0] - self.min[0]
    }

    #[inline]
    pub fn height(&self) -> N {
        self.max[1] - self.min[1]
    }
}
