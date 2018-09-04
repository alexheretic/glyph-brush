gfx_glyph
[![crates.io](https://img.shields.io/crates/v/gfx_glyph.svg)](https://crates.io/crates/gfx_glyph)
[![Documentation](https://docs.rs/gfx_glyph/badge.svg)](https://docs.rs/gfx_glyph)
================

Fast GPU cached text rendering using [gfx-rs](https://github.com/gfx-rs/gfx/tree/pre-ll) & [glyph-brush](../glyph-brush).

Makes use of three kinds of caching to optimise frame performance.

* Caching of glyph positioning output to avoid repeated cost of identical text
rendering on sequential frames.
* Caches draw calculations to avoid repeated cost of identical text rendering on
sequential frames.
* GPU cache logic to dynamically maintain a GPU texture of rendered glyphs.

```rust
extern crate gfx_glyph;
use gfx_glyph::{Section, GlyphBrushBuilder};

let garamond: &[u8] = include_bytes!("GaramondNo8-Reg.ttf");
let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(garamond)
    .build(gfx_factory.clone());

let section = Section {
    text: "Hello gfx_glyph",
    ..Section::default() // color, position, etc
};

glyph_brush.queue(section);
glyph_brush.queue(some_other_section);

glyph_brush.draw_queued(&mut gfx_encoder, &gfx_color, &gfx_depth)?;
```

## Examples
Have a look at
* `cargo run --example paragraph --release`
* `cargo run --example performance --release`
* `cargo run --example varied --release`
* `cargo run --example depth --release`


## Limitations
The current implementation supports OpenGL *(3.2 or later)*. In future we'll support the upcoming gfx-rs ll releases.
