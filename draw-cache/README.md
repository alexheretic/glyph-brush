glyph_brush_draw_cache
[![crates.io](https://img.shields.io/crates/v/glyph_brush_draw_cache.svg)](https://crates.io/crates/glyph_brush_draw_cache)
[![Documentation](https://docs.rs/glyph_brush_draw_cache/badge.svg)](https://docs.rs/glyph_brush_draw_cache)
======================
Rasterization cache for [ab_glyph](https://github.com/alexheretic/ab-glyph) used in glyph_brush.

* Manages a texture. Draws glyphs into it and provides texture rect lookup for glyphs.
* Automatic re-use & reordering when needed.

```rust
use glyph_brush_draw_cache::DrawCache;

// build a cache with default settings
let mut draw_cache = DrawCache::builder().build();

// queue up some glyphs to store in the cache
for (font_id, glyph) in glyphs {
    draw_cache.queue_glyph(font_id, glyph);
}

// process everything in the queue, rasterizing & uploading as necessary
draw_cache.cache_queued(&fonts, |rect, tex_data| update_texture(rect, tex_data))?;

// access a given glyph's texture position & pixel position for the texture quad
match draw_cache.rect_for(font_id, &glyph) {
    Some((tex_coords, px_coords)) => {}
    None => {/* The glyph has no outline, or wasn't queued up to be cached */}
}
```

## Example
See the **draw_cache_guts** example to see how it works _(run it from the top level)_.

```
cargo run --example draw_cache_guts
```

![](https://user-images.githubusercontent.com/2331607/82690363-f97a9380-9c53-11ea-97bc-6f3397cde00f.png)