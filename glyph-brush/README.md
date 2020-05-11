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
//TODO
```

## Examples
Have a look at
* `cargo run --example opengl --release`
