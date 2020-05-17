glyph_brush_draw_cache
[![crates.io](https://img.shields.io/crates/v/glyph_brush_draw_cache.svg)](https://crates.io/crates/glyph_brush_draw_cache)
[![Documentation](https://docs.rs/glyph_brush_draw_cache/badge.svg)](https://docs.rs/glyph_brush_draw_cache)
======================
Rasterization cache for [ab_glyph](https://github.com/alexheretic/ab-glyph) used in glyph_brush.

* Manages a texture. Draws glyphs into it and provides texture rect lookup for glyphs.
* Automatic re-use & reordering when needed.

## Example
See the **draw_cache_guts** example to see how it works _(run it from the top level)_.

```
cargo run --example draw_cache_guts
```