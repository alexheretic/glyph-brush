use super::*;
use std::iter::{FusedIterator, Iterator};
use std::mem;

pub(crate) const ZERO_V_METRICS: VMetrics = VMetrics {
    ascent: 0.0,
    descent: 0.0,
    line_gap: 0.0,
};

struct Characters<'a, 'b, 'font: 'a + 'b, L: LineBreaker, H: 'b + BuildHasher> {
    font_map: &'b HashMap<FontId, Font<'font>, H>,
    section_info: Skip<Enumerate<Iter<'a, GlyphInfo<'a>>>>,
    line_breaker: L,

    part_info: Option<PartInfo<'a>>,
}

struct PartInfo<'a> {
    glyph_info: &'a GlyphInfo<'a>,
    info_chars: RemainingNormCharIndices<'a>,
    substring_len: usize,
    line_breaks: Box<Iterator<Item = LineBreak> + 'a>,
    next_break: Option<LineBreak>,
}

impl<'a, 'b, 'font, L: LineBreaker, H: BuildHasher> Characters<'a, 'b, 'font, L, H> {
    fn new(
        font_map: &'b HashMap<FontId, Font<'font>, H>,
        section_info: Skip<Enumerate<Iter<'a, GlyphInfo<'a>>>>,
        line_breaker: L,
    ) -> Self {
        Self {
            font_map,
            section_info,
            line_breaker,

            part_info: None,
        }
    }
}

struct Character<'font> {
    glyph: ScaledGlyph<'font>,
    color: Color,
    font_id: FontId,
    line_break: Option<LineBreak>,
    control: bool,
}

impl<'a, 'b, 'font, L: LineBreaker, H: BuildHasher> Iterator for Characters<'a, 'b, 'font, L, H> {
    type Item = Character<'font>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.part_info.is_none() {
            let (_, glyph_info) = self.section_info.next()?;
            let substring = glyph_info.substring();
            let line_breaks = self.line_breaker.line_breaks(substring);
            self.part_info = Some(PartInfo {
                glyph_info,
                info_chars: glyph_info.remaining_char_indices(),
                substring_len: substring.len(),
                line_breaks,
                next_break: None,
            });
        }

        {
            let PartInfo {
                glyph_info:
                    GlyphInfo {
                        scale,
                        color,
                        font_id,
                        ..
                    },
                info_chars,
                substring_len,
                line_breaks,
                next_break,
            } = self.part_info.as_mut().unwrap();

            if let Some((byte_index, c)) = info_chars.next() {
                if next_break.is_none() || next_break.unwrap().offset() <= byte_index {
                    loop {
                        let next = line_breaks.next();
                        if next.is_none() || next.unwrap().offset() > byte_index {
                            mem::replace(next_break, next);
                            break;
                        }
                    }
                }

                let glyph = self.font_map[font_id].glyph(c).scaled(*scale);

                let mut line_break = next_break.filter(|b| b.offset() == byte_index + 1);
                if line_break.is_some() && byte_index + 1 == *substring_len {
                    // handle inherent end-of-str hard breaks
                    line_break = line_break.and(c.eol_line_break(&self.line_breaker));
                }

                return Some(Character {
                    glyph,
                    color: *color,
                    font_id: *font_id,
                    line_break,
                    control: c.is_control(),
                });
            }
        }

        self.part_info = None;
        self.next()
    }
}

impl<'a, 'b, 'font, L: LineBreaker, H: BuildHasher> FusedIterator
    for Characters<'a, 'b, 'font, L, H>
{}

pub(crate) struct Words<'a, 'b, 'font: 'a + 'b, L: LineBreaker, H: 'b + BuildHasher> {
    characters: Characters<'a, 'b, 'font, L, H>,
}

impl<'a, 'b, 'font, L: LineBreaker, H: BuildHasher> Words<'a, 'b, 'font, L, H> {
    pub(crate) fn new(
        font_map: &'b HashMap<FontId, Font<'font>, H>,
        section_info: Skip<Enumerate<Iter<'a, GlyphInfo<'a>>>>,
        line_breaker: L,
    ) -> Self {
        Self {
            characters: Characters::new(font_map, section_info, line_breaker),
        }
    }
}

impl<'a, 'b, 'font, L: LineBreaker, H: BuildHasher> Iterator for Words<'a, 'b, 'font, L, H> {
    type Item = Word<'font>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut glyphs = Vec::new();
        let mut bounds: Option<Rect<f32>> = None;
        let mut caret = point(0.0, 0.0);
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
                    caret.x += font.pair_kerning(glyph.scale(), id, glyph.id());
                }
                last_glyph_id = Some(glyph.id());
            }

            let advance_width = glyph.h_metrics().advance_width;

            if !control {
                let positioned = RelativePositionedGlyph {
                    relative: caret,
                    glyph,
                };

                if let Some(glyph_bounds) = positioned.bounds() {
                    if let Some(mut word) = bounds.take() {
                        word.min.x = word.min.x.min(glyph_bounds.min.x);
                        word.min.y = word.min.y.min(glyph_bounds.min.y);
                        word.max.x = word.max.x.max(glyph_bounds.max.x);
                        word.max.y = word.max.y.max(glyph_bounds.max.y);
                        bounds = Some(word);
                    }
                    else {
                        bounds = Some(glyph_bounds);
                    }

                    glyphs.push((positioned, color, font_id));
                }
            }

            caret.x += advance_width;

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
                bounds,
                layout_width: caret.x,
                hard_break,
                max_v_metrics: max_v_metrics.unwrap_or(ZERO_V_METRICS),
            });
        }

        None
    }
}

impl<'a, 'b, 'font, L: LineBreaker, H: BuildHasher> FusedIterator for Words<'a, 'b, 'font, L, H> {}

pub(crate) struct RelativePositionedGlyph<'font> {
    pub(crate) relative: Point<f32>,
    pub(crate) glyph: ScaledGlyph<'font>,
}

impl<'font> RelativePositionedGlyph<'font> {
    pub(crate) fn bounds(&self) -> Option<Rect<f32>> {
        self.glyph.exact_bounding_box().map(|mut bb| {
            bb.min.x += self.relative.x;
            bb.min.y += self.relative.y;
            bb.max.x += self.relative.x;
            bb.max.y += self.relative.y;
            bb
        })
    }

    pub(crate) fn screen_positioned(self, mut pos: Point<f32>) -> PositionedGlyph<'font> {
        pos.x += self.relative.x;
        pos.y += self.relative.y;
        self.glyph.positioned(pos)
    }
}

pub(crate) struct Word<'font> {
    pub glyphs: Vec<(RelativePositionedGlyph<'font>, Color, FontId)>,
    pub bounds: Option<Rect<f32>>,
    /// pixel advance width of word includes ending spaces
    pub layout_width: f32,
    pub max_v_metrics: VMetrics,
    /// indicates the break after the word is a hard one
    pub hard_break: bool,
}
