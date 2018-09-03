glyph_brush
[![crates.io](https://img.shields.io/crates/v/glyph_brush.svg)](https://crates.io/crates/glyph_brush)
[![Documentation](https://docs.rs/glyph_brush/badge.svg)](https://docs.rs/glyph_brush)
================
Fast cached text render library using [rusttype](https://gitlab.redox-os.org/redox-os/rusttype).

This crate provides render API agnostic rasterization & draw caching logic. Allowing generic vertex generation & re-use of previous frame vertices.

```rust
extern crate glyph_brush;

use glyph_brush::{BrushAction, BrushError, GlyphBrushBuilder, Section};

let dejavu: &[u8] = include_bytes!("../../examples/DejaVuSans.ttf");
let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(dejavu).build();

glyph_brush.queue(Section {
    text: "Hello glyph_brush",
    ..Section::default()
});
glyph_brush.queue(some_other_section);

match glyph_brush.process_queued(
    screen_dimensions,
    |rect, tex_data| update_texture(rect, tex_data),
    |vertex_data| into_vertex(vertex_data),
) {
    Ok(BrushAction::Draw(vertices)) => {
        // Draw new vertices.
    }
    Ok(BrushAction::ReDraw) => {
        // Re-draw last frame's vertices unmodified.
    }
    Err(BrushError::TextureTooSmall { suggested, .. }) => {
        // Enlarge texture + glyph_brush texture cache and retry.
    }
}
```
