use ab_glyph::*;

/// Id for a font
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct FontId(pub usize);

/// Mapper of [`FontId`](struct.FontId.html) -> [`Font`](rusttype/struct.Font.html)
pub trait FontMap<F: Font> {
    fn font(&self, id: FontId) -> &F;
}

impl<T, F> FontMap<F> for T
where
    F: Font,
    T: AsRef<[F]>,
{
    #[inline]
    fn font(&self, i: FontId) -> &F {
        &self.as_ref()[i.0]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use once_cell::sync::Lazy;

    static DEJA_VU: Lazy<FontRef<'static>> = Lazy::new(|| {
        FontRef::try_from_slice(include_bytes!("../../fonts/DejaVuSans.ttf")).unwrap()
    });
    static GARAMOND: Lazy<FontRef<'static>> = Lazy::new(|| {
        let font =
            FontRef::try_from_slice(include_bytes!("../../fonts/GaramondNo8-Reg.ttf")).unwrap();
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
        let fonts: Vec<FontRef<'static>> = vec![DEJA_VU.clone(), GARAMOND.clone()];
        assert_eq_font!(fonts.font(FontId(0)), DEJA_VU);
        assert_eq_font!(fonts.font(FontId(1)), GARAMOND);
    }

    #[test]
    fn font_map_for_arrays() {
        let fonts: [FontRef<'static>; 2] = [DEJA_VU.clone(), GARAMOND.clone()];
        assert_eq_font!(fonts.font(FontId(0)), DEJA_VU);
        assert_eq_font!(fonts.font(FontId(1)), GARAMOND);
    }

    #[test]
    fn font_map_for_slices() {
        let fonts: &[FontRef<'static>] = &[DEJA_VU.clone(), GARAMOND.clone()][..];
        assert_eq_font!(fonts.font(FontId(0)), DEJA_VU);
        assert_eq_font!(fonts.font(FontId(1)), GARAMOND);
    }
}
