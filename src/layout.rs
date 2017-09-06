use super::*;
use std::iter;
use std::iter::Skip;
use std::str::Chars;
use std::fmt;
use unicode_normalization::*;
use xi_unicode;

/// A specialised view on a [`Section`](struct.Section.html) for the purposes of calculating
/// glyph positions. Used by a [`GlyphPositioner`](trait.GlyphPositioner.html).
///
/// See [`Layout`](enum.Layout.html) for built-in positioner logic.
#[derive(Debug, Clone)]
pub struct GlyphInfo<'a> {
    /// Section text, use [`remaining_chars()`](struct.GlyphInfo.html#method.remaining_chars) instead in order
    /// to respect skip settings, ie in leftover payloads.
    pub text: &'a str,
    skip: usize,
    /// Position on screen to render text, in pixels from top-left.
    pub screen_position: (f32, f32),
    /// Max (width, height) bounds, in pixels from top-left.
    pub bounds: (f32, f32),
    /// Font scale
    pub scale: Scale,
}

impl<'a> GlyphInfo<'a> {
    /// Returns a unicode normalized char iterator, that respects the skipped chars
    /// that have already been already processed
    pub fn remaining_chars(&self) -> Skip<Recompositions<Chars<'a>>> {
        self.text.nfc().skip(self.skip)
    }

    /// Returns a new GlyphInfo instance whose
    /// [`remaining_chars()`](struct.GlyphInfo.html#method.remaining_chars) method will skip additional chars.
    pub fn skip(&self, skip: usize) -> GlyphInfo<'a> {
        let mut clone = self.clone();
        clone.skip += skip;
        clone
    }

    /// Returns a substring reference according the current skip value
    pub fn substring(&self) -> &'a str {
        let mut chars = self.text.chars();
        if self.skip != 0 {
            chars.nth(self.skip - 1);
        }
        chars.as_str()
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

/// Logic to calculate glyph positioning based on [`Font`](type.Font.html) and
/// [`GlyphInfo`](struct.GlyphInfo.html)
pub trait GlyphPositioner: Hash {
    /// Calculate a sequence of positioned glyphs to render. Custom implementations should always
    /// return the same result when called with the same arguments. If not consider disabling
    /// [`cache_glyph_positioning`](struct.GlyphBrushBuilder.html#method.cache_glyph_positioning).
    fn calculate_glyphs<'a, G>(&self, font: &Font, section: G)
        -> Vec<PositionedGlyph>
        where G: Into<GlyphInfo<'a>>;
    /// Return a rectangle according to the requested render position and bounds appropriate
    /// for the glyph layout.
    fn bounds_rect<'a, G>(&self, section: G) -> Rect<f32> where G: Into<GlyphInfo<'a>>;
}

/// Indicator that a character is a line break, soft or hard. Includes the offset position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineBreak {
    /// Soft line break (offset).
    Soft(usize),
    /// Hard line break (offset).
    Hard(usize),
}

impl LineBreak {
    /// Returns the offset of the line break, the index after the breaking character.
    pub fn offset(&self) -> usize {
        match *self {
            LineBreak::Soft(offset) | LineBreak::Hard(offset) => offset,
        }
    }
}

/// Producer of a [`LineBreak`](enum.LineBreak.html) iterator. Used to allow to the
/// [`Layout`](enum.Layout.html) to be line break aware in a generic way.
pub trait LineBreaker: fmt::Debug + Copy + Hash {
    fn line_breaks<'a>(&self, glyph_info: &GlyphInfo<'a>) -> Box<Iterator<Item=LineBreak> + 'a>;
}

/// Built-in [`GlyphPositioner`](trait.GlyphPositioner.html) implementations.
///
/// Takes generic [`LineBreaker`](trait.LineBreaker.html) to indicate the wrapping style.
/// See [`StandardLineBreaker`](struct.StandardLineBreaker.html),
/// [`AnyCharLineBreaker`](struct.AnyCharLineBreaker.html).
#[derive(Debug, Clone, Copy, Hash)]
pub enum Layout<L: LineBreaker> {
    /// Renders a single line from left-to-right according to the inner alignment.
    /// Hard breaking will end the line, partially hitting the width bound will end the line.
    SingleLine(L, HorizontalAlign),
    /// Renders multiple lines from left-to-right according to the inner alignment.
    /// Hard breaking characters will cause advancement to another line.
    /// A characters hitting the width bound will also cause another line to start.
    Wrap(L, HorizontalAlign),
}

