use super::*;
use characters::Characters;
use std::hash::BuildHasher;

/// Built-in [`GlyphPositioner`](trait.GlyphPositioner.html) implementations.
///
/// Takes generic [`LineBreaker`](trait.LineBreaker.html) to indicate the wrapping style.
/// See [`BuiltInLineBreaker`](enum.BuiltInLineBreaker.html).
///
/// # Example
/// ```
/// # use gfx_glyph::*;
/// let layout = Layout::default().h_align(HorizontalAlign::Right);
/// # let _layout = layout;
/// ```
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Layout<L: LineBreaker> {
    /// Renders a single line from left-to-right according to the inner alignment.
    /// Hard breaking will end the line, partially hitting the width bound will end the line.
    SingleLine {
        line_breaker: L,
        h_align: HorizontalAlign,
        v_align: VerticalAlign,
    },
    /// Renders multiple lines from left-to-right according to the inner alignment.
    /// Hard breaking characters will cause advancement to another line.
    /// A characters hitting the width bound will also cause another line to start.
    Wrap {
        line_breaker: L,
        h_align: HorizontalAlign,
        v_align: VerticalAlign,
    },
}

impl Default for Layout<BuiltInLineBreaker> {
    fn default() -> Self {
        Layout::default_wrap()
    }
}

impl Layout<BuiltInLineBreaker> {
    pub fn default_single_line() -> Self {
        Layout::SingleLine {
            line_breaker: BuiltInLineBreaker::default(),
            h_align: HorizontalAlign::Left,
            v_align: VerticalAlign::Top,
        }
    }

    pub fn default_wrap() -> Self {
        Layout::Wrap {
            line_breaker: BuiltInLineBreaker::default(),
            h_align: HorizontalAlign::Left,
            v_align: VerticalAlign::Top,
        }
    }
}

impl<L: LineBreaker> Layout<L> {
    /// Returns an identical `Layout` but with the input `h_align`
    pub fn h_align(self, h_align: HorizontalAlign) -> Self {
        use Layout::*;
        match self {
            SingleLine {
                line_breaker,
                v_align,
                ..
            } => SingleLine {
                line_breaker,
                v_align,
                h_align,
            },
            Wrap {
                line_breaker,
                v_align,
                ..
            } => Wrap {
                line_breaker,
                v_align,
                h_align,
            },
        }
    }

    /// Returns an identical `Layout` but with the input `v_align`
    pub fn v_align(self, v_align: VerticalAlign) -> Self {
        match v_align {
            VerticalAlign::Top => self,
        }
    }

    /// Returns an identical `Layout` but with the input `line_breaker`
    pub fn line_breaker<L2: LineBreaker>(self, line_breaker: L2) -> Layout<L2> {
        use Layout::*;
        match self {
            SingleLine {
                h_align, v_align, ..
            } => SingleLine {
                line_breaker,
                v_align,
                h_align,
            },
            Wrap {
                h_align, v_align, ..
            } => Wrap {
                line_breaker,
                v_align,
                h_align,
            },
        }
    }
}

impl<L: LineBreaker> GlyphPositioner for Layout<L> {
    fn calculate_glyphs<'font, H: BuildHasher>(
        &self,
        font_map: &HashMap<FontId, Font<'font>, H>,
        section: &VariedSection,
    ) -> Vec<(PositionedGlyph<'font>, Color, FontId)> {
        use Layout::{SingleLine, Wrap};

        let VariedSection {
            screen_position,
            bounds: (bound_w, bound_h),
            ..
        } = *section;

        match *self {
            SingleLine {
                h_align,
                v_align,
                line_breaker,
            } => Characters::new(font_map, section.text.iter(), line_breaker)
                .words()
                .lines(bound_w)
                .next()
                .map(|line| line.aligned_on_screen(screen_position, h_align, v_align))
                .unwrap_or_default(),

            Wrap {
                h_align,
                v_align,
                line_breaker,
            } => {
                let mut out = vec![];
                let mut caret = screen_position;

                let mut lines = Characters::new(font_map, section.text.iter(), line_breaker)
                    .words()
                    .lines(bound_w);

                for line in lines {
                    if caret.1 >= screen_position.1 + bound_h {
                        break;
                    }

                    let line_h = line.max_v_metrics.ascent - line.max_v_metrics.descent
                        + line.max_v_metrics.line_gap;

                    out.extend(line.aligned_on_screen(caret, h_align, v_align));
                    caret.1 += line_h;
                }

                out
            }
        }
    }

    fn bounds_rect(&self, section: &VariedSection) -> Rect<f32> {
        use Layout::{SingleLine, Wrap};

        let VariedSection {
            screen_position: (screen_x, screen_y),
            bounds: (bound_w, bound_h),
            ..
        } = *section;

        let h_align = match *self {
            Wrap { h_align, .. } | SingleLine { h_align, .. } => h_align,
        };

        match h_align {
            HorizontalAlign::Left => Rect {
                min: Point {
                    x: screen_x,
                    y: screen_y,
                },
                max: Point {
                    x: screen_x + bound_w,
                    y: screen_y + bound_h,
                },
            },
            HorizontalAlign::Center => Rect {
                min: Point {
                    x: screen_x - bound_w / 2.0,
                    y: screen_y,
                },
                max: Point {
                    x: screen_x + bound_w / 2.0,
                    y: screen_y + bound_h,
                },
            },
            HorizontalAlign::Right => Rect {
                min: Point {
                    x: screen_x - bound_w,
                    y: screen_y,
                },
                max: Point {
                    x: screen_x,
                    y: screen_y + bound_h,
                },
            },
        }
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

/// Describes vertical alignment preference for positioning & bounds. Currently a placeholder
/// for future functionality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VerticalAlign {
    /// Characters/bounds start underneath the render position and progress downwards
    Top,
}

