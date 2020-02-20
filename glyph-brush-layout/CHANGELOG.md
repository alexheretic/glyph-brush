# 0.1.9
* Fix consistency of section bounds by removing usage of glyph pixel bounds during word layout, instead always relying
  on advance width.
* Fix possible floating point errors when using section bounds that exactly bound the section.

# 0.1.8
* Update rusttype -> `0.8`. _Compatible with rusttype `0.6.5` & `0.7.9`._

# 0.1.7
* Update xi-unicode -> `0.2`.

# 0.1.6
* Fix missing line breaks for multi-byte breaking chars like Chinese characters.

# 0.1.5
* Add `GlyphPositioner::recalculate_glyphs` with a default unoptimised implementation. Custom layouts won't be broken by this change, but _will_ need to implement the new function to provide optimised behaviour.
* Optimise built-in layout's recalculate_glyphs for screen position changes with `GlyphChange::Geometry`.
* Optimise built-in layout's recalculate_glyphs for single color changes with `GlyphChange::Color`.
* Optimise built-in layout's recalculate_glyphs for alpha changes with `GlyphChange::Alpha`.
* Optimise layout re-positioning with `PositionedGlyph::set_position` usage.

# 0.1.4
* Implement `PartialEq` for `SectionGeometry` & `SectionText`.

# 0.1.3
* Implement `FontMap` for `AsRef<[Font]>` instead of `Index<usize, Output = Font>` to support arrays and slices. If this breaks your usage try implementing `FontMap` directly.

# 0.1.2
* Fix single-line vertical alignment y-adjustment for center & bottom.

# 0.1.1
* Re-export `rusttype::point`.
* Fix `bounds_rect` implementation for some `f32::INFINITY` cases.
* Handle zero & negative scale cases.

# 0.1
* Initial release.
