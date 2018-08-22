use super::*;
use layout::words::Words;
use std::iter::{FusedIterator, Iterator};
use std::{mem, str::CharIndices};

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
pub(crate) struct Characters<'a, 'b, 'font: 'a + 'b, L: LineBreaker> {
    font_map: &'b FontMap<'font>,
    section_text: Iter<'a, SectionText<'a>>,
    line_breaker: L,
    part_info: Option<PartInfo<'a>>,
}

struct PartInfo<'a> {
    section: &'a SectionText<'a>,
    info_chars: CharIndices<'a>,
    line_breaks: Box<Iterator<Item = LineBreak> + 'a>,
    next_break: Option<LineBreak>,
}

impl<'a, 'b, 'font, L: LineBreaker> Characters<'a, 'b, 'font, L> {
    /// Returns a new `Characters` iterator.
    pub(crate) fn new(
        font_map: &'b FontMap<'font>,
        section_text: Iter<'a, SectionText<'a>>,
        line_breaker: L,
    ) -> Self {
        Self {
            font_map,
            section_text,
            line_breaker,

            part_info: None,
        }
    }

    /// Wraps into a `Words` iterator.
    pub(crate) fn words(self) -> Words<'a, 'b, 'font, L> {
        Words { characters: self }
    }
}

impl<'a, 'b, 'font, L: LineBreaker> Iterator for Characters<'a, 'b, 'font, L> {
    type Item = Character<'font>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.part_info.is_none() {
            let section = self.section_text.next()?;
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

                let glyph = self.font_map[font_id].glyph(c).scaled(*scale);

                let mut line_break = next_break.filter(|b| b.offset() == byte_index + 1);
                if line_break.is_some() && byte_index + 1 == text.len() {
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

impl<'a, 'b, 'font, L: LineBreaker> FusedIterator for Characters<'a, 'b, 'font, L> {}
