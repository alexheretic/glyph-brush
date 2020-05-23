glyph_brush_layout
[![crates.io](https://img.shields.io/crates/v/glyph_brush_layout.svg)](https://crates.io/crates/glyph_brush_layout)
[![Documentation](https://docs.rs/glyph_brush_layout/badge.svg)](https://docs.rs/glyph_brush_layout)
==================
Text layout for [ab_glyph](https://github.com/alexheretic/ab-glyph).

* Generic positioning & linebreaking traits.
* Built-in layout logic:
  - Mixed font & scale sections in a single layout.
  - Horizontal align left/center/right.
  - Vertical align top/center/bottom.
  - Unicode line breaking.
  - Bounded layouts.

```rust
use glyph_brush_layout::{ab_glyph::*, *};

let dejavu = FontRef::try_from_slice(include_bytes!("../../fonts/DejaVuSans.ttf"))?;
let garamond = FontRef::try_from_slice(include_bytes!("../../fonts/GaramondNo8-Reg.ttf"))?;

// Simple font mapping: FontId(0) -> deja vu sans, FontId(1) -> garamond
let fonts = &[dejavu, garamond];

// Layout "hello glyph_brush_layout" on an unbounded line with the second
// word suitably bigger, greener and serif-ier.
let glyphs = Layout::default().calculate_glyphs(
    fonts,
    &SectionGeometry {
        screen_position: (150.0, 50.0),
        ..SectionGeometry::default()
    },
    &[
        SectionText {
            text: "hello ",
            scale: PxScale::from(20.0),
            font_id: FontId(0),
        },
        SectionText {
            text: "glyph_brush_layout",
            scale: PxScale::from(25.0),
            font_id: FontId(1),
        },
    ],
);

assert_eq!(glyphs.len(), 24);

let SectionGlyph { glyph, font_id, section_index, byte_index } = &glyphs[4];
assert_eq!(glyph.id, fonts[0].glyph_id('o'));
assert_eq!(*font_id, FontId(0));
assert_eq!(*section_index, 0);
assert_eq!(*byte_index, 4);

let SectionGlyph { glyph, font_id, section_index, byte_index } = &glyphs[14];
assert_eq!(glyph.id, fonts[1].glyph_id('u'));
assert_eq!(*font_id, FontId(1));
assert_eq!(*section_index, 1);
assert_eq!(*byte_index, 8);
```
