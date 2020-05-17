# glyph-brush
Fast cached text rendering.

## [glyph_brush](glyph-brush) [![](https://img.shields.io/crates/v/glyph_brush.svg)](https://crates.io/crates/glyph_brush) [![](https://docs.rs/glyph_brush/badge.svg)](https://docs.rs/glyph_brush)
Render API agnostic rasterization & draw caching text rendering.

## [gfx_glyph](gfx-glyph) [![](https://img.shields.io/crates/v/gfx_glyph.svg)](https://crates.io/crates/gfx_glyph) [![](https://docs.rs/gfx_glyph/badge.svg)](https://docs.rs/gfx_glyph)
glyph_brush wrapper for [gfx-rs v0.18](https://github.com/gfx-rs/gfx/tree/pre-ll) (OpenGL).

## [glyph_brush_layout](layout) [![](https://img.shields.io/crates/v/glyph_brush_layout.svg)](https://crates.io/crates/glyph_brush_layout) [![](https://docs.rs/glyph_brush_layout/badge.svg)](https://docs.rs/glyph_brush_layout)
Text layout for [ab_glyph](https://github.com/alexheretic/ab-glyph) used in glyph_brush.

## [glyph_brush_draw_cache](draw-cache) [![](https://img.shields.io/crates/v/glyph_brush_draw_cache.svg)](https://crates.io/crates/glyph_brush_draw_cache) [![](https://docs.rs/glyph_brush_draw_cache/badge.svg)](https://docs.rs/glyph_brush_draw_cache)
Rasterization cache for [ab_glyph](https://github.com/alexheretic/ab-glyph) used in glyph_brush.


## Examples
`cargo run -p glyph_brush --example opengl --release`

![](https://i.ibb.co/rvS2vp9/glyph-brush-example.png)

Also look at the individual crate readmes.

## Minimum supported rust compiler
All crates maintained with [latest stable rust](https://gist.github.com/alexheretic/d1e98d8433b602e57f5d0a9637927e0c).