#[cfg(test)]
mod layout_test {
    use super::*;
    use ordered_float::OrderedFloat;
    use std::collections::*;
    use std::f32;
    use BuiltInLineBreaker::*;

    lazy_static! {
        static ref A_FONT: Font<'static> =
            Font::from_bytes(include_bytes!("../../tests/DejaVuSansMono.ttf") as &[u8])
                .expect("Could not create rusttype::Font");
        static ref FONT_MAP: HashMap<FontId, Font<'static>> =
            vec![(FontId(0), A_FONT.clone())].into_iter().collect();
    }

    /// Checks the order of glyphs in the first arg iterable matches the
    /// second arg string characters
    /// $glyphs: Vec<(PositionedGlyph<'font>, Color, FontId)>
    macro_rules! assert_glyph_order {
        ($glyphs:expr, $string:expr) => {{
            let expected_len = $string.chars().count();
            assert_eq!($glyphs.len(), expected_len, "Unexpected number of glyphs");
            let mut glyphs = $glyphs.iter();
            for c in $string.chars() {
                assert_eq!(
                    glyphs.next().unwrap().0.id(),
                    A_FONT.glyph(c).id(),
                    "Unexpected glyph id, expecting id for char `{}`",
                    c
                );
            }
        }};
    }

    #[test]
    fn single_line_chars_left_simple() {
        let glyphs = Layout::default_single_line()
            .line_breaker(AnyCharLineBreaker)
            .calculate_glyphs(
                &FONT_MAP,
                &Section {
                    text: "hello world",
                    scale: Scale::uniform(20.0),
                    ..Section::default()
                }.into(),
            );

        assert_glyph_order!(glyphs, "helloworld");

        assert_relative_eq!(glyphs[0].0.position().x, 0.0);
        let last_glyph = &glyphs.last().unwrap().0;
        assert!(
            last_glyph.position().x > 0.0,
            "unexpected last position {:?}",
            last_glyph.position()
        );
    }

    #[test]
    fn single_line_chars_right() {
        let glyphs = Layout::default_single_line()
            .line_breaker(AnyCharLineBreaker)
            .h_align(HorizontalAlign::Right)
            .calculate_glyphs(
                &FONT_MAP,
                &Section {
                    text: "hello world",
                    scale: Scale::uniform(20.0),
                    ..Section::default()
                }.into(),
            );

        assert_glyph_order!(glyphs, "helloworld");
        let last_glyph = &glyphs.last().unwrap().0;
        assert!(glyphs[0].0.position().x < last_glyph.position().x);
        assert!(
            last_glyph.position().x <= 0.0,
            "unexpected last position {:?}",
            last_glyph.position()
        );

        // this is pretty approximate because of the pixel bounding box, but should be around 0
        let rightmost_x = last_glyph.pixel_bounding_box().unwrap().max.x as f32
            + last_glyph.unpositioned().h_metrics().left_side_bearing;
        assert_relative_eq!(rightmost_x, 0.0, epsilon = 1e-1);
    }

