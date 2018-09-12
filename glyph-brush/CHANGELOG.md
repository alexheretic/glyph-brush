# master
* Add public `DefaultSectionHasher`.
* Add `GlyphBrush::texture_dimensions`.
* Remove leaked public `GlyphedSection`.
* Improve some documentation using gfx-glyph specific terminology.
* Optimise calculate_glyph_cache trimming using intermediate fx-hashing.
  ```
  name                             control ns/iter  change ns/iter  diff ns/iter   diff %  speedup
  render_100_small_sections_fully  25,412           20,844                -4,568  -17.98%   x 1.22
  ```
* Add example usage using gl-rs.

# 0.1
* Initial release.
