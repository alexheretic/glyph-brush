use super::*;
use std::iter::{Skip, Enumerate};
use std::slice::Iter;
use std::str::Chars;
use unicode_normalization::*;

#[derive(Debug, Clone)]
pub struct SectionGlyphInfo<'a> {
    /// Position on screen to render text, in pixels from top-left.
    pub screen_position: (f32, f32),
    /// Max (width, height) bounds, in pixels from top-left.
    pub bounds: (f32, f32),

    pub text: Vec<GlyphInfo<'a>>,
    pub text_index: usize,
}

impl<'a, 'b> From<&'b VariedSection<'a>> for SectionGlyphInfo<'a> {
    fn from(section: &'b VariedSection<'a>) -> Self {
        let VariedSection { screen_position, bounds, ref text, .. } = *section;
        Self {
            screen_position,
            bounds,
            text: text.iter().map(|t| GlyphInfo::from(*t)).collect(),
            text_index: 0,
        }
    }
}

impl<'a> SectionGlyphInfo<'a> {
    /// Returns a clone info that has marked text up to `index` as processed, and
    /// `GlyphInfo` at index as input `info`
    pub fn with_info(&self, index: usize, info: GlyphInfo<'a>) -> Self {
        let mut section = self.clone();
        section.text[index] = info;
        section.text_index = index;
        section
    }

    pub fn remaining_info(&self) -> Skip<Enumerate<Iter<GlyphInfo<'a>>>> {
        self.text.iter().enumerate().skip(self.text_index)
    }
}

/// A specialised view on a [`Section`](struct.Section.html) for the purposes of calculating
/// glyph positions. Used by a [`GlyphPositioner`](trait.GlyphPositioner.html).
///
/// See [`Layout`](enum.Layout.html) for built-in positioner logic.
#[derive(Debug, Clone, Copy)]
pub struct GlyphInfo<'a> {
    /// Section text, use [`remaining_chars()`](struct.GlyphInfo.html#method.remaining_chars) instead in order
    /// to respect skip settings, ie in leftover payloads.
    pub text: &'a str,
    skip: usize,
    pub scale: Scale,
    pub color: Color,
    pub font_id: FontId,
}

impl<'a> GlyphInfo<'a> {
    /// Returns a unicode normalized char iterator, that respects the skipped chars
    /// that have already been already processed
    pub fn remaining_chars(&self) -> Skip<Recompositions<Chars<'a>>> {
        self.text.nfc().skip(self.skip)
    }

    /// Returns a new GlyphInfo instance whose
    /// [`remaining_chars()`](struct.GlyphInfo.html#method.remaining_chars)
    /// method will skip additional chars.
    pub fn skip(&self, skip: usize) -> GlyphInfo<'a> {
        let mut clone = *self;
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

impl<'a> From<SectionText<'a>> for GlyphInfo<'a> {
    fn from(section: SectionText<'a>) -> Self {
        let SectionText { text, scale, color, font_id, .. } = section;
        Self {
            text,
            scale,
            skip: 0,
            color,
            font_id,
        }
    }
}

/// Logic to calculate glyph positioning based on [`Font`](type.Font.html) and
/// [`GlyphInfo`](struct.GlyphInfo.html)
pub trait GlyphPositioner: Hash {
    /// Calculate a sequence of positioned glyphs to render. Custom implementations should always
    /// return the same result when called with the same arguments. If not consider disabling
    /// [`cache_glyph_positioning`](struct.GlyphBrushBuilder.html#method.cache_glyph_positioning).
    fn calculate_glyphs<'a, G>(&self, font: &HashMap<FontId, Font>, section: G)
        -> Vec<GlyphedSectionText>
        where G: Into<SectionGlyphInfo<'a>>;
    /// Return a rectangle according to the requested render position and bounds appropriate
    /// for the glyph layout.
    fn bounds_rect<'a, G>(&self, section: G) -> Rect<f32> where G: Into<SectionGlyphInfo<'a>>;
}



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
            SingleLine { line_breaker, v_align, .. } => {
                SingleLine { line_breaker, v_align, h_align }
            }
            Wrap { line_breaker, v_align, .. } => Wrap { line_breaker, v_align, h_align },
        }
    }

    /// Returns an identical `Layout` but with the input `v_align`
    pub fn v_align(self, v_align: VerticalAlign) -> Self {
        match v_align {
            VerticalAlign::Top => self
        }
    }

    /// Returns an identical `Layout` but with the input `line_breaker`
    pub fn line_breaker<L2: LineBreaker>(self, line_breaker: L2) -> Layout<L2> {
        use Layout::*;
        match self {
            SingleLine { h_align, v_align, .. } => SingleLine { line_breaker, v_align, h_align },
            Wrap { h_align, v_align, .. } => Wrap { line_breaker, v_align, h_align },
        }
    }
}

