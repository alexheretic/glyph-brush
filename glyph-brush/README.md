glyph_brush
[![crates.io](https://img.shields.io/crates/v/glyph_brush.svg)](https://crates.io/crates/glyph_brush)
[![Documentation](https://docs.rs/glyph_brush/badge.svg)](https://docs.rs/glyph_brush)
================
Fast caching text render library using [ab_glyph](https://github.com/alexheretic/ab-glyph). Provides render API agnostic rasterization & draw caching logic.

Makes extensive use of caching to optimise frame performance.

* GPU texture cache logic to dynamically maintain a GPU texture of rendered glyphs.
* Caching of glyph layout output to avoid repeated cost of identical text rendering on sequential frames.
* Layouts are re-used to optimise similar layout calculation after a change.
* Vertex generation is cached per section and re-assembled into the total vertex array on change.
* Avoids any layout or vertex calculations when identical text is rendered on sequential frames.

The crate is designed to be easily wrapped to create a convenient render API specific version, for example [gfx-glyph](https://github.com/alexheretic/gfx-glyph/tree/master/gfx-glyph).

```rust
use glyph_brush::{ab_glyph::FontArc, BrushAction, BrushError, GlyphBrushBuilder, Section, Text};

let dejavu = FontArc::try_from_slice(include_bytes!("../../fonts/DejaVuSans.ttf"))?;
let mut glyph_brush = GlyphBrushBuilder::using_font(dejavu).build();

glyph_brush.queue(Section::default().add_text(Text::new("Hello glyph_brush")));
glyph_brush.queue(some_other_section);

match glyph_brush.process_queued(
    |rect, tex_data| update_texture(rect, tex_data),
    |vertex_data| into_vertex(vertex_data),
) {
    Ok(BrushAction::Draw(vertices)) => {
        // Draw new vertices.
    }
    Ok(BrushAction::ReDraw) => {
        // Re-draw last frame's vertices unmodified.
    }
    Err(BrushError::TextureTooSmall { suggested }) => {
        // Enlarge texture + glyph_brush texture cache and retry.
    }
}
```

## Examples
Have a look at
* `cargo run --example opengl --release`
