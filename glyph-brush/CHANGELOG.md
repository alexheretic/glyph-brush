# Unreleased
* Add `GlyphCruncher::fonts()` to common trait, hoisted from direct implementations. Add something like the following if you implement `GlyphCruncher`.
  ```rust
  impl GlyphCruncher for Foo {
      // new
      #[inline]
      fn fonts(&self) -> &[Font<'font>] {
          self.glyph_brush.fonts()
      }
  }
  ```
* Fix 2-draw style causing texture cache thrashing. _Probably a very rare bug_.
* Require log `0.4.4` to fix compile issue with older versions.
* Improve documentation.

# 0.2.4
* Add `GlyphBrush::keep_cached`.

# 0.2.3
* Use hashbrown map & sets improves some benchmarks by 1-4%.

# 0.2.2
* Add `GlyphCalculator::fonts` & `GlyphCalculatorGuard::fonts` methods.

# 0.2.1
* Fix on-off single section cache clearing.
* Fix double initial draw.

# 0.2
* Add public `DefaultSectionHasher`.
* Add `GlyphBrush::texture_dimensions`.
* Remove leaked public `GlyphedSection`.
* Remove `current` from `TextureTooSmall`, replaced by using `texture_dimensions()`.
* Improve some documentation using gfx-glyph specific terminology.
* Fix `pixel_bounds` returning `None` for some infinite bound + non-default layout cases.
* Optimise calculate_glyph_cache trimming using intermediate fx-hashing.
  ```
  name                             control ns/iter  change ns/iter  diff ns/iter   diff %  speedup
  render_100_small_sections_fully  25,412           20,844                -4,568  -17.98%   x 1.22
  ```
* Add example usage using gl-rs.

# 0.1
* Initial release.