impl<L: LineBreaker> GlyphPositioner for Layout<L> {
    fn calculate_glyphs<'a, G: Into<SectionGlyphInfo<'a>>>(
        &self,
        font_map: &HashMap<FontId, Font>,
        section: G,
    ) -> Vec<GlyphedSectionText> {
        self.calculate_glyphs_and_leftover(font_map, &section.into()).0
    }

    fn bounds_rect<'a, G: Into<SectionGlyphInfo<'a>>>(&self, section: G) -> Rect<f32> {
        let SectionGlyphInfo {
            screen_position: (screen_x, screen_y),
            bounds: (bound_w, bound_h),
            ..
        } = section.into();
        match *self {
            Layout::SingleLine { h_align: HorizontalAlign::Left, .. } |
            Layout::Wrap { h_align: HorizontalAlign::Left, .. } => Rect {
                min: Point { x: screen_x, y: screen_y },
                max: Point { x: screen_x + bound_w, y: screen_y + bound_h },
            },
            Layout::SingleLine { h_align: HorizontalAlign::Center, .. } |
            Layout::Wrap { h_align: HorizontalAlign::Center, .. } => Rect {
                min: Point { x: screen_x - bound_w / 2.0, y: screen_y },
                max: Point { x: screen_x + bound_w / 2.0, y: screen_y + bound_h },
            },
            Layout::SingleLine { h_align: HorizontalAlign::Right, .. } |
            Layout::Wrap { h_align: HorizontalAlign::Right, .. } => Rect {
                min: Point { x: screen_x - bound_w, y: screen_y },
                max: Point { x: screen_x, y: screen_y + bound_h },
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

/// Container for glyphs leftover/unable to fit in a layout and/or within render bounds
#[derive(Debug, Clone)]
pub enum LayoutLeftover<'a> {
    /// leftover text after a new hard line break, indicated by the
    /// [`LineBreaker`](trait.LineBreaker.html)
    HardBreak(
        Point<f32>,
        SectionGlyphInfo<'a>,
        // line max v-metrics
        VMetrics,
    ),
    /// text that would fall outside of the horizontal bound
    OutOfWidthBound(
        Point<f32>,
        SectionGlyphInfo<'a>,
        // line max v-metrics
        VMetrics,
    ),
    /// text that would fall fully outside the vertical bound
    /// note: does not include hidden text within a layout
    /// for example a `_` character hidden but between visible characters would be ignored
    OutOfHeightBound(
        Point<f32>,
        SectionGlyphInfo<'a>,
        /// line max v-metrics
        VMetrics,
    ),
}

impl<L: LineBreaker> Layout<L> {
    pub fn calculate_glyphs_and_leftover<'a>(
        &self,
        font_map: &HashMap<FontId, Font>,
        section: &SectionGlyphInfo<'a>,
    ) -> (Vec<GlyphedSectionText>, Option<LayoutLeftover<'a>>) {
        match *self {
            Layout::SingleLine { line_breaker, h_align, v_align } => {
                single_line(font_map, line_breaker, h_align, v_align, section)
            }
            Layout::Wrap { line_breaker, h_align, v_align }  => {
                paragraph(font_map, line_breaker, h_align, v_align, section.clone())
            }
        }
    }
}

/// Positions glyphs in a single line left to right with the screen position marking
/// the top-left corner.
/// Returns (positioned-glyphs, text that could not be positioned (outside bounds))
///
/// TODO this is the guts of the layout code, it should be split up more as it's fairly unweildy now
fn single_line<'a, L: LineBreaker>(
    font_map: &HashMap<FontId, Font>,
    line_breaker: L,
    h_align: HorizontalAlign,
    v_align: VerticalAlign,
    section_glyph_info: &SectionGlyphInfo<'a>,
) -> (Vec<GlyphedSectionText>, Option<LayoutLeftover<'a>>) {

    match v_align {
        VerticalAlign::Top => {}
    };

    let SectionGlyphInfo {
        screen_position: (screen_x, screen_y),
        bounds: (bound_w, bound_h),
        .. } = *section_glyph_info;

    let mut result: Vec<GlyphedSectionText> = Vec::new();
    let mut leftover = None;

    let mut caret = point(0.0, 0.0);
    let mut caret_initialized = false;

    let mut vertically_hidden_tail_start = None;
    let mut max_line_v: Option<VMetrics> = None;
    let mut ascent_adjustment = None;

    macro_rules! shift_previous_ascent_by {
        ($ascent_adjustment:expr) => {
            if let Some(adjustment) = $ascent_adjustment.take() {
                // adjust all preview glyphs down to the new max ascent
                for part in &mut result {
                    let mut adjusted = Vec::with_capacity(part.0.len());
                    for glyph in part.0.drain(..) {
                        let mut pos = glyph.position();
                        pos.y += adjustment;
                        adjusted.push(glyph.into_unpositioned().positioned(pos));
                    }
                    part.0 = adjusted;
                }
                true
            }
            else { false }
        }
    };

    'sections: for (info_index, glyph_info) in section_glyph_info.remaining_info() {
        let GlyphInfo { scale, color, font_id, .. } = *glyph_info;
        let font = &font_map[&font_id];

        let mut v_metrics = font.v_metrics(scale);
        if let Some(max) = max_line_v {
            if max.ascent < v_metrics.ascent {
                ascent_adjustment = Some(v_metrics.ascent - max.ascent);
            }
            else {
                v_metrics = max_line_v.unwrap();
            }
        }
        else {
            max_line_v = Some(v_metrics);
        }

        if !caret_initialized {
            caret = point(screen_x, screen_y + v_metrics.ascent);
            caret_initialized = true;
        }

        let mut last_glyph_id = None;

        let mut line_breaks = line_breaker.line_breaks(glyph_info);
        let mut previous_break = None;
        let mut next_break = line_breaks.next();
        let mut glyphs = vec![];

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
                    leftover = Some(LayoutLeftover::HardBreak(
                        caret,
                        section_glyph_info.with_info(info_index, glyph_info.skip(index)),
                        v_metrics,
                    ));
                    if !glyphs.is_empty() {
                        shift_previous_ascent_by!(ascent_adjustment);
                        result.push(GlyphedSectionText(glyphs, color, font_id));
                    }
                    break 'sections;
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

            // ensure correct ascent
            caret.y = screen_y + v_metrics.ascent;

            let glyph = base_glyph.scaled(scale).positioned(caret);
            if let Some(bb) = glyph.pixel_bounding_box() {
                if bb.max.x as f32 > (screen_x + bound_w) {
                    if let Some(line_break) = next_break {
                        if line_break.offset() == index {
                            // current char is a breaking char
                            leftover = Some(LayoutLeftover::OutOfWidthBound(
                                caret,
                                section_glyph_info.with_info(info_index, glyph_info.skip(index)),
                                if glyphs.is_empty() { max_line_v.unwrap() } else { v_metrics },
                            ));
                            if !glyphs.is_empty() {
                                shift_previous_ascent_by!(ascent_adjustment);
                                result.push(GlyphedSectionText(glyphs, color, font_id));
                            }
                            break 'sections;
                        }
                    }

                    if let Some(line_break) = previous_break {
                        while glyphs.len() > line_break.offset() {
                            // TODO glyph count / char count mismatch test
                            glyphs.pop();
                        }
                        leftover = Some(LayoutLeftover::OutOfWidthBound(
                            caret,
                            section_glyph_info.with_info(
                                info_index,
                                glyph_info.skip(line_break.offset())
                            ),
                            if glyphs.is_empty() { max_line_v.unwrap() } else { v_metrics },
                        ));
                    }
                    else {
                        // there has been no separator
                        glyphs.clear();
                        leftover = Some(LayoutLeftover::OutOfWidthBound(
                            caret,
                            section_glyph_info.with_info(info_index, *glyph_info),
                            max_line_v.unwrap(),
                        ));
                    }
                    if !glyphs.is_empty() {
                        shift_previous_ascent_by!(ascent_adjustment);
                        result.push(GlyphedSectionText(glyphs, color, font_id));
                    }
                    break 'sections;
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
            glyphs.push(glyph.standalone());
        }

        if !glyphs.is_empty() {
            if shift_previous_ascent_by!(ascent_adjustment) {
                max_line_v = Some(v_metrics);
            }
            result.push(GlyphedSectionText(glyphs, color, font_id));
        }

        if let Some(idx) = vertically_hidden_tail_start {
            // if entire tail was vertically hidden then return as unrendered text
            // otherwise ignore
            leftover = Some(LayoutLeftover::OutOfHeightBound(
                caret,
                section_glyph_info.with_info(info_index, glyph_info.skip(idx)),
                max_line_v.unwrap(),
            ));
            break 'sections;
        }
    }

    adjust_for_alignment(&mut result, h_align, section_glyph_info);

    (result, leftover)
}

fn adjust_for_alignment(
    line: &mut Vec<GlyphedSectionText>,
    h_align: HorizontalAlign,
    &SectionGlyphInfo { screen_position: (screen_x, ..), .. }: &SectionGlyphInfo,
) {
    if !line.is_empty() {
        match h_align {
            HorizontalAlign::Left => (), // all done
            HorizontalAlign::Right | HorizontalAlign::Center => {
                // Right alignment attained from left by shifting the line
                // leftwards by the rightmost x distance from render position
                // Central alignment is attained from left by shifting the line
                // leftwards by half the rightmost x distance from render position
                let rightmost_x_offset = {
                    let last = line.last().and_then(|s| s.0.last()).unwrap();
                    last.pixel_bounding_box()
                        .map(|bb| bb.max.x as f32)
                        .unwrap_or_else(|| last.position().x)
                        + last.unpositioned().h_metrics().left_side_bearing
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

                for part in line.iter_mut() {
                    let mut shifted = Vec::with_capacity(part.0.len());

                    for glyph in part.0.drain(..) {
                        let Point { x, y } = glyph.position();
                        let x = x - shift_left;
                        shifted.push(glyph.into_unpositioned().positioned(Point { x, y }));
                    }

                    part.0 = shifted;
                }
            }
        }
    }
}


fn paragraph<'a, L: LineBreaker>(
    font_map: &HashMap<FontId, Font>,
    line_breaker: L,
    h_align: HorizontalAlign,
    v_align: VerticalAlign,
    mut section: SectionGlyphInfo<'a>)
    -> (Vec<GlyphedSectionText>, Option<LayoutLeftover<'a>>)
{
    let mut out = vec![];
    let mut paragraph_leftover = None;

    loop {
        let (glyphs, mut leftover) = Layout::SingleLine { line_breaker, h_align, v_align }
            .calculate_glyphs_and_leftover(font_map, &section);

        out.extend_from_slice(&glyphs);
        if leftover.is_none() { break; }

        paragraph_leftover = match leftover.take().unwrap() {
            LayoutLeftover::HardBreak(p, remaining_glyphs, v_metrics) => {
                let advance_height = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;
                if remaining_glyphs.bounds.1 - advance_height < 0.0 {
                    Some(LayoutLeftover::OutOfHeightBound(p, remaining_glyphs, v_metrics))
                }
                else {
                    section = remaining_glyphs;
                    section.screen_position.1 += advance_height;
                    section.bounds.1 -= advance_height;
                    None
                }
            },
            LayoutLeftover::OutOfWidthBound(p, remaining_glyphs, v_metrics) => {
                let advance_height = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;
                // use the next line when we run out of width
                if remaining_glyphs.bounds.1 - advance_height < 0.0 {
                    Some(LayoutLeftover::OutOfHeightBound(p, remaining_glyphs, v_metrics))
                }
                else {
                    section = remaining_glyphs;
                    section.screen_position.1 += advance_height;
                    section.bounds.1 -= advance_height;
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
    use BuiltInLineBreaker::*;

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
        }};
    }

    macro_rules! merged_glyphs_and_leftover {
        ($layout:expr, $section:expr) => {{
            let _ = ::pretty_env_logger::init();

            let mut font_map = HashMap::new();
            font_map.insert(FontId(0), A_FONT.clone());

            let (all_glyphs, leftover) = $layout
                .calculate_glyphs_and_leftover(
                    &font_map,
                    &SectionGlyphInfo::from(&$section.into())
                );

            let glyphs: Vec<_> = all_glyphs.into_iter()
                .flat_map(|s| s.0.into_iter())
                .collect();

            (glyphs, leftover)
        }};
    }

    #[test]
    fn single_line_chars_left_simple() {
        let (glyphs, leftover) = merged_glyphs_and_leftover!(
            Layout::default_single_line()
                .line_breaker(AnyCharLineBreaker),
            Section {
                text: "hello world",
                scale: Scale::uniform(20.0),
                ..Section::default()
            }
        );

        assert!(leftover.is_none());
        assert_glyph_order!(glyphs, "hello world");

        assert_eq!(glyphs[0].position().x, 0.0);
        assert!(glyphs[10].position().x > 0.0,
            "unexpected last position {:?}", glyphs[10].position());
    }

    #[test]
    fn single_line_chars_right() {
        let (glyphs, leftover) = merged_glyphs_and_leftover!(
            Layout::default_single_line()
                .line_breaker(AnyCharLineBreaker)
                .h_align(HorizontalAlign::Right),
            Section {
                text: "hello world",
                scale: Scale::uniform(20.0),
                ..Section::default()
            }
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
        let (glyphs, leftover) = merged_glyphs_and_leftover!(
            Layout::default_single_line()
                .line_breaker(AnyCharLineBreaker)
                .h_align(HorizontalAlign::Center),
            Section {
                text: "hello world",
                scale: Scale::uniform(20.0),
                ..Section::default()
            }
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

    fn remaining_text(section: &SectionGlyphInfo) -> String {
        let remaining_pieces: Vec<_> = section.remaining_info()
            .map(|info| info.1.remaining_chars().collect::<String>())
            .collect();

        remaining_pieces.join("")
    }

    #[test]
    fn single_line_chars_left_finish_at_newline() {
        let (glyphs, leftover) = merged_glyphs_and_leftover!(
            Layout::default_single_line().line_breaker(AnyCharLineBreaker),
            Section {
                text: "hello\nworld",
                scale: Scale::uniform(20.0),
                ..Section::default()
            }
        );

        if let Some(LayoutLeftover::HardBreak(_, leftover, ..)) = leftover {
            assert_eq!(remaining_text(&leftover), "world");
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
    fn wrap_word_left() {
        let (glyphs, leftover) = merged_glyphs_and_leftover!(
            Layout::default_single_line(),
            Section {
                text: "hello what's _happening_?",
                scale: Scale::uniform(20.0),
                bounds: (85.0, f32::INFINITY), // should only be enough room for the 1st word
                ..Section::default()
            }
        );

        if let Some(LayoutLeftover::OutOfWidthBound(_, leftover, ..)) = leftover {
            assert_eq!(remaining_text(&leftover), "what's _happening_?");
        }
        else {
            assert!(false, "Unexpected leftover {:?}", leftover);
        }

        assert_glyph_order!(glyphs, "hello ");
        assert_eq!(glyphs[0].position().x, 0.0);
        assert!(glyphs[5].position().x > 0.0,
            "unexpected last position {:?}", glyphs[5].position());


        let (glyphs, leftover) = merged_glyphs_and_leftover!(
            Layout::default_single_line(),
            Section {
                text: "hello what's _happening_?",
                scale: Scale::uniform(20.0),
                bounds: (125.0, f32::INFINITY), // should only be enough room for the 1,2 words
                ..Section::default()
            }
        );

        if let Some(LayoutLeftover::OutOfWidthBound(_, leftover, ..)) = leftover {
            assert_eq!(remaining_text(&leftover), "_happening_?");
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
    fn single_line_little_verticle_room() {
        let (glyphs, leftover) = merged_glyphs_and_leftover!(
            Layout::default_single_line().line_breaker(AnyCharLineBreaker),
            Section {
                text: "hello world",
                scale: Scale::uniform(20.0),
                bounds: (f32::INFINITY, 5.0),
                ..Section::default()
            }
        );

        assert!(leftover.is_none(), "unexpected leftover {:?}", leftover);
        assert_glyph_order!(glyphs, "hll ld"); // e,o,w,o,r hidden

        // letter `l` should be in the same place as when all the word is visible
        let (all_glyphs, _) = merged_glyphs_and_leftover!(
            Layout::default_single_line().line_breaker(AnyCharLineBreaker),
            Section {
                text: "hello world",
                scale: Scale::uniform(20.0),
                ..Section::default()
            }
        );
        assert_eq!(all_glyphs[9].id(), A_FONT.glyph('l').unwrap().id());
        assert_relative_eq!(all_glyphs[9].position().x, glyphs[4].position().x);
        assert_relative_eq!(all_glyphs[9].position().y, glyphs[4].position().y);
    }

    #[test]
    fn single_line_little_verticle_room_tail_lost() {
        let (glyphs, leftover) = merged_glyphs_and_leftover!(
            Layout::default_single_line().line_breaker(AnyCharLineBreaker),
            Section {
                text: "hellowor__",
                scale: Scale::uniform(20.0),
                bounds: (f32::INFINITY, 5.0),
                ..Section::default()
            }
        );

        if let Some(LayoutLeftover::OutOfHeightBound(_, leftover, ..)) = leftover {
            assert_eq!(remaining_text(&leftover), "owor__");
        }
        else {
            assert!(false, "Unexpected leftover {:?}", leftover);
        }

        assert_glyph_order!(glyphs, "hll"); // e hidden
    }

    #[test]
    fn single_line_limited_horizontal_room() {
        let (glyphs, leftover) = merged_glyphs_and_leftover!(
            Layout::default_single_line().line_breaker(AnyCharLineBreaker),
            Section {
                text: "hello world",
                scale: Scale::uniform(20.0),
                bounds: (50.0, f32::INFINITY),
                ..Section::default()
            }
        );

        if let Some(LayoutLeftover::OutOfWidthBound(_, leftover, ..)) = leftover {
            assert_eq!(remaining_text(&leftover), "o world");
        }
        else {
            assert!(false, "Unexpected leftover {:?}", leftover);
        }

        assert_glyph_order!(glyphs, "hell");
        assert_eq!(glyphs[0].position().x, 0.0);
    }

    #[test]
    fn wrap_layout_with_new_lines() {
        let test_str = "Autumn moonlight\n\
            a worm digs silently\n\
            into the chestnut.";

        let (glyphs, leftover) = merged_glyphs_and_leftover!(
            Layout::default_wrap(),
            Section {
                text: test_str,
                scale: Scale::uniform(20.0),
                ..Section::default()
            }
        );

        assert!(leftover.is_none(), "Unexpected leftover {:?}", leftover);
        assert_glyph_order!(glyphs, "Autumn moonlighta worm digs silentlyinto the chestnut.");
        assert!(glyphs[16].position().y > glyphs[0].position().y,
            "second line should be lower than first");
        assert!(glyphs[36].position().y > glyphs[16].position().y,
            "third line should be lower than second");
    }

    #[test]
    fn leftover_max_vmetrics() {
        let (glyphs, leftover) = merged_glyphs_and_leftover!(
            Layout::default_single_line(),
            VariedSection {
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
            }
        );

        let max_v_metrics = A_FONT.v_metrics(Scale::uniform(40.0));

        if let Some(LayoutLeftover::OutOfWidthBound(_, leftover, line_v_metrics)) = leftover {
            assert_eq!(remaining_text(&leftover), "the chestnut.");
            assert_eq!(line_v_metrics, max_v_metrics, "unexpected line v_metrics");
        }
        else {
            assert!(false, "Unexpected leftover {:?}", leftover);
        }

        for g in glyphs {
            println!("{:?}", (g.scale(), g.position()));
            // all glyphs should have the same ascent drawing position
            let y_pos = g.position().y;
            assert_relative_eq!(y_pos, max_v_metrics.ascent);
        }
    }
}
