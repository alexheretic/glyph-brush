# Unreleased
* Use queue call counting & fine grained hashing to match up previous calls with current calls figuring out what has changed allowing optimised use of `recalculate_glyphs` for fast layouts.
  - Compute if just geometry (ie section position) has changed -> `GlyphChange::Geometry`.
  - Compute if just color has changed -> `GlyphChange::Color`.
  - Compute if just alpha has changed -> `GlyphChange::Alpha`.
  ```
  name                                   control ns/iter  change ns/iter  diff ns/iter   diff %  speedup
  continually_modify_alpha_of_1_of_3     858,297          561,664             -296,633  -34.56%   x 1.53
  continually_modify_color_of_1_of_3     870,442          558,456             -311,986  -35.84%   x 1.56
  continually_modify_position_of_1_of_3  867,278          567,991             -299,287  -34.51%   x 1.53
  ```

# 0.3
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
* Implement `PartialEq` for `*Section`s
* Implement `Clone`, `PartialEq` for `BrushError`
* Implement `Debug`, `Clone` for other public things.
* Add `GlyphBrush::queue_pre_positioned` for fully custom glyph positioning.

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
