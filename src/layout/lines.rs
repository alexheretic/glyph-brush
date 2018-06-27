use super::*;
use layout::words::RelativePositionedGlyph;
use layout::words::Words;
use layout::words::ZERO_V_METRICS;
use rusttype::vector;
use std::iter::Peekable;
use std::iter::{FusedIterator, Iterator};

/// A line of `Word`s limited to a max width bound.
pub(crate) struct Line<'font> {
    pub glyphs: Vec<(RelativePositionedGlyph<'font>, Color, FontId)>,
    pub max_v_metrics: VMetrics,
}

impl<'font> Line<'font> {
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
        match v_align {
            VerticalAlign::Top => {}
        };

        let screen_left = match h_align {
            HorizontalAlign::Left => point(screen_position.0, screen_position.1),
            // - Right alignment attained from left by shifting the line
            //   leftwards by the rightmost x distance from render position
            // - Central alignment is attained from left by shifting the line
            //   leftwards by half the rightmost x distance from render position
            _ => {
                let shift_left = {
                    let last_glyph = &self.glyphs.last().unwrap().0;
                    let rightmost_x = last_glyph
                        .bounds()
                        .map(|bounds| bounds.max.x.ceil())
                        .unwrap_or(last_glyph.relative.x)
                        + last_glyph.glyph.h_metrics().left_side_bearing;

                    if h_align == HorizontalAlign::Center {
                        rightmost_x / 2.0
                    }
                    else {
                        rightmost_x
                    }
                };
                point(screen_position.0 - shift_left, screen_position.1)
            }
        };

        self.glyphs
            .into_iter()
            .map(|(glyph, color, font_id)| (glyph.screen_positioned(screen_left), color, font_id))
            .collect()
    }
}

/// `Line` iterator.
pub(crate) struct Lines<'a, 'b, 'font: 'a + 'b, L: LineBreaker, H: 'b + BuildHasher> {
    pub(crate) words: Peekable<Words<'a, 'b, 'font, L, H>>,
    pub(crate) width_bound: f32,
}

impl<'a, 'b, 'font, L: LineBreaker, H: BuildHasher> Iterator for Lines<'a, 'b, 'font, L, H> {
    type Item = Line<'font>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut caret = vector(0.0, 0.0);
        let mut line = Line {
            glyphs: Vec::new(),
            max_v_metrics: ZERO_V_METRICS,
        };

        let mut progressed = false;

        #[allow(while_let_loop)] // peek/next borrow clash
        loop {
            if let Some(word) = self.words.peek() {
                let word_max_x = word.bounds.map(|b| b.max.x).unwrap_or(word.layout_width);
                if (caret.x + word_max_x).ceil() > self.width_bound {
                    break;
                }
            }
            else {
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

            if word.bounds.is_some() {
                line.glyphs
                    .extend(word.glyphs.into_iter().map(|(mut g, color, font_id)| {
                        g.relative = g.relative + caret;
                        (g, color, font_id)
                    }));
            }

            if word.hard_break {
                break;
            }

            caret.x += word.layout_width;
        }

        Some(line).filter(|_| progressed)
    }
}

impl<'a, 'b, 'font, L: LineBreaker, H: BuildHasher> FusedIterator for Lines<'a, 'b, 'font, L, H> {}