    #[test]
    fn single_line_chars_center() {
        let glyphs = Layout::default_single_line()
            .line_breaker(AnyCharLineBreaker)
            .h_align(HorizontalAlign::Center)
            .calculate_glyphs(
                &FONT_MAP,
                &Section {
                    text: "hello world",
                    scale: Scale::uniform(20.0),
                    ..Section::default()
                }.into(),
            );

        assert_glyph_order!(glyphs, "helloworld");
        assert!(
            glyphs[0].0.position().x < 0.0,
            "unexpected first glyph position {:?}",
            glyphs[0].0.position()
        );

        let last_glyph = &glyphs.last().unwrap().0;
        assert!(
            last_glyph.position().x > 0.0,
            "unexpected last glyph position {:?}",
            last_glyph.position()
        );

        let leftmost_x = glyphs[0].0.position().x;
        // this is pretty approximate because of the pixel bounding box, but should be around
        // the negation of the left
        let rightmost_x = last_glyph.pixel_bounding_box().unwrap().max.x as f32
            + last_glyph.unpositioned().h_metrics().left_side_bearing;
        assert_relative_eq!(rightmost_x, -leftmost_x, epsilon = 1e-1);
    }

    #[test]
    fn single_line_chars_left_finish_at_newline() {
        let glyphs = Layout::default_single_line()
            .line_breaker(AnyCharLineBreaker)
            .calculate_glyphs(
                &FONT_MAP,
                &Section {
                    text: "hello\nworld",
                    scale: Scale::uniform(20.0),
                    ..Section::default()
                }.into(),
            );

        assert_glyph_order!(glyphs, "hello");
        assert_relative_eq!(glyphs[0].0.position().x, 0.0);
        assert!(
            glyphs[4].0.position().x > 0.0,
            "unexpected last position {:?}",
            glyphs[4].0.position()
        );
    }

    #[test]
    fn wrap_word_left() {
        let glyphs = Layout::default_single_line().calculate_glyphs(
            &FONT_MAP,
            &Section {
                text: "hello what's _happening_?",
                scale: Scale::uniform(20.0),
                bounds: (85.0, f32::INFINITY), // should only be enough room for the 1st word
                ..Section::default()
            }.into(),
        );

        assert_glyph_order!(glyphs, "hello");
        assert_relative_eq!(glyphs[0].0.position().x, 0.0);
        let last_glyph = &glyphs.last().unwrap().0;
        assert!(
            last_glyph.position().x > 0.0,
            "unexpected last position {:?}",
            last_glyph.position()
        );

        let glyphs = Layout::default_single_line().calculate_glyphs(
            &FONT_MAP,
            &Section {
                text: "hello what's _happening_?",
                scale: Scale::uniform(20.0),
                bounds: (125.0, f32::INFINITY), // should only be enough room for the 1,2 words
                ..Section::default()
            }.into(),
        );

        assert_glyph_order!(glyphs, "hellowhat's");
        assert_relative_eq!(glyphs[0].0.position().x, 0.0);
        let last_glyph = &glyphs.last().unwrap().0;
        assert!(
            last_glyph.position().x > 0.0,
            "unexpected last position {:?}",
            last_glyph.position()
        );
    }

    #[test]
    fn single_line_limited_horizontal_room() {
        let glyphs = Layout::default_single_line()
            .line_breaker(AnyCharLineBreaker)
            .calculate_glyphs(
                &FONT_MAP,
                &Section {
                    text: "helloworld",
                    scale: Scale::uniform(20.0),
                    bounds: (50.0, f32::INFINITY),
                    ..Section::default()
                }.into(),
            );

        assert_glyph_order!(glyphs, "hell");
        assert_relative_eq!(glyphs[0].0.position().x, 0.0);
    }

    #[test]
    fn wrap_layout_with_new_lines() {
        let test_str = "Autumn moonlight\n\
                        a worm digs silently\n\
                        into the chestnut.";

        let glyphs = Layout::default_wrap().calculate_glyphs(
            &FONT_MAP,
            &Section {
                text: test_str,
                scale: Scale::uniform(20.0),
                ..Section::default()
            }.into(),
        );

        assert_glyph_order!(glyphs, "Autumnmoonlightawormdigssilentlyintothechestnut.");
        assert!(
            glyphs[16].0.position().y > glyphs[0].0.position().y,
            "second line should be lower than first"
        );
        assert!(
            glyphs[36].0.position().y > glyphs[16].0.position().y,
            "third line should be lower than second"
        );
    }

