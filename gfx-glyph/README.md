gfx_glyph
[![crates.io](https://img.shields.io/crates/v/gfx_glyph.svg)](https://crates.io/crates/gfx_glyph)
[![Documentation](https://docs.rs/gfx_glyph/badge.svg)](https://docs.rs/gfx_glyph)
================

Fast GPU cached text rendering using [gfx-rs v0.18](https://github.com/gfx-rs/gfx/tree/pre-ll) & [glyph-brush](https://github.com/alexheretic/glyph-brush/tree/master/glyph-brush).

```rust
// TODO
```

## Examples
Have a look at
* `cargo run --example paragraph --release`
* `cargo run --example performance --release`
* `cargo run --example varied --release`
* `cargo run --example depth --release`


## Limitations
The current implementation supports OpenGL *(3.2 or later)* only. Use [glyph-brush](https://github.com/alexheretic/glyph-brush/tree/master/glyph-brush) directly if this is an issue.
