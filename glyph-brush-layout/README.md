glyph_brush_layout
[![crates.io](https://img.shields.io/crates/v/glyph_brush_layout.svg)](https://crates.io/crates/glyph_brush_layout)
[![Documentation](https://docs.rs/glyph_brush_layout/badge.svg)](https://docs.rs/glyph_brush_layout)
================
Text layout for [rusttype](https://gitlab.redox-os.org/redox-os/rusttype).

* Generic positioning & linebreaking traits.
* Built-in layout logic:
  - Mixed font, scale, color sections in a single layout.
  - Horizontal align left/center/right.
  - Vertical align top/center/bottom.
  - Unicode line breaking.
  - Bounded layouts.

```rust
use glyph_brush_layout::*;

let dejavu = Font::from_bytes(&include_bytes!("DejaVuSans.ttf")[..])?;
let garamond = Font::from_bytes(&include_bytes!("GaramondNo8-Reg.ttf")[..])?;

// Simple vec font mapping: FontId(0) -> deja vu sans, FontId(1) -> garamond
let fonts = vec![dejavu, garamond];

// Layout "hello glyph_brush_layout" on an unbounded line with the second
// word suitably bigger, greener and serif-ier.
let glyphs = Layout::default().calculate_glyphs(
    &fonts,
    &SectionGeometry {
        screen_position: (150.0, 50.0),
        ..SectionGeometry::default()
    },
    &[
        SectionText {
            text: "hello ",
            scale: Scale::uniform(20.0),
            ..SectionText::default()
        },
        SectionText {
            text: "glyph_brush_layout",
            scale: Scale::uniform(25.0),
            font_id: FontId(1),
            color: [0.0, 1.0, 0.0, 1.0],
        },
    ],
);

assert_eq!(glyphs.len(), 23);

let (o_glyph, glyph_4_color, glyph_4_font) = &glyphs[4];
assert_eq!(o_glyph.id(), fonts[0].glyph('o').id());
assert_eq!(*glyph_4_color, [0.0, 0.0, 0.0, 1.0]);
assert_eq!(*glyph_4_font, FontId(0));

let (s_glyph, glyph_14_color, glyph_14_font) = &glyphs[14];
assert_eq!(s_glyph.id(), fonts[1].glyph('s').id());
assert_eq!(*glyph_14_color, [0.0, 1.0, 0.0, 1.0]);
assert_eq!(*glyph_14_font, FontId(1));
```
