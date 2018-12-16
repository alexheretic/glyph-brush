use full_rusttype::Font;

/// Id for a font
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct FontId(pub usize);

/// Mapper of [`FontId`](struct.FontId.html) -> [`Font`](rusttype/struct.Font.html)
pub trait FontMap<'font> {
    fn font(&self, id: FontId) -> &Font<'font>;
}

impl<'font, T> FontMap<'font> for T
where
    T: AsRef<[Font<'font>]>,
{
    #[inline]
    fn font(&self, i: FontId) -> &Font<'font> {
        &self.as_ref()[i.0]
    }
}