impl Default for Layout<StandardLineBreaker> {
    fn default() -> Self { Layout::Wrap(StandardLineBreaker, HorizontalAlign::Left) }
}

impl<L: LineBreaker> GlyphPositioner for Layout<L> {
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
            Layout::SingleLine(_, HorizontalAlign::Left) |
            Layout::Wrap(_, HorizontalAlign::Left) => Rect {
                min: Point { x: screen_x, y: screen_y },
                max: Point { x: screen_x + bound_w, y: screen_y + bound_h },
            },
            Layout::SingleLine(_, HorizontalAlign::Center) |
            Layout::Wrap(_, HorizontalAlign::Center) => Rect {
                min: Point { x: screen_x - bound_w / 2.0, y: screen_y },
                max: Point { x: screen_x + bound_w / 2.0, y: screen_y + bound_h },
            },
            Layout::SingleLine(_, HorizontalAlign::Right) |
            Layout::Wrap(_, HorizontalAlign::Right) => Rect {
                min: Point { x: screen_x - bound_w, y: screen_y },
                max: Point { x: screen_x, y: screen_y + bound_h },
            },
        }
    }
}

/// [`LineBreaker`](trait.LineBreaker.html) that follows Unicode Standard Annex #14. That
/// effectively means it wraps words in a way that should work for most cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StandardLineBreaker;

impl LineBreaker for StandardLineBreaker {
    fn line_breaks<'a>(&self, glyph_info: &GlyphInfo<'a>) -> Box<Iterator<Item=LineBreak> + 'a> {
        Box::new(xi_unicode::LineBreakIterator::new(glyph_info.substring())
            .map(|(offset, hard)| {
                if hard { LineBreak::Hard(offset) } else { LineBreak::Soft(offset)}
            }))
    }
}

/// [`LineBreaker`](trait.LineBreaker.html) that soft breaks on any character, and hard breaks
/// similarly to [`StandardLineBreaker`](struct.StandardLineBreaker.html).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnyCharLineBreaker;

// Iterator that indicates all characters are soft line breaks, except hard ones which are hard.
struct AnyCharLineBreakerIter<'a> {
    chars: iter::Enumerate<Skip<Recompositions<Chars<'a>>>>,
    breaks: xi_unicode::LineBreakIterator<'a>,
    current_break: Option<(usize, bool)>,
}

impl<'a> Iterator for AnyCharLineBreakerIter<'a> {
    type Item = LineBreak;

    fn next(&mut self) -> Option<LineBreak> {
        if let Some((index, _)) = self.chars.next() {
            while self.current_break.is_some() {
                if self.current_break.as_ref().unwrap().0 < index + 1 {
                    self.current_break = self.breaks.next();
                }
                else { break; }
            }
            if let Some((break_index, true)) = self.current_break {
                if break_index == index + 1 {
                    return Some(LineBreak::Hard(break_index));
                }
            }
            Some(LineBreak::Soft(index + 1))
        }
        else { None }
    }
}

impl LineBreaker for AnyCharLineBreaker {
    fn line_breaks<'a>(&self, glyph_info: &GlyphInfo<'a>) -> Box<Iterator<Item=LineBreak> + 'a> {
        let mut unicode_breaker = xi_unicode::LineBreakIterator::new(glyph_info.substring());
        let current_break = unicode_breaker.next();

        Box::new(AnyCharLineBreakerIter {
            chars: glyph_info.remaining_chars().enumerate(),
            breaks: unicode_breaker,
            current_break
        })
    }
}

