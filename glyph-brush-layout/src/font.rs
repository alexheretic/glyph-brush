use full_rusttype::Font;
use std::ops;

/// Id for a font
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct FontId(pub usize);

/// Mapper of [`FontId`](struct.FontId.html) -> [`Font`](rusttype/struct.Font.html)
pub trait FontMap<'font> {
    fn font(&self, FontId) -> &Font<'font>;
}

impl<'font, T> FontMap<'font> for T
where
    T: ops::Index<usize, Output = Font<'font>>,
{
    #[inline]
    fn font(&self, i: FontId) -> &Font<'font> {
        self.index(i.0)
    }
}
