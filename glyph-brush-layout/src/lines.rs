use super::{Color, FontId, FontMap, HorizontalAlign, VerticalAlign};
use crate::{
    linebreak::LineBreaker,
    words::{RelativePositionedGlyph, Words, ZERO_V_METRICS},
};
use full_rusttype::{point, vector, PositionedGlyph, VMetrics};
use std::iter::{FusedIterator, Iterator, Peekable};

/// A line of `Word`s limited to a max width bound.
pub(crate) struct Line<'font> {
    pub glyphs: Vec<(RelativePositionedGlyph<'font>, Color, FontId)>,
    pub max_v_metrics: VMetrics,
}

impl<'font> Line<'font> {
    #[inline]
    pub(crate) fn line_height(&self) -> f32 {
        self.max_v_metrics.ascent - self.max_v_metrics.descent + self.max_v_metrics.line_gap
    }

    /// Returns line glyphs positioned on the screen and aligned.
    pub fn aligned_on_screen(
        self,
        screen_position: (f32, f32),
        h_align: HorizontalAlign,
        v_align: VerticalAlign,
    ) -> Vec<(PositionedGlyph<'font>, Color, FontId)> {
        if self.glyphs.is_empty() {
            return Vec::new();
        }

        // implement v-aligns when they're are supported
        let screen_left = match h_align {
            HorizontalAlign::Left => point(screen_position.0, screen_position.1),
            // - Right alignment attained from left by shifting the line
            //   leftwards by the rightmost x distance from render position
            // - Central alignment is attained from left by shifting the line
            //   leftwards by half the rightmost x distance from render position
            HorizontalAlign::Center | HorizontalAlign::Right => {
                let mut shift_left = {
                    let last_glyph = &self.glyphs.last().unwrap().0;
                    last_glyph
                        .bounds()
                        .map(|bounds| bounds.max.x.ceil())
                        .unwrap_or(last_glyph.relative.x)
                        + last_glyph.glyph.h_metrics().left_side_bearing
                };
                if h_align == HorizontalAlign::Center {
                    shift_left /= 2.0;
                }
                point(screen_position.0 - shift_left, screen_position.1)
            }
        };

        let screen_pos = match v_align {
            VerticalAlign::Top => screen_left,
            VerticalAlign::Center => {
                let mut screen_pos = screen_left;
                screen_pos.y -= self.line_height() / 2.0;
                screen_pos
            }
            VerticalAlign::Bottom => {
                let mut screen_pos = screen_left;
                screen_pos.y -= self.line_height();
                screen_pos
            }
        };

        self.glyphs
            .into_iter()
            .map(|(glyph, color, font_id)| (glyph.screen_positioned(screen_pos), color, font_id))
            .collect()
    }
}

/// `Line` iterator.
///
/// Will iterator through `Word` until the next word would break the `width_bound`.
///
/// Note: Will always have at least one word, if possible, even if the word itself
/// breaks the `width_bound`.
pub(crate) struct Lines<'a, 'b, 'font, L, F>
where
    'font: 'a + 'b,
    L: LineBreaker,
    F: FontMap<'font>,
{
    pub(crate) words: Peekable<Words<'a, 'b, 'font, L, F>>,
    pub(crate) width_bound: f32,
}

impl<'font, L: LineBreaker, F: FontMap<'font>> Iterator for Lines<'_, '_, 'font, L, F> {
    type Item = Line<'font>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut caret = vector(0.0, 0.0);
        let mut line = Line {
            glyphs: Vec::new(),
            max_v_metrics: ZERO_V_METRICS,
        };

        let mut progressed = false;

        while let Some(word) = self.words.peek() {
            let word_in_bounds = {
                let word_x = caret.x + word.layout_width_no_trail;
                // Reduce float errors by using relative "<= width bound" check
                word_x < self.width_bound || approx::relative_eq!(word_x, self.width_bound)
            };

            // only if `progressed` means the first word is allowed to overlap the bounds
            if !word_in_bounds && progressed {
                break;
            }

            let word = self.words.next().unwrap();
            progressed = true;

            if word.max_v_metrics.ascent > line.max_v_metrics.ascent {
                let diff_y = word.max_v_metrics.ascent - caret.y;
                caret.y += diff_y;

                // modify all smaller lined glyphs to occupy the new larger line
                for (glyph, ..) in &mut line.glyphs {
                    glyph.relative.y += diff_y;
                }

                line.max_v_metrics = word.max_v_metrics;
            }

            line.glyphs
                .extend(word.glyphs.into_iter().map(|(mut g, color, font_id)| {
                    g.relative = g.relative + caret;
                    (g, color, font_id)
                }));

            caret.x += word.layout_width;

            if word.hard_break {
                break;
            }
        }

        Some(line).filter(|_| progressed)
    }
}

impl<'font, L: LineBreaker, F: FontMap<'font>> FusedIterator for Lines<'_, '_, 'font, L, F> {}
