use super::{Color, FontId, FontMap};
use crate::{
    characters::{Character, Characters},
    linebreak::{LineBreak, LineBreaker},
    lines::Lines,
};
use full_rusttype::{point, Point, PositionedGlyph, Rect, ScaledGlyph, VMetrics};
use std::iter::{FusedIterator, Iterator};

pub(crate) const ZERO_V_METRICS: VMetrics = VMetrics {
    ascent: 0.0,
    descent: 0.0,
    line_gap: 0.0,
};

/// Single 'word' ie a sequence of `Character`s where the last is a line-break.
///
/// Glyphs are relatively positioned from (0, 0) in a left-top alignment style.
pub(crate) struct Word<'font> {
    pub glyphs: Vec<(RelativePositionedGlyph<'font>, Color, FontId)>,
    /// pixel advance width of word includes ending spaces/invisibles
    pub layout_width: f32,
    /// pixel advance width of word not including any trailing spaces/invisibles
    pub layout_width_no_trail: f32,
    pub max_v_metrics: VMetrics,
    /// indicates the break after the word is a hard one
    pub hard_break: bool,
}

/// A scaled glyph that's relatively positioned.
pub(crate) struct RelativePositionedGlyph<'font> {
    pub relative: Point<f32>,
    pub glyph: ScaledGlyph<'font>,
}

impl<'font> RelativePositionedGlyph<'font> {
    #[inline]
    pub(crate) fn bounds(&self) -> Option<Rect<f32>> {
        self.glyph.exact_bounding_box().map(|mut bb| {
            bb.min.x += self.relative.x;
            bb.min.y += self.relative.y;
            bb.max.x += self.relative.x;
            bb.max.y += self.relative.y;
            bb
        })
    }

    #[inline]
    pub(crate) fn screen_positioned(self, mut pos: Point<f32>) -> PositionedGlyph<'font> {
        pos.x += self.relative.x;
        pos.y += self.relative.y;
        self.glyph.positioned(pos)
    }
}

/// `Word` iterator.
pub(crate) struct Words<'a, 'b, 'font: 'a + 'b, L, F>
where
    L: LineBreaker,
    F: FontMap<'font>,
{
    pub(crate) characters: Characters<'a, 'b, 'font, L, F>,
}

impl<'a, 'b, 'font, L, F> Words<'a, 'b, 'font, L, F>
where
    L: LineBreaker,
    F: FontMap<'font>,
{
    pub(crate) fn lines(self, width_bound: f32) -> Lines<'a, 'b, 'font, L, F> {
        Lines {
            words: self.peekable(),
            width_bound,
        }
    }
}

impl<'font, L, F> Iterator for Words<'_, '_, 'font, L, F>
where
    L: LineBreaker,
    F: FontMap<'font>,
{
    type Item = Word<'font>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let mut glyphs = Vec::new();
        let mut caret = 0.0;
        let mut caret_no_trail = caret;
        let mut last_glyph_id = None;
        let mut max_v_metrics = None;
        let mut hard_break = false;
        let mut progress = false;

        for Character {
            glyph,
            color,
            font_id,
            line_break,
            control,
        } in &mut self.characters
        {
            progress = true;
            {
                let font = glyph.font().expect("standalone not supported");
                let v_metrics = font.v_metrics(glyph.scale());
                if max_v_metrics.is_none() || v_metrics > max_v_metrics.unwrap() {
                    max_v_metrics = Some(v_metrics);
                }

                if let Some(id) = last_glyph_id.take() {
                    caret += font.pair_kerning(glyph.scale(), id, glyph.id());
                }
                last_glyph_id = Some(glyph.id());
            }

            let advance_width = glyph.h_metrics().advance_width;

            if !control {
                let positioned = RelativePositionedGlyph {
                    relative: point(caret, 0.0),
                    glyph,
                };

                caret += advance_width;

                if positioned.bounds().is_some() {
                    glyphs.push((positioned, color, font_id));

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
                max_v_metrics: max_v_metrics.unwrap_or(ZERO_V_METRICS),
            });
        }

        None
    }
}

impl<'font, L: LineBreaker, F: FontMap<'font>> FusedIterator for Words<'_, '_, 'font, L, F> {}
