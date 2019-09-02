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

#[cfg(test)]
mod test {
    use super::*;
    use once_cell::sync::Lazy;

    static DEJA_VU: Lazy<Font<'static>> = Lazy::new(|| {
        Font::from_bytes(include_bytes!("../../fonts/DejaVuSans.ttf") as &[u8]).unwrap()
    });
    static GARAMOND: Lazy<Font<'static>> = Lazy::new(|| {
        let font =
            Font::from_bytes(include_bytes!("../../fonts/GaramondNo8-Reg.ttf") as &[u8]).unwrap();
        assert_ne!(font.glyph_count(), DEJA_VU.glyph_count());
        font
    });

    macro_rules! assert_eq_font {
        ($fa:expr, $fb:expr) => {
            assert_eq!(
                $fa.glyph_count(),
                $fb.glyph_count(),
                "Unexpected glyph_count"
            );
        };
    }

    #[test]
    fn font_map_for_vecs() {
        let fonts: Vec<Font> = vec![DEJA_VU.clone(), GARAMOND.clone()];
        assert_eq_font!(fonts.font(FontId(0)), DEJA_VU);
        assert_eq_font!(fonts.font(FontId(1)), GARAMOND);
    }

    #[test]
    fn font_map_for_arrays() {
        let fonts: [Font; 2] = [DEJA_VU.clone(), GARAMOND.clone()];
        assert_eq_font!(fonts.font(FontId(0)), DEJA_VU);
        assert_eq_font!(fonts.font(FontId(1)), GARAMOND);
    }

    #[test]
    fn font_map_for_slices() {
        let fonts: &[Font] = &[DEJA_VU.clone(), GARAMOND.clone()][..];
        assert_eq_font!(fonts.font(FontId(0)), DEJA_VU);
        assert_eq_font!(fonts.font(FontId(1)), GARAMOND);
    }
}
