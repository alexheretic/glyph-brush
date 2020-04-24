use super::{Color, EolLineBreak, FontId, FontMap, SectionText};
use crate::{
    linebreak::{LineBreak, LineBreaker},
    words::Words,
};
use ab_glyph::*;
use std::{
    iter::{FusedIterator, Iterator},
    marker::PhantomData,
    mem, slice,
    str::CharIndices,
};

/// Single character info
pub(crate) struct Character<'b, F: Font> {
    pub glyph: Glyph,
    pub scale_font: PxScaleFont<&'b F>,

    pub color: Color,
    pub font_id: FontId,
    /// Line break proceeding this character.
    pub line_break: Option<LineBreak>,
    /// Equivalent to `char::is_control()`.
    pub control: bool,
    /// Equivalent to `char::is_whitespace()`.
    pub whitespace: bool,
}

/// `Character` iterator
pub(crate) struct Characters<'a, 'b, L, F, FM>
where
    F: 'a + 'b,
    L: LineBreaker,
    F: Font,
    FM: FontMap<F>,
{
    font_map: &'b FM,
    section_text: slice::Iter<'a, SectionText<'a>>,
    line_breaker: L,
    part_info: Option<PartInfo<'a>>,
    phantom: PhantomData<F>,
}

struct PartInfo<'a> {
    section: &'a SectionText<'a>,
    info_chars: CharIndices<'a>,
    line_breaks: Box<dyn Iterator<Item = LineBreak> + 'a>,
    next_break: Option<LineBreak>,
}

impl<'a, 'b, L, F, FM> Characters<'a, 'b, L, F, FM>
where
    L: LineBreaker,
    F: Font,
    FM: FontMap<F>,
{
    /// Returns a new `Characters` iterator.
    pub(crate) fn new(
        font_map: &'b FM,
        section_text: slice::Iter<'a, SectionText<'a>>,
        line_breaker: L,
    ) -> Self {
        Self {
            font_map,
            section_text,
            line_breaker,
            part_info: None,
            phantom: PhantomData,
        }
    }

    /// Wraps into a `Words` iterator.
    pub(crate) fn words(self) -> Words<'a, 'b, L, F, FM> {
        Words { characters: self }
    }
}

impl<'b, L, F, FM> Iterator for Characters<'_, 'b, L, F, FM>
where
    L: LineBreaker,
    F: Font,
    FM: FontMap<F>,
{
    type Item = Character<'b, F>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.part_info.is_none() {
            let mut section;
            loop {
                section = self.section_text.next()?;
                if valid_section(&section) {
                    break;
                }
            }
            let line_breaks = self.line_breaker.line_breaks(section.text);
            self.part_info = Some(PartInfo {
                section,
                info_chars: section.text.char_indices(),
                line_breaks,
                next_break: None,
            });
        }

        {
            let PartInfo {
                section:
                    SectionText {
                        scale,
                        color,
                        font_id,
                        text,
                    },
                info_chars,
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

                let scale_font: PxScaleFont<&'b F> = self.font_map.font(*font_id).as_scaled(*scale);

                let glyph = scale_font.scaled_glyph(c);

                let c_len = c.len_utf8();
                let mut line_break = next_break.filter(|b| b.offset() == byte_index + c_len);
                if line_break.is_some() && byte_index + c_len == text.len() {
                    // handle inherent end-of-str hard breaks
                    line_break = line_break.and(c.eol_line_break(&self.line_breaker));
                }

                return Some(Character {
                    glyph,
                    scale_font,
                    color: *color,
                    font_id: *font_id,
                    line_break,
                    control: c.is_control(),
                    whitespace: c.is_whitespace(),
                });
            }
        }

        self.part_info = None;
        self.next()
    }
}

impl<'font, L, F, FM> FusedIterator for Characters<'_, '_, L, F, FM>
where
    L: LineBreaker,
    F: Font,
    FM: FontMap<F>,
{
}

#[inline]
fn valid_section(s: &SectionText<'_>) -> bool {
    let PxScale { x, y } = s.scale;
    x > 0.0 && y > 0.0
}