/// Describes horizontal alignment preference for positioning & bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HorizontalAlign {
    /// Leftmost character is immediately to the right of the render position.<br/>
    /// Bounds start from the render position and advance rightwards.
    Left,
    /// Leftmost & rightmost characters are equidistant to the render position.<br/>
    /// Bounds start from the render position and advance equally left & right.
    Center,
    /// Rightmost character is immetiately to the left of the render position.<br/>
    /// Bounds start from the render position and advance leftwards.
    Right,
}

/// Container for glyphs leftover/unable to fit in a layout and/or within render bounds
#[derive(Clone)]
pub enum LayoutLeftover<'a> {
    /// leftover text after a new hard line break, indicated by the
    /// [`LineBreaker`](trait.LineBreaker.html)
    HardBreak(Point<f32>, GlyphInfo<'a>),
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
            LayoutLeftover::HardBreak(..) => "HardBreak",
            LayoutLeftover::OutOfWidthBound(..) => "OutOfWidthBound",
            LayoutLeftover::OutOfHeightBound(..) => "OutOfHeightBound",
        })
    }
}

impl<L: LineBreaker> Layout<L> {
    pub fn calculate_glyphs_and_leftover<'a>(&self, font: &Font, section: &GlyphInfo<'a>)
        -> (Vec<PositionedGlyph>, Option<LayoutLeftover<'a>>)
    {
        match *self {
            Layout::SingleLine(line_breaker, h_align) =>
                single_line(font, line_breaker, h_align, section),
            Layout::Wrap(line_breaker, h_align) =>
                paragraph(font, line_breaker, h_align, section.clone()),
        }
    }
}

