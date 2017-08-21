use super::*;
use std::iter::Skip;
use std::str::Chars;
use std::fmt;
use unicode_normalization::*;

#[derive(Debug, Clone)]
pub struct GlyphInfo<'a> {
    pub text: &'a str,
    pub skip: usize,
    pub screen_position: (f32, f32),
    pub bounds: (f32, f32),
    pub scale: Scale,
}

impl<'a> GlyphInfo<'a> {
    fn nfc_chars(&self) -> Skip<Recompositions<Chars<'a>>> {
        self.text.nfc().skip(self.skip)
    }

    fn skip(&self, skip: usize) -> GlyphInfo<'a> {
        let mut clone = self.clone();
        clone.skip += skip;
        clone
    }
}

impl<'a, 'b> From<&'b Section<'a>> for GlyphInfo<'a> {
    fn from(section: &'b Section<'a>) -> Self {
        let Section { text, screen_position, bounds, scale, .. } = *section;
        Self {
            text,
            skip: 0,
            screen_position,
            bounds,
            scale,
        }
    }
}

pub trait GlyphPositioner: Hash {
    /// Calculate a sequence of positioned glyphs to render
    fn calculate_glyphs<'a, G>(&self, font: &Font, section: G)
        -> Vec<PositionedGlyph>
        where G: Into<GlyphInfo<'a>>;
    /// Return a rectangle according to the requested render position and bounds appropriate
    /// for the glyph layout
    fn bounds_rect<'a, G>(&self, section: G) -> Rect<f32> where G: Into<GlyphInfo<'a>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layout {
    /// Renders a single line according to the inner alignment
    /// new lines will end the line,
    /// partially hitting the width bound will end the line
    SingleLine(HorizontalAlign),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HorizontalAlign {
    /// Leftmost character is immediately to the right of the render position
    Left,
    /// Leftmost & rightmost characters are equidistant to the render position
    Center,
    /// Rightmost character is immetiately to the left of the redner position
    Right,
}

impl Default for Layout {
    fn default() -> Self { Layout::SingleLine(HorizontalAlign::Left) }
}

impl GlyphPositioner for Layout {
    fn calculate_glyphs<'a, G: Into<GlyphInfo<'a>>>(&self, font: &Font, section: G)
        -> Vec<PositionedGlyph>
    {
        self.calculate_glyphs_and_leftover(font, &section.into()).0
    }

    fn bounds_rect<'a, G: Into<GlyphInfo<'a>>>(&self, section: G) -> Rect<f32> {
        let GlyphInfo {
            screen_position: (screen_x, screen_y),
            bounds: (bound_w, bound_h),
            .. } = section.into();
        match *self {
            Layout::SingleLine(HorizontalAlign::Left) => Rect {
                min: Point { x: screen_x, y: screen_y },
                max: Point { x: screen_x + bound_w, y: screen_y + bound_h },
            },
            Layout::SingleLine(HorizontalAlign::Center) => Rect {
                min: Point { x: screen_x - bound_w / 2.0, y: screen_y },
                max: Point { x: screen_x + bound_w / 2.0, y: screen_y + bound_h },
            },
            Layout::SingleLine(HorizontalAlign::Right) => Rect {
                min: Point { x: screen_x - bound_w, y: screen_y },
                max: Point { x: screen_x, y: screen_y + bound_h },
            },
        }
    }
}

pub enum LayoutLeftover<'a> {
    /// leftover text after a new line character
    AfterNewline(Point<f32>, GlyphInfo<'a>),
    /// text that would fall outside of the horizontal bound
    OutOfWidthBound(Point<f32>, GlyphInfo<'a>),
    /// text that would fall fully outside the vertical bound
    /// note: does not include hidden text within a layout
    /// for example a `_` character hidden but between visible characters would be ignored
    OutOfHeightBound(Point<f32>, GlyphInfo<'a>),
}

impl<'a> fmt::Debug for LayoutLeftover<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            LayoutLeftover::AfterNewline(..) => "AfterNewline",
            LayoutLeftover::OutOfWidthBound(..) => "OutOfWidthBound",
            LayoutLeftover::OutOfHeightBound(..) => "OutOfHeightBound",
        })
    }
}

