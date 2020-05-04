use crate::{
    characters::{Character, Characters},
    linebreak::{LineBreak, LineBreaker},
    lines::Lines,
    FontMap, SectionGlyph, SectionText,
};
use ab_glyph::*;
use std::iter::{FusedIterator, Iterator};

#[derive(Clone, Debug, Default)]
pub(crate) struct VMetrics {
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
}

impl VMetrics {
    #[inline]
    pub fn height(&self) -> f32 {
        self.ascent - self.descent
    }

    #[inline]
    pub fn max(self, other: Self) -> Self {
        if other.height() > self.height() {
            other
        } else {
            self
        }
    }
}

impl<F: Font> From<PxScaleFont<F>> for VMetrics {
    #[inline]
    fn from(scale_font: PxScaleFont<F>) -> Self {
        Self {
            ascent: scale_font.ascent(),
            descent: scale_font.descent(),
            line_gap: scale_font.line_gap(),
        }
    }
}

/// Single 'word' ie a sequence of `Character`s where the last is a line-break.
///
/// Glyphs are relatively positioned from (0, 0) in a left-top alignment style.
pub(crate) struct Word {
    pub glyphs: Vec<SectionGlyph>,
    /// pixel advance width of word includes ending spaces/invisibles
    pub layout_width: f32,
    /// pixel advance width of word not including any trailing spaces/invisibles
    pub layout_width_no_trail: f32,
    pub max_v_metrics: VMetrics,
    /// indicates the break after the word is a hard one
    pub hard_break: bool,
}

/// `Word` iterator.
pub(crate) struct Words<'a, 'b, L, F, FM, S>
where
    L: LineBreaker,
    F: Font,
    FM: FontMap<F>,
    S: Iterator<Item = SectionText<'a>>,
{
    pub(crate) characters: Characters<'a, 'b, L, F, FM, S>,
}

impl<'a, 'b, L, F, FM, S> Words<'a, 'b, L, F, FM, S>
where
    L: LineBreaker,
    F: Font,
    FM: FontMap<F>,
    S: Iterator<Item = SectionText<'a>>,
{
    pub(crate) fn lines(self, width_bound: f32) -> Lines<'a, 'b, L, F, FM, S> {
        Lines {
            words: self.peekable(),
            width_bound,
        }
    }
}

impl<'a, 'b, L, F: 'b, FM, S> Iterator for Words<'a, 'b, L, F, FM, S>
where
    L: LineBreaker,
    F: Font,
    FM: FontMap<F>,
    S: Iterator<Item = SectionText<'a>>,
{
    type Item = Word;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let mut glyphs = Vec::new();
        let mut caret = 0.0;
        let mut caret_no_trail = caret;
        let mut last_glyph_id = None;
        let mut max_v_metrics = VMetrics::default();
        let mut hard_break = false;
        let mut progress = false;

        for Character {
            mut glyph,
            scale_font,
            font_id,
            line_break,
            control,
            whitespace,
            section_index,
            byte_index,
        } in &mut self.characters
        {
            progress = true;
            {
                max_v_metrics = max_v_metrics.max(scale_font.into());

                if let Some(id) = last_glyph_id.take() {
                    caret += scale_font.kern(id, glyph.id);
                }
                last_glyph_id = Some(glyph.id);
            }

            let advance_width = scale_font.h_advance(glyph.id);

            if !control {
                glyph.position = point(caret, 0.0);
                glyphs.push(SectionGlyph {
                    glyph,
                    font_id,
                    section_index,
                    byte_index,
                });
                caret += advance_width;

                if !whitespace {
                    // not an invisible trail
                    caret_no_trail = caret;
                }
            }

            if line_break.is_some() {
                if let Some(LineBreak::Hard(..)) = line_break {
                    hard_break = true
                }
                break;
            }
        }

        if progress {
            return Some(Word {
                glyphs,
                layout_width: caret,
                layout_width_no_trail: caret_no_trail,
                hard_break,
                max_v_metrics,
            });
        }

        None
    }
}

impl<'a, L, F, FM, S> FusedIterator for Words<'a, '_, L, F, FM, S>
where
    L: LineBreaker,
    F: Font,
    FM: FontMap<F>,
    S: Iterator<Item = SectionText<'a>>,
{
}