/// Positions glyphs in a single line left to right with the screen position marking
/// the top-left corner.
/// Returns (positioned-glyphs, text that could not be positioned (outside bounds))
fn single_line<'a, L: LineBreaker>(
    font: &Font,
    line_breaker: L,
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

    let mut line_breaks = line_breaker.line_breaks(glyph_info);
    let mut previous_break = None;
    let mut next_break = line_breaks.next();


    for (index, c) in glyph_info.remaining_chars().enumerate() {
        while next_break.is_some() {
            if next_break.as_ref().unwrap().offset() < index {
                previous_break = next_break.take();
                next_break = line_breaks.next();
            }
            else { break; }
        }

        if let Some(LineBreak::Hard(offset)) = next_break {
            if offset == index && offset != glyph_info.text.len() {
                leftover = Some(LayoutLeftover::HardBreak(caret, glyph_info.skip(index)));
                break;
            }
        }

        if c.is_control() { continue; }

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
                if let Some(line_break) = next_break {
                    if line_break.offset() == index {
                        // current char is a breaking char
                        leftover = Some(LayoutLeftover::OutOfWidthBound(
                            caret,
                            glyph_info.skip(index)));
                        break;
                    }
                }

                if let Some(line_break) = previous_break {
                    while result.len() > line_break.offset() {
                        result.pop();
                    }
                    leftover = Some(LayoutLeftover::OutOfWidthBound(
                        caret,
                        glyph_info.skip(line_break.offset())));
                }
                else {
                    // there has been no separator
                    result.clear();
                    leftover = Some(LayoutLeftover::OutOfWidthBound(
                        caret,
                        glyph_info.clone()));
                }
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
                    last.pixel_bounding_box()
                        .map(|bb| bb.max.x as f32)
                        .unwrap_or_else(|| last.position().x)
                        + last.unpositioned().h_metrics().left_side_bearing
                        - screen_x
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

fn paragraph<'a, L: LineBreaker>(
    font: &Font,
    line_breaker: L,
    h_align: HorizontalAlign,
    mut glyph_info: GlyphInfo<'a>)
    -> (Vec<PositionedGlyph>, Option<LayoutLeftover<'a>>)
{
    let v_metrics = font.v_metrics(glyph_info.scale);
    let advance_height = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;

    let mut out = vec![];
    let mut paragraph_leftover = None;
    loop {
        let (glyphs, mut leftover) = Layout::SingleLine(line_breaker, h_align)
            .calculate_glyphs_and_leftover(font, &glyph_info);
        out.extend_from_slice(&glyphs);
        if leftover.is_none() { break; }

        paragraph_leftover = match leftover.take().unwrap() {
            LayoutLeftover::HardBreak(p, remaining_glyphs) => {
                if remaining_glyphs.bounds.1 - advance_height < 0.0 {
                    Some(LayoutLeftover::OutOfHeightBound(p, remaining_glyphs))
                }
                else {
                    glyph_info = remaining_glyphs;
                    glyph_info.screen_position.1 += advance_height;
                    glyph_info.bounds.1 -= advance_height;
                    None
                }
            },
            LayoutLeftover::OutOfWidthBound(p, remaining_glyphs) => {
                // use the next line when we run out of width
                if remaining_glyphs.bounds.1 - advance_height < 0.0 {
                    Some(LayoutLeftover::OutOfHeightBound(p, remaining_glyphs))
                }
                else {
                    glyph_info = remaining_glyphs;
                    glyph_info.screen_position.1 += advance_height;
                    glyph_info.bounds.1 -= advance_height;
                    None
                }
            },
            leftover @ LayoutLeftover::OutOfHeightBound(..) => {
                Some(leftover)
            },
        };
        if paragraph_leftover.is_some() { break; }
    }
    (out, paragraph_leftover)
}

#[cfg(test)]
mod layout_test {
    use super::*;
    use std::f32;

    lazy_static! {
        static ref A_FONT: Font<'static> = FontCollection::
            from_bytes(include_bytes!("../tests/DejaVuSansMono.ttf") as &[u8])
            .into_font()
            .expect("Could not create rusttype::Font");
    }

    /// Checks the order of glyphs in the first arg iterable matches the
    /// second arg string characters
    macro_rules! assert_glyph_order {
        ($glyphs:expr, $string:expr) => {{
            let expected_len = $string.len();
            assert_eq!($glyphs.len(), expected_len, "Unexpected number of glyphs");
            let mut glyphs = $glyphs.iter();
            for c in $string.chars() {
                assert_eq!(glyphs.next().unwrap().id(), A_FONT.glyph(c).unwrap().id(),
                    "Unexpected glyph id, expecting id for char `{}`", c);
            }
        }}
    }

    #[test]
    fn single_line_chars_left_simple() {
        let _ = ::pretty_env_logger::init();

        let (glyphs, leftover) = Layout::SingleLine(AnyCharLineBreaker, HorizontalAlign::Left)
            .calculate_glyphs_and_leftover(
                &A_FONT,
                &GlyphInfo::from(&Section {
                    text: "hello world",
                    scale: Scale::uniform(20.0),
                    ..Section::default()
                })
            );

        assert!(leftover.is_none());

        assert_glyph_order!(glyphs, "hello world");

        assert_eq!(glyphs[0].position().x, 0.0);
        assert!(glyphs[10].position().x > 0.0,
            "unexpected last position {:?}", glyphs[10].position());
    }

    #[test]
    fn single_line_chars_right() {
        let _ = ::pretty_env_logger::init();

        let (glyphs, leftover) = Layout::SingleLine(AnyCharLineBreaker, HorizontalAlign::Right)
            .calculate_glyphs_and_leftover(
                &A_FONT,
                &GlyphInfo::from(&Section {
                    text: "hello world",
                    scale: Scale::uniform(20.0),
                    ..Section::default()
                })
            );

        assert!(leftover.is_none());
        assert_glyph_order!(glyphs, "hello world");
        assert!(glyphs[0].position().x < glyphs[10].position().x);
        assert!(glyphs[10].position().x <= 0.0,
            "unexpected last position {:?}", glyphs[10].position());

        // this is pretty approximate because of the pixel bounding box, but should be around 0
        let rightmost_x = glyphs[10].pixel_bounding_box().unwrap().max.x as f32
            + glyphs[10].unpositioned().h_metrics().left_side_bearing;
        assert_relative_eq!(rightmost_x, 0.0, epsilon = 1e-1);
    }

    #[test]
    fn single_line_chars_center() {
        let _ = ::pretty_env_logger::init();

        let (glyphs, leftover) = Layout::SingleLine(AnyCharLineBreaker, HorizontalAlign::Center)
            .calculate_glyphs_and_leftover(
                &A_FONT,
                &GlyphInfo::from(&Section {
                    text: "hello world",
                    scale: Scale::uniform(20.0),
                    ..Section::default()
                })
            );

        assert!(leftover.is_none());
        assert_glyph_order!(glyphs, "hello world");
        assert!(glyphs[0].position().x < 0.0,
            "unexpected first glyph position {:?}", glyphs[0].position());
        assert!(glyphs[10].position().x > 0.0,
            "unexpected last glyph position {:?}", glyphs[10].position());

        let leftmost_x = glyphs[0].position().x;
        // this is pretty approximate because of the pixel bounding box, but should be around
        // the negation of the left
        let rightmost_x = glyphs[10].pixel_bounding_box().unwrap().max.x as f32
            + glyphs[10].unpositioned().h_metrics().left_side_bearing;
        assert_relative_eq!(rightmost_x, -leftmost_x, epsilon = 1e-1);
    }

    #[test]
    fn wrap_word_left() {
        let _ = ::pretty_env_logger::init();

        let (glyphs, leftover) = Layout::SingleLine(StandardLineBreaker, HorizontalAlign::Left)
            .calculate_glyphs_and_leftover(
                &A_FONT,
                &GlyphInfo::from(&Section {
                    text: "hello what's _happening_?",
                    scale: Scale::uniform(20.0),
                    bounds: (85.0, f32::INFINITY), // should only be enough room for the 1st word
                    ..Section::default()
                })
            );

        if let Some(LayoutLeftover::OutOfWidthBound(_, leftover)) = leftover {
            assert_eq!(leftover.remaining_chars().collect::<String>(), "what's _happening_?");
        }
        else {
            assert!(false, "Unexpected leftover {:?}", leftover);
        }

        assert_glyph_order!(glyphs, "hello ");
        assert_eq!(glyphs[0].position().x, 0.0);
        assert!(glyphs[5].position().x > 0.0,
            "unexpected last position {:?}", glyphs[5].position());

        let (glyphs, leftover) = Layout::SingleLine(StandardLineBreaker, HorizontalAlign::Left)
            .calculate_glyphs_and_leftover(
                &A_FONT,
                &GlyphInfo::from(&Section {
                    text: "hello what's _happening_?",
                    scale: Scale::uniform(20.0),
                    bounds: (125.0, f32::INFINITY), // should only be enough room for the 1,2 words
                    ..Section::default()
                })
            );

        if let Some(LayoutLeftover::OutOfWidthBound(_, leftover)) = leftover {
            assert_eq!(leftover.remaining_chars().collect::<String>(), "_happening_?");
        }
        else {
            assert!(false, "Unexpected leftover {:?}", leftover);
        }

        assert_glyph_order!(glyphs, "hello what's ");
        assert_eq!(glyphs[0].position().x, 0.0);
        assert!(glyphs[12].position().x > 0.0,
            "unexpected last position {:?}", glyphs[12].position());
    }

    #[test]
    fn single_line_chars_left_finish_at_newline() {
        let _ = ::pretty_env_logger::init();

        let (glyphs, leftover) = Layout::SingleLine(AnyCharLineBreaker, HorizontalAlign::Left)
            .calculate_glyphs_and_leftover(
                &A_FONT,
                &GlyphInfo::from(&Section {
                    text: "hello\nworld",
                    scale: Scale::uniform(20.0),
                    ..Section::default()
                })
            );

        if let Some(LayoutLeftover::HardBreak(_, leftover)) = leftover {
            assert_eq!(leftover.remaining_chars().collect::<String>(), "world");
        }
        else {
            assert!(false, "Unexpected leftover {:?}", leftover);
        }
        assert_glyph_order!(glyphs, "hello");
        assert_eq!(glyphs[0].position().x, 0.0);
        assert!(glyphs[4].position().x > 0.0,
            "unexpected last position {:?}", glyphs[4].position());
    }

    #[test]
    fn single_line_little_verticle_room() {
        let _ = ::pretty_env_logger::init();

        let (glyphs, leftover) = Layout::SingleLine(AnyCharLineBreaker, HorizontalAlign::Left)
            .calculate_glyphs_and_leftover(
                &A_FONT,
                &GlyphInfo::from(&Section {
                    text: "hello world",
                    scale: Scale::uniform(20.0),
                    bounds: (f32::INFINITY, 5.0),
                    ..Section::default()
                })
            );

        assert!(leftover.is_none(), "unexpected leftover {:?}", leftover);
        assert_glyph_order!(glyphs, "hll ld"); // e,o,w,o,r hidden

        // letter `l` should be in the same place as when all the word is visible
        let (all_glyphs, _) = Layout::SingleLine(StandardLineBreaker, HorizontalAlign::Left)
            .calculate_glyphs_and_leftover(
                &A_FONT,
                &GlyphInfo::from(&Section {
                    text: "hello world",
                    scale: Scale::uniform(20.0),
                    ..Section::default()
                })
            );
        assert_eq!(all_glyphs[9].id(), A_FONT.glyph('l').unwrap().id());
        assert_relative_eq!(all_glyphs[9].position().x, glyphs[4].position().x);
        assert_relative_eq!(all_glyphs[9].position().y, glyphs[4].position().y);
    }

    #[test]
    fn single_line_little_verticle_room_tail_lost() {
        let _ = ::pretty_env_logger::init();

        let (glyphs, leftover) = Layout::SingleLine(AnyCharLineBreaker, HorizontalAlign::Left)
            .calculate_glyphs_and_leftover(
                &A_FONT,
                &GlyphInfo::from(&Section {
                    text: "hellowor__",
                    scale: Scale::uniform(20.0),
                    // vertical bound of 5px means only tall letters will be seen
                    bounds: (f32::INFINITY, 5.0),
                    ..Section::default()
                })
            );

        if let Some(LayoutLeftover::OutOfHeightBound(_, leftover)) = leftover {
            assert_eq!(leftover.remaining_chars().collect::<String>(), "owor__");
        }
        else {
            assert!(false, "Unexpected leftover {:?}", leftover);
        }

        assert_glyph_order!(glyphs, "hll"); // e hidden
    }

    #[test]
    fn single_line_limited_horizontal_room() {
        let _ = ::pretty_env_logger::init();

        let (glyphs, leftover) = Layout::SingleLine(AnyCharLineBreaker, HorizontalAlign::Left)
            .calculate_glyphs_and_leftover(
                &A_FONT,
                &GlyphInfo::from(&Section {
                    text: "hello world",
                    scale: Scale::uniform(20.0),
                    bounds: (50.0, f32::INFINITY),
                    ..Section::default()
                })
            );

        if let Some(LayoutLeftover::OutOfWidthBound(_, leftover)) = leftover {
            assert_eq!(leftover.remaining_chars().collect::<String>(), "o world");
        }
        else {
            assert!(false, "Unexpected leftover {:?}", leftover);
        }

        assert_glyph_order!(glyphs, "hell");
        assert_eq!(glyphs[0].position().x, 0.0);
    }

    #[test]
    fn wrap_layout_with_new_lines() {
        let _ = ::pretty_env_logger::init();

        let test_str = "Autumn moonlight\n\
            a worm digs silently\n\
            into the chestnut.";

        let (glyphs, leftover) = Layout::Wrap(StandardLineBreaker, HorizontalAlign::Left)
            .calculate_glyphs_and_leftover(
                &A_FONT,
                &GlyphInfo::from(&Section {
                    text: test_str,
                    scale: Scale::uniform(20.0),
                    ..Section::default()
                })
            );

        assert!(leftover.is_none(), "Unexpected leftover {:?}", leftover);
        assert_glyph_order!(glyphs, "Autumn moonlighta worm digs silentlyinto the chestnut.");
        assert!(glyphs[16].position().y > glyphs[0].position().y,
            "second line should be lower than first");
        assert!(glyphs[36].position().y > glyphs[16].position().y,
            "third line should be lower than second");
    }
}
