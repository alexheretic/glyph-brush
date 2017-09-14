gfx_glyph
<a href="https://crates.io/crates/gfx_glyph">
  <img src="http://img.shields.io/crates/v/gfx_glyph.svg">
</a>
<a href="https://docs.rs/gfx_glyph">
  <img src="https://docs.rs/gfx_glyph/badge.svg">
</a>
================

Fast GPU cached text rendering using [gfx-rs](https://github.com/gfx-rs/gfx) & [rusttype](https://github.com/dylanede/rusttype).

Makes use of three kinds of caching to optimise frame performance.

* Caching of glyph positioning output to avoid repeated cost of identical text
rendering on sequential frames.
* Caches draw calculations to avoid repeated cost of identical text rendering on
sequential frames.
* GPU cache logic to dynamically maintain a GPU texture of rendered glyphs.

```rust
extern crate gfx_glyph;
use gfx_glyph::{Section, GlyphBrushBuilder};

let arial: &[u8] = include_bytes!("examples/Arial Unicode.ttf");
let mut glyph_brush = GlyphBrushBuilder::using_font(arial)
    .build(gfx_factory.clone());

let section = Section {
    text: "Hello gfx_glyph",
    ..Section::default() // color, position, etc
};

glyph_brush.queue(section);
glyph_brush.queue(some_other_section);

glyph_brush.draw_queued(&mut gfx_encoder, &gfx_color, &gfx_depth).unwrap();
```

## Limitations
The current implementation only supports OpenGL 3.0 or later. But other rendering languages (that are supported by gfx) should be easy enough to add. Send in your PRs!

## Examples
Have a look at
* `cargo run --example paragraph --release`
* `cargo run --example performance --release`

## Changelog
**0.4**
* Support depth testing with configurable gfx depth test (via `GlyphBrushBuilder::depth_test`).
  * `Section`s now have a `z` value to indicate the depth.
  * Actual depth testing is disabled by default, but a reference to the depth buffer is now required to draw.
* Streamline API for use with built-in `Layout`s, while still allowing custom layouts.
  * Built-in layouts are now a member of `Section`.
  * Custom layouts can still be used by using `GlyphBrush::queue_custom_layout` method instead of `queue`.
  * `Section<'a, L>` are now generic to allow pluggable `LineBreaker` logic in the layout. This is a little unfortunate for the API surface.
* Remove unnecessary `OwnedSection` and `StaticSection` to simplify the API.
* `pixel_bounding_box` renamed to `pixel_bounds` & `pixel_bounds_custom_layout`
  * These now return `Option<_>` to handle the bounds of 'nothing' properly
* `GlyphBrushBuilder` `gpu_cache_position_tolerance` default reduced to 0.1 (from 1.0)

**0.3**
* Use `Into<SharedBytes>` instead of explicit `&[u8]` for font byte input to improve flexibility.

Notable non-breaking changes:
* **0.3.2**
  * Move fixed GPU caching logic into crate replacing `rusttype::gpu_cache`
  * `Section` & `StaticSection` implement `Copy`
* **0.3.3**
  * Fix another GPU caching issue that could cause missing glyphs
  * Fix a layout issue that could miss a character immediately preceding EOF
  * Optimise GPU cache sorting performance

**0.2**
* Adopt default line breaking logic according to the Unicode Standard Annex \#14 with `StandardLineBreaker` (included in `Layout::default()`). A `LineBreaker` implementation can be provided instead of using one of these.
