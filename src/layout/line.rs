#![allow(unused)]
use super::*;
use layout::words::Words;
use layout::words::ZERO_V_METRICS;
use std::iter::Peekable;
use std::iter::{FusedIterator, Iterator};
use std::mem;

/// Positions glyphs in a single line left to right with the screen position marking
/// the top-left corner.
/// Returns (positioned-glyphs, text that could not be positioned (outside bounds))
pub(super) fn single_line<'font, 'a, L: LineBreaker, H: BuildHasher>(
    words: &mut Peekable<Words<'a, '_, 'font, L, H>>,
    (screen_x, screen_y): (f32, f32),
    bound_w: f32,
    h_align: HorizontalAlign,
    v_align: VerticalAlign,
) -> Option<(Vec<(PositionedGlyph<'font>, Color, FontId)>, VMetrics)> {
    // implement v-aligns when they're are supported
    match v_align {
        VerticalAlign::Top => {}
    };

    let mut result: Vec<(PositionedGlyph, _, _)> = Vec::new();
    let mut caret = point(screen_x, screen_y);
    let mut max_v_ascent: VMetrics = ZERO_V_METRICS;

    let mut progressed = false;

    #[allow(while_let_loop)]
    loop {
        if let Some(word) = words.peek() {
            let word_max_x = word.bounds.map(|b| b.max.x).unwrap_or(word.layout_width);
            if (caret.x + word_max_x).ceil() > screen_x + bound_w {
                break;
            }
        }
        else {
            break;
        }

        let mut word = words.next().unwrap();
        progressed = true;

        if word.max_v_metrics.ascent > max_v_ascent.ascent {
            let diff_y = screen_y + word.max_v_metrics.ascent - caret.y;
            caret.y += diff_y;

            // modify all smaller lined glyphs to occupy the new larger line
            result = shift_glyphs(result, |pos| pos.y += diff_y);

            max_v_ascent = word.max_v_metrics;
        }

        if word.bounds.is_some() {
            result.extend(
                word.glyphs
                    .into_iter()
                    .map(|(g, color, font_id)| (g.screen_positioned(caret), color, font_id)),
            );
        }

        if word.hard_break {
            break;
        }

        caret.x += word.layout_width;
    }

    apply_h_alignment(&mut result, h_align, screen_x);

    Some((result, max_v_ascent)).filter(|_| progressed)
}

fn apply_h_alignment(
    line: &mut Vec<(PositionedGlyph, Color, FontId)>,
    h_align: HorizontalAlign,
    screen_x: f32,
) {
    if line.is_empty() {
        return;
    }
    match h_align {
        HorizontalAlign::Left => (), // all done
        HorizontalAlign::Right | HorizontalAlign::Center => {
            // Right alignment attained from left by shifting the line
            // leftwards by the rightmost x distance from render position
            // Central alignment is attained from left by shifting the line
            // leftwards by half the rightmost x distance from render position
            let rightmost_x_offset = {
                let last_glyph = &line.last().unwrap().0;
                last_glyph
                    .pixel_bounding_box()
                    .map(|bb| bb.max.x as f32)
                    .unwrap_or_else(|| last_glyph.position().x)
                    + last_glyph.unpositioned().h_metrics().left_side_bearing
                    - screen_x
            };
            let shift_left = {
                if h_align == HorizontalAlign::Right {
                    rightmost_x_offset
                }
                else {
                    rightmost_x_offset / 2.0
                }
            };

            let glyphs = mem::replace(line, Vec::new());
            mem::replace(line, shift_glyphs(glyphs, |pos| pos.x -= shift_left));
        }
    }
}

#[inline]
fn shift_glyphs<'font, F: Fn(&mut Point<f32>)>(
    glyphs: Vec<(PositionedGlyph<'font>, Color, FontId)>,
    shifter: F,
) -> Vec<(PositionedGlyph<'font>, Color, FontId)> {
    glyphs
        .into_iter()
        .map(|(glyph, color, font)| {
            let mut pos = glyph.position();
            shifter(&mut pos);
            (glyph.into_unpositioned().positioned(pos), color, font)
        })
        .collect()
}

pub(super) fn page<'font, 'a, L: LineBreaker, H: BuildHasher>(
    words: &mut Peekable<Words<'a, '_, 'font, L, H>>,
    (screen_x, screen_y): (f32, f32),
    (bound_w, bound_h): (f32, f32),
    h_align: HorizontalAlign,
    v_align: VerticalAlign,
) -> Vec<(PositionedGlyph<'font>, Color, FontId)> {
    let mut out = vec![];
    let mut caret_y = screen_y;

    while caret_y < screen_y + bound_h {
        if let Some((glyphs, max_v_metrics)) =
            single_line(words, (screen_x, caret_y), bound_w, h_align, v_align)
        {
            out.extend(glyphs);
            caret_y += max_v_metrics.ascent - max_v_metrics.descent + max_v_metrics.line_gap;
        }
        else {
            break;
        }
    }

    out
}