impl Layout {
    pub fn calculate_glyphs_and_leftover<'a>(&self, font: &Font, section: &GlyphInfo<'a>)
        -> (Vec<PositionedGlyph>, Option<LayoutLeftover<'a>>)
    {
        match *self {
            Layout::SingleLine(h_align) => single_line(font, h_align, section),
        }
    }
}

/// Positions glyphs in a single line left to right with the screen position marking
/// the top-left corner.
/// Returns (positioned-glyphs, text that could not be positioned (outside bounds))
fn single_line<'a>(
    font: &Font,
    h_align: HorizontalAlign,
    glyph_info: &GlyphInfo<'a>)
    -> (Vec<PositionedGlyph>, Option<LayoutLeftover<'a>>)
{
    let &GlyphInfo {
        screen_position: (screen_x, screen_y),
        bounds: (bound_w, bound_h),
        scale,
        .. } = glyph_info;

    let mut result = Vec::new();
    let mut leftover = None;
    let v_metrics = font.v_metrics(scale);
    let mut caret = point(screen_x, screen_y + v_metrics.ascent);
    let mut last_glyph_id = None;
    let mut vertically_hidden_tail_start = None;
    for (index, c) in glyph_info.nfc_chars().enumerate() {
        if c.is_control() {
            if c == '\n' {
                leftover = Some(LayoutLeftover::AfterNewline(caret, glyph_info.skip(index+1)));
                break;
            }
            continue;
        }
        let base_glyph = if let Some(glyph) = font.glyph(c) {
            glyph
        }
        else { continue };
        if let Some(id) = last_glyph_id.take() {
            caret.x += font.pair_kerning(scale, id, base_glyph.id());
        }
        last_glyph_id = Some(base_glyph.id());
        let glyph = base_glyph.scaled(scale).positioned(caret);
        if let Some(bb) = glyph.pixel_bounding_box() {
            if bb.max.x as f32 > (screen_x + bound_w) {
                leftover = Some(LayoutLeftover::OutOfWidthBound(caret, glyph_info.skip(index)));
                break;
            }
            if bb.min.y as f32 > (screen_y + bound_h) {
                if vertically_hidden_tail_start.is_none() {
                    vertically_hidden_tail_start = Some(index);
                }
                caret.x += glyph.unpositioned().h_metrics().advance_width;
                continue;
            }
            vertically_hidden_tail_start = None;
        }
        caret.x += glyph.unpositioned().h_metrics().advance_width;
        result.push(glyph.standalone());
    }
    if let Some(idx) = vertically_hidden_tail_start {
        // if entire tail was vertically hidden then return as unrendered text
        // otherwise ignore
        leftover = Some(LayoutLeftover::OutOfHeightBound(caret, glyph_info.skip(idx)));
    }

    if !result.is_empty() {
        match h_align {
            HorizontalAlign::Left => (), // all done
            HorizontalAlign::Right | HorizontalAlign::Center => {
                // Right alignment attained from left by shifting the line
                // leftwards by the rightmost x distance from render position
                // Central alignment is attained from left by shifting the line
                // leftwards by half the rightmost x distance from render position
                let rightmost_x_offset = {
                    let last = &result[result.len()-1];
                    last.position().x + last.unpositioned().h_metrics().advance_width - screen_x
                };
                let shift_left = {
                    if h_align == HorizontalAlign::Right { rightmost_x_offset }
                    else { rightmost_x_offset / 2.0 }
                };
                let mut shifted = Vec::with_capacity(result.len());
                for glyph in result.drain(..) {
                    let Point { x, y } = glyph.position();
                    let x = x - shift_left;
                    shifted.push(glyph.into_unpositioned().positioned(Point { x, y }));
                }
                result = shifted;
            },
        }
    }

    (result, leftover)
}

// fn left_aligned<'a>(font: &'a Font,
//                     scale: Scale,
//                     screen_position: (f32, f32),
//                     width: u32,
//                     text: &str) -> Vec<PositionedGlyph<'static>> {
//     use unicode_normalization::UnicodeNormalization;
//     let mut result = Vec::new();
//     let v_metrics = font.v_metrics(scale);
//     let advance_height = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;
//     let mut caret = point(screen_position.0, screen_position.1 + v_metrics.ascent);
//     let mut last_glyph_id = None;
//     for c in text.nfc() {
//         if c.is_control() {
//             if c == '\n' {
//                 caret = point(0.0, caret.y + advance_height);
//             }
//             continue;
//         }
//         let base_glyph = if let Some(glyph) = font.glyph(c) {
//             glyph
//         }
//         else { continue };
//         if let Some(id) = last_glyph_id.take() {
//             caret.x += font.pair_kerning(scale, id, base_glyph.id());
//         }
//         last_glyph_id = Some(base_glyph.id());
//         let mut glyph = base_glyph.scaled(scale).positioned(caret);
//         if let Some(bb) = glyph.pixel_bounding_box() {
//             if bb.max.x > width as i32 {
//                 caret = point(0.0, caret.y + advance_height);
//                 glyph = glyph.into_unpositioned().positioned(caret);
//                 last_glyph_id = None;
//             }
//         }
//         caret.x += glyph.unpositioned().h_metrics().advance_width;
//         result.push(glyph.standalone());
//     }
//     result
// }

#[cfg(test)]
mod layout_test {
    use super::*;
    use std::f32;

    const A_FONT: &[u8] = include_bytes!("../test/DejaVuSansMono.ttf") as &[u8];

    #[test]
    fn single_line_left_unbounded() {
        let _ = ::pretty_env_logger::init();

        let font = FontCollection::from_bytes(A_FONT)
            .into_font()
            .expect("Could not create rusttype::Font");

        let (glyphs, leftover) = Layout::SingleLine(HorizontalAlign::Left)
            .calculate_glyphs_and_leftover(
                &font,
                &GlyphInfo::from(&Section {
                    text: "hello world",
                    scale: Scale::uniform(20.0),
                    ..Section::default()
                })
            );

        assert!(leftover.is_none());
        assert_eq!(glyphs.len(), 11);
        assert_eq!(glyphs[0].position().x, 0.0);
        assert!(glyphs[10].position().x > 0.0,
            "unexpected last position {:?}", glyphs[10].position());

        assert_eq!(glyphs[0].id(), font.glyph('h').unwrap().id());
        assert_eq!(glyphs[1].id(), font.glyph('e').unwrap().id());
        assert_eq!(glyphs[2].id(), font.glyph('l').unwrap().id());
        assert_eq!(glyphs[3].id(), font.glyph('l').unwrap().id());
        assert_eq!(glyphs[4].id(), font.glyph('o').unwrap().id());
        assert_eq!(glyphs[5].id(), font.glyph(' ').unwrap().id());
        assert_eq!(glyphs[6].id(), font.glyph('w').unwrap().id());
        assert_eq!(glyphs[7].id(), font.glyph('o').unwrap().id());
        assert_eq!(glyphs[8].id(), font.glyph('r').unwrap().id());
        assert_eq!(glyphs[9].id(), font.glyph('l').unwrap().id());
        assert_eq!(glyphs[10].id(), font.glyph('d').unwrap().id());
    }

    #[test]
    fn single_line_right_unbounded() {
        let _ = ::pretty_env_logger::init();

        let font = FontCollection::from_bytes(A_FONT)
            .into_font()
            .expect("Could not create rusttype::Font");

        let (glyphs, leftover) = Layout::SingleLine(HorizontalAlign::Right)
            .calculate_glyphs_and_leftover(
                &font,
                &GlyphInfo::from(&Section {
                    text: "hello world",
                    scale: Scale::uniform(20.0),
                    ..Section::default()
                })
            );

        assert!(leftover.is_none());
        assert_eq!(glyphs.len(), 11);
        assert!(glyphs[0].position().x < glyphs[10].position().x);
        assert!(glyphs[10].position().x <= 0.0,
            "unexpected last position {:?}", glyphs[10].position());

        let rightmost_x = glyphs[10].position().x + glyphs[10].unpositioned().h_metrics().advance_width;
        assert_relative_eq!(rightmost_x, 0.0, epsilon = 1e-5);

        assert_eq!(glyphs[0].id(), font.glyph('h').unwrap().id());
        assert_eq!(glyphs[1].id(), font.glyph('e').unwrap().id());
        assert_eq!(glyphs[2].id(), font.glyph('l').unwrap().id());
        assert_eq!(glyphs[3].id(), font.glyph('l').unwrap().id());
        assert_eq!(glyphs[4].id(), font.glyph('o').unwrap().id());
        assert_eq!(glyphs[5].id(), font.glyph(' ').unwrap().id());
        assert_eq!(glyphs[6].id(), font.glyph('w').unwrap().id());
        assert_eq!(glyphs[7].id(), font.glyph('o').unwrap().id());
        assert_eq!(glyphs[8].id(), font.glyph('r').unwrap().id());
        assert_eq!(glyphs[9].id(), font.glyph('l').unwrap().id());
        assert_eq!(glyphs[10].id(), font.glyph('d').unwrap().id());
    }

    #[test]
    fn single_line_center_unbounded() {
        let _ = ::pretty_env_logger::init();

        let font = FontCollection::from_bytes(A_FONT)
            .into_font()
            .expect("Could not create rusttype::Font");

        let (glyphs, leftover) = Layout::SingleLine(HorizontalAlign::Center)
            .calculate_glyphs_and_leftover(
                &font,
                &GlyphInfo::from(&Section {
                    text: "hello world",
                    scale: Scale::uniform(20.0),
                    ..Section::default()
                })
            );

        assert!(leftover.is_none());
        assert_eq!(glyphs.len(), 11);
        assert!(glyphs[0].position().x < 0.0,
            "unexpected first glyph position {:?}", glyphs[0].position());
        assert!(glyphs[10].position().x > 0.0,
            "unexpected last glyph position {:?}", glyphs[10].position());

        let leftmost_x = glyphs[0].position().x;
        let rightmost_x = glyphs[10].position().x + glyphs[10].unpositioned().h_metrics().advance_width;
        assert_relative_eq!(rightmost_x, -leftmost_x);

        assert_eq!(glyphs[0].id(), font.glyph('h').unwrap().id());
        assert_eq!(glyphs[1].id(), font.glyph('e').unwrap().id());
        assert_eq!(glyphs[2].id(), font.glyph('l').unwrap().id());
        assert_eq!(glyphs[3].id(), font.glyph('l').unwrap().id());
        assert_eq!(glyphs[4].id(), font.glyph('o').unwrap().id());
        assert_eq!(glyphs[5].id(), font.glyph(' ').unwrap().id());
        assert_eq!(glyphs[6].id(), font.glyph('w').unwrap().id());
        assert_eq!(glyphs[7].id(), font.glyph('o').unwrap().id());
        assert_eq!(glyphs[8].id(), font.glyph('r').unwrap().id());
        assert_eq!(glyphs[9].id(), font.glyph('l').unwrap().id());
        assert_eq!(glyphs[10].id(), font.glyph('d').unwrap().id());
    }

    #[test]
    fn single_line_left_finish_at_newline() {
        let _ = ::pretty_env_logger::init();

        let font = FontCollection::from_bytes(A_FONT)
            .into_font()
            .expect("Could not create rusttype::Font");

        let (glyphs, leftover) = Layout::SingleLine(HorizontalAlign::Left)
            .calculate_glyphs_and_leftover(
                &font,
                &GlyphInfo::from(&Section {
                    text: "hello\nworld",
                    scale: Scale::uniform(20.0),
                    ..Section::default()
                })
            );

        if let Some(LayoutLeftover::AfterNewline(_, leftover)) = leftover {
            assert_eq!(leftover.nfc_chars().collect::<String>(), "world");
        }
        else {
            assert!(false, "Unexpected leftover {:?}", leftover);
        }
        assert_eq!(glyphs.len(), 5);
        assert_eq!(glyphs[0].position().x, 0.0);
        assert!(glyphs[4].position().x > 0.0,
            "unexpected last position {:?}", glyphs[4].position());

        assert_eq!(glyphs[0].id(), font.glyph('h').unwrap().id());
        assert_eq!(glyphs[1].id(), font.glyph('e').unwrap().id());
        assert_eq!(glyphs[2].id(), font.glyph('l').unwrap().id());
        assert_eq!(glyphs[3].id(), font.glyph('l').unwrap().id());
        assert_eq!(glyphs[4].id(), font.glyph('o').unwrap().id());
    }

    #[test]
    fn single_line_little_verticle_room() {
        let _ = ::pretty_env_logger::init();

        let font = FontCollection::from_bytes(A_FONT)
            .into_font()
            .expect("Could not create rusttype::Font");

        let (glyphs, leftover) = Layout::SingleLine(HorizontalAlign::Left)
            .calculate_glyphs_and_leftover(
                &font,
                &GlyphInfo::from(&Section {
                    text: "hello world",
                    scale: Scale::uniform(20.0),
                    bounds: (f32::INFINITY, 5.0),
                    ..Section::default()
                })
            );

        assert!(leftover.is_none(), "unexpected leftover {:?}", leftover);
        assert_eq!(glyphs.len(), 6);

        assert_eq!(glyphs[0].id(), font.glyph('h').unwrap().id());
        // 'e' hidden
        assert_eq!(glyphs[1].id(), font.glyph('l').unwrap().id());
        assert_eq!(glyphs[2].id(), font.glyph('l').unwrap().id());
        // 'o' hidden
        assert_eq!(glyphs[3].id(), font.glyph(' ').unwrap().id());
        // 'w' hidden
        // 'o' hidden
        // 'r' hidden
        assert_eq!(glyphs[4].id(), font.glyph('l').unwrap().id());
        assert_eq!(glyphs[5].id(), font.glyph('d').unwrap().id());

        // letter `l` should be in the same place as when all the word is visible
        let (all_glyphs, _) = Layout::SingleLine(HorizontalAlign::Left)
            .calculate_glyphs_and_leftover(
                &font,
                &GlyphInfo::from(&Section {
                    text: "hello world",
                    scale: Scale::uniform(20.0),
                    ..Section::default()
                })
            );
        assert_eq!(all_glyphs[9].id(), font.glyph('l').unwrap().id());
        assert_relative_eq!(all_glyphs[9].position().x, glyphs[4].position().x);
        assert_relative_eq!(all_glyphs[9].position().y, glyphs[4].position().y);
    }

    #[test]
    fn single_line_little_verticle_room_tail_lost() {
        let _ = ::pretty_env_logger::init();

        let font = FontCollection::from_bytes(A_FONT)
            .into_font()
            .expect("Could not create rusttype::Font");

        let (glyphs, leftover) = Layout::SingleLine(HorizontalAlign::Left)
            .calculate_glyphs_and_leftover(
                &font,
                &GlyphInfo::from(&Section {
                    text: "hellowor__",
                    scale: Scale::uniform(20.0),
                    // vertical bound of 5px means only tall letters will be seen
                    bounds: (f32::INFINITY, 5.0),
                    ..Section::default()
                })
            );

        if let Some(LayoutLeftover::OutOfHeightBound(_, leftover)) = leftover {
            assert_eq!(leftover.nfc_chars().collect::<String>(), "owor__");
        }
        else {
            assert!(false, "Unexpected leftover {:?}", leftover);
        }
        assert_eq!(glyphs.len(), 3);

        assert_eq!(glyphs[0].id(), font.glyph('h').unwrap().id());
        // 'e' hidden
        assert_eq!(glyphs[1].id(), font.glyph('l').unwrap().id());
        assert_eq!(glyphs[2].id(), font.glyph('l').unwrap().id());
    }

    #[test]
    fn single_line_limited_horizontal_room() {
        let font = FontCollection::from_bytes(A_FONT)
            .into_font()
            .expect("Could not create rusttype::Font");

        let (glyphs, leftover) = Layout::SingleLine(HorizontalAlign::Left)
            .calculate_glyphs_and_leftover(
                &font,
                &GlyphInfo::from(&Section {
                    text: "hello world",
                    scale: Scale::uniform(20.0),
                    bounds: (50.0, f32::INFINITY),
                    ..Section::default()
                })
            );

        if let Some(LayoutLeftover::OutOfWidthBound(_, leftover)) = leftover {
            assert_eq!(leftover.nfc_chars().collect::<String>(), "o world");
        }
        else {
            assert!(false, "Unexpected leftover {:?}", leftover);
        }

        assert_eq!(glyphs.len(), 4);
        assert_eq!(glyphs[0].position().x, 0.0);

        assert_eq!(glyphs[0].id(), font.glyph('h').unwrap().id());
        assert_eq!(glyphs[1].id(), font.glyph('e').unwrap().id());
        assert_eq!(glyphs[2].id(), font.glyph('l').unwrap().id());
        assert_eq!(glyphs[3].id(), font.glyph('l').unwrap().id());
    }
}
