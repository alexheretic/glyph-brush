use super::{Color, EolLineBreak, FontId, FontMap, SectionText};
use crate::{
    linebreak::{LineBreak, LineBreaker},
    rusttype::{Scale, ScaledGlyph},
    words::Words,
};
use std::{
    iter::{FusedIterator, Iterator},
    marker::PhantomData,
    mem, slice,
    str::CharIndices,
};

/// Single character info
pub(crate) struct Character<'font> {
    pub glyph: ScaledGlyph<'font>,
    pub color: Color,
    pub font_id: FontId,
    /// Line break proceeding this character.
    pub line_break: Option<LineBreak>,
    /// Equivalent to `char::is_control()`.
    pub control: bool,
}

/// `Character` iterator
pub(crate) struct Characters<'a, 'b, 'font, L, F>
where
    'font: 'a + 'b,
    L: LineBreaker,
    F: FontMap<'font>,
{
    font_map: &'b F,
    section_text: slice::Iter<'a, SectionText<'a>>,
    line_breaker: L,
    part_info: Option<PartInfo<'a>>,
    phantom: PhantomData<&'font ()>,
}

struct PartInfo<'a> {
    section: &'a SectionText<'a>,
    info_chars: CharIndices<'a>,
    line_breaks: Box<dyn Iterator<Item = LineBreak> + 'a>,
    next_break: Option<LineBreak>,
}

impl<'a, 'b, 'font, L, F> Characters<'a, 'b, 'font, L, F>
where
    L: LineBreaker,
    F: FontMap<'font>,
{
    /// Returns a new `Characters` iterator.
    pub(crate) fn new(
        font_map: &'b F,
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
    pub(crate) fn words(self) -> Words<'a, 'b, 'font, L, F> {
        Words { characters: self }
    }
}

impl<'font, L, F> Iterator for Characters<'_, '_, 'font, L, F>
where
    L: LineBreaker,
    F: FontMap<'font>,
{
    type Item = Character<'font>;

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

                let glyph = self.font_map.font(*font_id).glyph(c).scaled(*scale);

                let c_len = c.len_utf8();
                let mut line_break = next_break.filter(|b| b.offset() == byte_index + c_len);
                if line_break.is_some() && byte_index + c_len == text.len() {
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

impl<'font, L, F> FusedIterator for Characters<'_, '_, 'font, L, F>
where
    L: LineBreaker,
    F: FontMap<'font>,
{
}

#[inline]
fn valid_section(s: &SectionText<'_>) -> bool {
    let Scale { x, y } = s.scale;
    x > 0.0 && y > 0.0
}