    #[test]
    fn leftover_max_vmetrics() {
        let glyphs = Layout::default_single_line().calculate_glyphs(
            &FONT_MAP,
            &VariedSection {
                text: vec![
                    SectionText {
                        text: "Autumn moonlight, ",
                        scale: Scale::uniform(30.0),
                        ..SectionText::default()
                    },
                    SectionText {
                        text: "a worm digs silently ",
                        scale: Scale::uniform(40.0),
                        ..SectionText::default()
                    },
                    SectionText {
                        text: "into the chestnut.",
                        scale: Scale::uniform(10.0),
                        ..SectionText::default()
                    },
                ],
                bounds: (750.0, f32::INFINITY),
                ..VariedSection::default()
            },
        );

        let max_v_metrics = A_FONT.v_metrics(Scale::uniform(40.0));

        for g in glyphs {
            println!("{:?}", (g.0.scale(), g.0.position()));
            // all glyphs should have the same ascent drawing position
            let y_pos = g.0.position().y;
            assert_relative_eq!(y_pos, max_v_metrics.ascent);
        }
    }

    #[test]
    fn eol_new_line_hard_breaks() {
        let glyphs = Layout::default_wrap().calculate_glyphs(
            &FONT_MAP,
            &VariedSection {
                text: vec![
                    SectionText {
                        text: "Autumn moonlight, \n",
                        ..SectionText::default()
                    },
                    SectionText {
                        text: "a worm digs silently ",
                        ..SectionText::default()
                    },
                    SectionText {
                        text: "\n",
                        ..SectionText::default()
                    },
                    SectionText {
                        text: "into the chestnut.",
                        ..SectionText::default()
                    },
                ],
                ..VariedSection::default()
            },
        );

        let y_ords: HashSet<OrderedFloat<f32>> = glyphs
            .iter()
            .map(|g| OrderedFloat(g.0.position().y))
            .collect();

        println!("Y ords: {:?}", y_ords);
        assert_eq!(y_ords.len(), 3, "expected 3 distinct lines");

        assert_glyph_order!(glyphs, "Autumnmoonlight,awormdigssilentlyintothechestnut.");

        let line_2_glyph = &glyphs[16].0;
        let line_3_glyph = &&glyphs[33].0;
        assert_eq!(line_2_glyph.id(), A_FONT.glyph('a').id());
        assert!(line_2_glyph.position().y > glyphs[0].0.position().y);

        assert_eq!(line_3_glyph.id(), A_FONT.glyph('i').id());
        assert!(line_3_glyph.position().y > line_2_glyph.position().y);
    }

    #[test]
    fn single_line_multibyte_chars_finish_at_break() {
        let unicode_str = "❤❤é❤❤\n❤ß❤";
        assert_eq!(
            unicode_str, "\u{2764}\u{2764}\u{e9}\u{2764}\u{2764}\n\u{2764}\u{df}\u{2764}",
            "invisible char funny business",
        );
        assert_eq!(unicode_str.len(), 23);
        assert_eq!(
            xi_unicode::LineBreakIterator::new(unicode_str).find(|n| n.1),
            Some((15, true)),
        );

        let glyphs = Layout::default_single_line().calculate_glyphs(
            &FONT_MAP,
            &Section {
                text: unicode_str,
                scale: Scale::uniform(20.0),
                ..Section::default()
            }.into(),
        );

        assert_glyph_order!(glyphs, "\u{2764}\u{2764}\u{e9}\u{2764}\u{2764}");
        assert_relative_eq!(glyphs[0].0.position().x, 0.0);
        assert!(
            glyphs[4].0.position().x > 0.0,
            "unexpected last position {:?}",
            glyphs[4].0.position()
        );
    }

    #[test]
    fn no_inherent_section_break() {
        let glyphs = Layout::default_wrap().calculate_glyphs(
            &FONT_MAP,
            &VariedSection {
                text: vec![
                    SectionText {
                        text: "The ",
                        ..SectionText::default()
                    },
                    SectionText {
                        text: "moon",
                        ..SectionText::default()
                    },
                    SectionText {
                        text: "light",
                        ..SectionText::default()
                    },
                ],
                bounds: (50.0, ::std::f32::INFINITY),
                ..VariedSection::default()
            },
        );

        assert_glyph_order!(glyphs, "The");

        let y_ords: HashSet<OrderedFloat<f32>> = glyphs
            .iter()
            .map(|g| OrderedFloat(g.0.position().y))
            .collect();

        assert_eq!(y_ords.len(), 1, "Y ords: {:?}", y_ords);
    }
}
