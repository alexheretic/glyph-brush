use super::{
    BuiltInLineBreaker, Color, FontId, FontMap, GlyphPositioner, LineBreaker, PositionedGlyph,
    Rect, SectionGeometry, SectionText,
};
use crate::{characters::Characters, rusttype::point, GlyphChange};
use full_rusttype::vector;
use std::{borrow::Cow, mem};

/// Built-in [`GlyphPositioner`](trait.GlyphPositioner.html) implementations.
///
/// Takes generic [`LineBreaker`](trait.LineBreaker.html) to indicate the wrapping style.
/// See [`BuiltInLineBreaker`](enum.BuiltInLineBreaker.html).
///
/// # Example
/// ```
/// # use glyph_brush_layout::*;
/// let layout = Layout::default().h_align(HorizontalAlign::Right);
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
    #[inline]
    fn default() -> Self {
        Layout::default_wrap()
    }
}

impl Layout<BuiltInLineBreaker> {
    #[inline]
    pub fn default_single_line() -> Self {
        Layout::SingleLine {
            line_breaker: BuiltInLineBreaker::default(),
            h_align: HorizontalAlign::Left,
            v_align: VerticalAlign::Top,
        }
    }

    #[inline]
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
        use crate::Layout::*;
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
        use crate::Layout::*;
        match self {
            SingleLine {
                line_breaker,
                h_align,
                ..
            } => SingleLine {
                line_breaker,
                v_align,
                h_align,
            },
            Wrap {
                line_breaker,
                h_align,
                ..
            } => Wrap {
                line_breaker,
                v_align,
                h_align,
            },
        }
    }

    /// Returns an identical `Layout` but with the input `line_breaker`
    pub fn line_breaker<L2: LineBreaker>(self, line_breaker: L2) -> Layout<L2> {
        use crate::Layout::*;
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
    fn calculate_glyphs<'font, F: FontMap<'font>>(
        &self,
        font_map: &F,
        geometry: &SectionGeometry,
        sections: &[SectionText<'_>],
    ) -> Vec<(PositionedGlyph<'font>, Color, FontId)> {
        use crate::Layout::{SingleLine, Wrap};

        let SectionGeometry {
            screen_position,
            bounds: (bound_w, bound_h),
            ..
        } = *geometry;

        match *self {
            SingleLine {
                h_align,
                v_align,
                line_breaker,
            } => Characters::new(font_map, sections.iter(), line_breaker)
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
                let v_align_top = v_align == VerticalAlign::Top;

                let lines = Characters::new(font_map, sections.iter(), line_breaker)
                    .words()
                    .lines(bound_w);

                for line in lines {
                    // top align can bound check & exit early
                    if v_align_top && caret.1 >= screen_position.1 + bound_h {
                        break;
                    }

                    let line_height = line.line_height();
                    out.extend(line.aligned_on_screen(caret, h_align, VerticalAlign::Top));
                    caret.1 += line_height;
                }

                if !out.is_empty() {
                    match v_align {
                        // already aligned
                        VerticalAlign::Top => {}
                        // convert from top
                        VerticalAlign::Center | VerticalAlign::Bottom => {
                            let shift_up = if v_align == VerticalAlign::Center {
                                (caret.1 - screen_position.1) / 2.0
                            } else {
                                caret.1 - screen_position.1
                            };

                            let (min_x, max_x) = h_align.x_bounds(screen_position.0, bound_w);
                            let (min_y, max_y) = v_align.y_bounds(screen_position.1, bound_h);

                            // y-position and filter out-of-bounds glyphs
                            let shifted: Vec<_> = out
                                .drain(..)
                                .filter_map(|(mut g, color, font)| {
                                    let mut pos = g.position();
                                    pos.y -= shift_up;
                                    g.set_position(pos);
                                    Some((g, color, font)).filter(|(g, ..)| {
                                        g.pixel_bounding_box()
                                            .map(|bb| {
                                                bb.max.y as f32 >= min_y
                                                    && bb.min.y as f32 <= max_y
                                                    && bb.max.x as f32 >= min_x
                                                    && bb.min.x as f32 <= max_x
                                            })
                                            .unwrap_or(false)
                                    })
                                })
                                .collect();
                            mem::replace(&mut out, shifted);
                        }
                    }
                }

                out
            }
        }
    }

    fn bounds_rect(&self, geometry: &SectionGeometry) -> Rect<f32> {
        use crate::Layout::{SingleLine, Wrap};

        let SectionGeometry {
            screen_position: (screen_x, screen_y),
            bounds: (bound_w, bound_h),
        } = *geometry;

        let (h_align, v_align) = match *self {
            Wrap {
                h_align, v_align, ..
            }
            | SingleLine {
                h_align, v_align, ..
            } => (h_align, v_align),
        };

        let (x_min, x_max) = h_align.x_bounds(screen_x, bound_w);
        let (y_min, y_max) = v_align.y_bounds(screen_y, bound_h);

        Rect {
            min: point(x_min, y_min),
            max: point(x_max, y_max),
        }
    }

    #[allow(clippy::float_cmp)]
    fn recalculate_glyphs<'font, F>(
        &self,
        previous: Cow<'_, Vec<(PositionedGlyph<'font>, Color, FontId)>>,
        change: GlyphChange,
        fonts: &F,
        geometry: &SectionGeometry,
        sections: &[SectionText<'_>],
    ) -> Vec<(PositionedGlyph<'font>, Color, FontId)>
    where
        F: FontMap<'font>,
    {
        match change {
            GlyphChange::Geometry(old) if old.bounds == geometry.bounds => {
                // position change
                let adjustment = vector(
                    geometry.screen_position.0 - old.screen_position.0,
                    geometry.screen_position.1 - old.screen_position.1,
                );

                let mut glyphs = previous.into_owned();
                for (glyph, ..) in &mut glyphs {
                    let new_pos = glyph.position() + adjustment;
                    glyph.set_position(new_pos);
                }

                glyphs
            }
            GlyphChange::Color if !sections.is_empty() && !previous.is_empty() => {
                let new_color = sections[0].color;
                if sections.iter().all(|s| s.color == new_color) {
                    // if only the color changed, but the new section only use a single color
                    // we can simply set all the olds to the new color
                    let mut glyphs = previous.into_owned();
                    for (_, color, ..) in &mut glyphs {
                        *color = new_color;
                    }
                    glyphs
                } else {
                    self.calculate_glyphs(fonts, geometry, sections)
                }
            }
            GlyphChange::Alpha if !sections.is_empty() && !previous.is_empty() => {
                let new_alpha = sections[0].color[3];
                if sections.iter().all(|s| s.color[3] == new_alpha) {
                    // if only the alpha changed, but the new section only uses a single alpha
                    // we can simply set all the olds to the new alpha
                    let mut glyphs = previous.into_owned();
                    for (_, color, ..) in &mut glyphs {
                        color[3] = new_alpha;
                    }
                    glyphs
                } else {
                    self.calculate_glyphs(fonts, geometry, sections)
                }
            }
            _ => self.calculate_glyphs(fonts, geometry, sections),
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

impl HorizontalAlign {
    #[inline]
    pub(crate) fn x_bounds(self, screen_x: f32, bound_w: f32) -> (f32, f32) {
        let (min, max) = match self {
            HorizontalAlign::Left => (screen_x, screen_x + bound_w),
            HorizontalAlign::Center => (screen_x - bound_w / 2.0, screen_x + bound_w / 2.0),
            HorizontalAlign::Right => (screen_x - bound_w, screen_x),
        };

        (min.floor(), max.ceil())
    }
}

/// Describes vertical alignment preference for positioning & bounds. Currently a placeholder
/// for future functionality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VerticalAlign {
    /// Characters/bounds start underneath the render position and progress downwards.
    Top,
    /// Characters/bounds center at the render position and progress outward equally.
    Center,
    /// Characters/bounds start above the render position and progress upward.
    Bottom,
}

impl VerticalAlign {
    #[inline]
    pub(crate) fn y_bounds(self, screen_y: f32, bound_h: f32) -> (f32, f32) {
        let (min, max) = match self {
            VerticalAlign::Top => (screen_y, screen_y + bound_h),
            VerticalAlign::Center => (screen_y - bound_h / 2.0, screen_y + bound_h / 2.0),
            VerticalAlign::Bottom => (screen_y - bound_h, screen_y),
        };

        (min.floor(), max.ceil())
    }
}

#[cfg(test)]
mod bounds_test {
    use super::*;
    use std::f32::INFINITY as inf;

    #[test]
    fn v_align_y_bounds_inf() {
        assert_eq!(VerticalAlign::Top.y_bounds(0.0, inf), (0.0, inf));
        assert_eq!(VerticalAlign::Center.y_bounds(0.0, inf), (-inf, inf));
        assert_eq!(VerticalAlign::Bottom.y_bounds(0.0, inf), (-inf, 0.0));
    }

    #[test]
    fn h_align_x_bounds_inf() {
        assert_eq!(HorizontalAlign::Left.x_bounds(0.0, inf), (0.0, inf));
        assert_eq!(HorizontalAlign::Center.x_bounds(0.0, inf), (-inf, inf));
        assert_eq!(HorizontalAlign::Right.x_bounds(0.0, inf), (-inf, 0.0));
    }
}

#[cfg(test)]
mod layout_test {
    use super::*;
    use crate::{
        rusttype::{Font, Scale},
        BuiltInLineBreaker::*,
    };
    use approx::assert_relative_eq;
    use once_cell::sync::Lazy;
    use ordered_float::OrderedFloat;
    use std::{collections::*, f32};
    use xi_unicode;

    static A_FONT: Lazy<Font<'static>> = Lazy::new(|| {
        Font::from_bytes(include_bytes!("../../fonts/DejaVuSansMono.ttf") as &[u8])
            .expect("Could not create rusttype::Font")
    });
    static CJK_FONT: Lazy<Font<'static>> = Lazy::new(|| {
        Font::from_bytes(include_bytes!("../../fonts/WenQuanYiMicroHei.ttf") as &[u8])
            .expect("Could not create rusttype::Font")
    });
    static FONT_MAP: Lazy<Vec<Font<'static>>> =
        Lazy::new(|| vec![A_FONT.clone(), CJK_FONT.clone()]);

    /// Checks the order of glyphs in the first arg iterable matches the
    /// second arg string characters
    /// $glyphs: Vec<(PositionedGlyph<'font>, Color, FontId)>
    macro_rules! assert_glyph_order {
        ($glyphs:expr, $string:expr) => {
            assert_glyph_order!($glyphs, $string, font = A_FONT)
        };
        ($glyphs:expr, $string:expr, font = $font:expr) => {{
            let expected_len = $string.chars().count();
            assert_eq!($glyphs.len(), expected_len, "Unexpected number of glyphs");
            let mut glyphs = $glyphs.iter();
            for c in $string.chars() {
                assert_eq!(
                    glyphs.next().unwrap().0.id(),
                    $font.glyph(c).id(),
                    "Unexpected glyph id, expecting id for char `{}`",
                    c
                );
            }
        }};
    }

    /// Compile test for trait stability
    #[allow(unused)]
    #[derive(Hash)]
    enum SimpleCustomGlyphPositioner {}
    impl GlyphPositioner for SimpleCustomGlyphPositioner {
        fn calculate_glyphs<'font, F: FontMap<'font>>(
            &self,
            _: &F,
            _: &SectionGeometry,
            _: &[SectionText<'_>],
        ) -> Vec<(PositionedGlyph<'font>, Color, FontId)> {
            vec![]
        }

        /// Return a screen rectangle according to the requested render position and bounds
        /// appropriate for the glyph layout.
        fn bounds_rect(&self, _: &SectionGeometry) -> Rect<f32> {
            Rect {
                min: point(0.0, 0.0),
                max: point(0.0, 0.0),
            }
        }
    }

    #[test]
    fn zero_scale_glyphs() {
        let glyphs = Layout::default_single_line()
            .line_breaker(AnyCharLineBreaker)
            .calculate_glyphs(
                &*FONT_MAP,
                &SectionGeometry::default(),
                &[SectionText {
                    text: "hello world",
                    scale: Scale::uniform(0.0),
                    ..SectionText::default()
                }],
            );

        assert!(glyphs.is_empty(), "{:?}", glyphs);
    }

    #[test]
    fn negative_scale_glyphs() {
        let glyphs = Layout::default_single_line()
            .line_breaker(AnyCharLineBreaker)
            .calculate_glyphs(
                &*FONT_MAP,
                &SectionGeometry::default(),
                &[SectionText {
                    text: "hello world",
                    scale: Scale::uniform(-20.0),
                    ..SectionText::default()
                }],
            );

        assert!(glyphs.is_empty(), "{:?}", glyphs);
    }

    #[test]
    fn single_line_chars_left_simple() {
        let glyphs = Layout::default_single_line()
            .line_breaker(AnyCharLineBreaker)
            .calculate_glyphs(
                &*FONT_MAP,
                &SectionGeometry::default(),
                &[SectionText {
                    text: "hello world",
                    scale: Scale::uniform(20.0),
                    ..SectionText::default()
                }],
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
                &*FONT_MAP,
                &SectionGeometry::default(),
                &[SectionText {
                    text: "hello world",
                    scale: Scale::uniform(20.0),
                    ..SectionText::default()
                }],
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
                &*FONT_MAP,
                &SectionGeometry::default(),
                &[SectionText {
                    text: "hello world",
                    scale: Scale::uniform(20.0),
                    ..SectionText::default()
                }],
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
                &*FONT_MAP,
                &SectionGeometry::default(),
                &[SectionText {
                    text: "hello\nworld",
                    scale: Scale::uniform(20.0),
                    ..SectionText::default()
                }],
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
            &*FONT_MAP,
            &SectionGeometry {
                bounds: (85.0, f32::INFINITY), // should only be enough room for the 1st word
                ..SectionGeometry::default()
            },
            &[SectionText {
                text: "hello what's _happening_?",
                scale: Scale::uniform(20.0),
                ..SectionText::default()
            }],
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
            &*FONT_MAP,
            &SectionGeometry {
                bounds: (125.0, f32::INFINITY),
                ..SectionGeometry::default()
            },
            &[SectionText {
                text: "hello what's _happening_?",
                scale: Scale::uniform(20.0),
                ..SectionText::default()
            }],
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
                &*FONT_MAP,
                &SectionGeometry {
                    bounds: (50.0, f32::INFINITY),
                    ..SectionGeometry::default()
                },
                &[SectionText {
                    text: "helloworld",
                    scale: Scale::uniform(20.0),
                    ..SectionText::default()
                }],
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
            &*FONT_MAP,
            &SectionGeometry::default(),
            &[SectionText {
                text: test_str,
                scale: Scale::uniform(20.0),
                ..SectionText::default()
            }],
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
            &*FONT_MAP,
            &SectionGeometry {
                bounds: (750.0, f32::INFINITY),
                ..SectionGeometry::default()
            },
            &[
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
            &*FONT_MAP,
            &SectionGeometry::default(),
            &[
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
            &*FONT_MAP,
            &SectionGeometry::default(),
            &[SectionText {
                text: unicode_str,
                scale: Scale::uniform(20.0),
                ..SectionText::default()
            }],
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
            &*FONT_MAP,
            &SectionGeometry {
                bounds: (50.0, ::std::f32::INFINITY),
                ..SectionGeometry::default()
            },
            &[
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
        );

        assert_glyph_order!(glyphs, "Themoonlight");

        let y_ords: HashSet<OrderedFloat<f32>> = glyphs
            .iter()
            .map(|g| OrderedFloat(g.0.position().y))
            .collect();

        assert_eq!(y_ords.len(), 2, "Y ords: {:?}", y_ords);

        let first_line_y = y_ords.iter().min().unwrap();
        let second_line_y = y_ords.iter().max().unwrap();

        assert_relative_eq!(glyphs[0].0.position().y, first_line_y);
        assert_relative_eq!(glyphs[3].0.position().y, second_line_y);
    }

    #[test]
    fn recalculate_identical() {
        let glyphs = Layout::default().calculate_glyphs(
            &*FONT_MAP,
            &SectionGeometry::default(),
            &[SectionText {
                text: "hello world",
                scale: Scale::uniform(20.0),
                ..SectionText::default()
            }],
        );

        let recalc = Layout::default().recalculate_glyphs(
            Cow::Owned(glyphs),
            GlyphChange::Unknown,
            &*FONT_MAP,
            &SectionGeometry::default(),
            &[SectionText {
                text: "hello world",
                scale: Scale::uniform(20.0),
                ..SectionText::default()
            }],
        );

        assert_glyph_order!(recalc, "helloworld");

        assert_relative_eq!(recalc[0].0.position().x, 0.0);
        let last_glyph = &recalc.last().unwrap().0;
        assert!(
            last_glyph.position().x > 0.0,
            "unexpected last position {:?}",
            last_glyph.position()
        );
    }

    #[test]
    fn recalculate_position() {
        let geometry_1 = SectionGeometry {
            screen_position: (0.0, 0.0),
            ..<_>::default()
        };

        let glyphs = Layout::default().calculate_glyphs(
            &*FONT_MAP,
            &geometry_1,
            &[SectionText {
                text: "hello world",
                scale: Scale::uniform(20.0),
                ..SectionText::default()
            }],
        );

        let original_y = glyphs[0].0.position().y;

        let recalc = Layout::default().recalculate_glyphs(
            Cow::Owned(glyphs),
            GlyphChange::Geometry(geometry_1),
            &*FONT_MAP,
            &SectionGeometry {
                screen_position: (0.0, 50.0),
                ..geometry_1
            },
            &[SectionText {
                text: "hello world",
                scale: Scale::uniform(20.0),
                ..SectionText::default()
            }],
        );

        assert_glyph_order!(recalc, "helloworld");

        assert_relative_eq!(recalc[0].0.position().x, 0.0);
        assert_relative_eq!(recalc[0].0.position().y, original_y + 50.0);
        let last_glyph = &recalc.last().unwrap().0;
        assert!(
            last_glyph.position().x > 0.0,
            "unexpected last position {:?}",
            last_glyph.position()
        );
    }

    #[test]
    fn recalculate_colors() {
        const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
        const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

        let glyphs = Layout::default().calculate_glyphs(
            &*FONT_MAP,
            &SectionGeometry::default(),
            &[SectionText {
                text: "hello world",
                color: RED,
                ..<_>::default()
            }],
        );

        assert_glyph_order!(glyphs, "helloworld");
        assert_eq!(glyphs[2].1, RED);

        let recalc = Layout::default().recalculate_glyphs(
            Cow::Owned(glyphs),
            GlyphChange::Color,
            &*FONT_MAP,
            &SectionGeometry::default(),
            &[SectionText {
                text: "hello world",
                color: BLUE,
                ..<_>::default()
            }],
        );

        assert_glyph_order!(recalc, "helloworld");
        assert_eq!(recalc[2].1, BLUE);
    }

    #[test]
    fn recalculate_alpha() {
        const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
        const RED_HALF_ALPHA: [f32; 4] = [1.0, 0.0, 0.0, 0.5];

        const YELLOW: [f32; 4] = [1.0, 1.0, 0.0, 1.0];
        const YELLOW_HALF_ALPHA: [f32; 4] = [1.0, 1.0, 0.0, 0.5];

        let glyphs = Layout::default().calculate_glyphs(
            &*FONT_MAP,
            &SectionGeometry::default(),
            &[
                SectionText {
                    text: "hello",
                    color: RED,
                    ..<_>::default()
                },
                SectionText {
                    text: " world",
                    color: YELLOW,
                    ..<_>::default()
                },
            ],
        );

        assert_glyph_order!(glyphs, "helloworld");
        assert_eq!(glyphs[2].1, RED);

        let recalc = Layout::default().recalculate_glyphs(
            Cow::Owned(glyphs),
            GlyphChange::Alpha,
            &*FONT_MAP,
            &SectionGeometry::default(),
            &[
                SectionText {
                    text: "hello",
                    color: RED_HALF_ALPHA,
                    ..<_>::default()
                },
                SectionText {
                    text: " world",
                    color: YELLOW_HALF_ALPHA,
                    ..<_>::default()
                },
            ],
        );

        assert_glyph_order!(recalc, "helloworld");
        assert_eq!(recalc[2].1, RED_HALF_ALPHA);
    }

    /// Chinese sentance squeezed into a vertical pipe meaning each character is on
    /// a seperate line.
    #[test]
    fn wrap_word_chinese() {
        let glyphs = Layout::default().calculate_glyphs(
            &*FONT_MAP,
            &SectionGeometry {
                bounds: (25.0, f32::INFINITY),
                ..<_>::default()
            },
            &[SectionText {
                text: "提高代碼執行率",
                scale: Scale::uniform(20.0),
                font_id: FontId(1),
                ..<_>::default()
            }],
        );

        assert_glyph_order!(glyphs, "提高代碼執行率", font = CJK_FONT);

        let x_positions: HashSet<_> = glyphs
            .iter()
            .map(|g| OrderedFloat(g.0.position().x))
            .collect();
        assert_eq!(x_positions, std::iter::once(OrderedFloat(0.0)).collect());

        let y_positions: HashSet<_> = glyphs
            .iter()
            .map(|g| OrderedFloat(g.0.position().y))
            .collect();

        assert_eq!(y_positions.len(), 7, "{:?}", y_positions);
    }
}
