# 0.7.10
* Fix conversions from OwnedSection -> Section with non-default extra type.

# 0.7.9
* Add `GlyphCruncher::glyphs` doc example of using `section_index` & `byte_index`.
* Update _rustc-hash_ to `2`.

# v0.7.8
* Update _ordered-float_ to `4`.

# v0.7.7
* Allow `Text::new` to work with any `X` type. **This may break usage**, however it will hopefully be non-breaking in practice as the compiler should always be able to infer this.
* Add `Section::builder` for more flexible `X`/"extra" type usage than `Section::default` & for owned text too.
* Add more flexible `X` type usage to `GlyphBrush::keep_cached`.
* Add `Section::from(text)` & `Section::from(vec![text])` conversions.
* Update `GlyphCruncher::glyphs`, `GlyphCruncher::glyph_bounds` docs.

# v0.7.6
* Removed _log_ dependency.

# 0.7.5
* Implement `Eq` for `BrushError`.

# 0.7.4
* Update _ordered-float_ to `3`.

# 0.7.3
* Add documentation of `GlyphBrush` generic types & workarounds when `.build()` type inference fails.
* wasm: Fix _twox-hash_ compilation.

# 0.7.2
* Add `GlyphBrushBuilder::multithread` to allow setting the (default on) draw-cache multithreading.

# 0.7.1
* Update _ordered-float_ to `2`.

# 0.7
* **OpenType (.otf) fonts are now supported** in addition to .ttf fonts. They're just as fast as .ttf fonts too.
* Rework crate switching from _rusttype_ to _ab_glyph_. ab_glyph is re-exported to `glyph_brush::ab_glyph`,
  rusttype types are gone, e.g. all usage of `rusttype::Scale` is replaced with `ab_glyph::PxScale`.
* New `Section` struct redesign replaces `VariedSection` & the old `Section` supporting generic extra data & builder syntax.
  ```rust
  // 0.7: Optional builder style, one API for simple & varied sections
  Section::default().add_text(Text::new("Hello").with_scale(25.0))
  ```
  ```rust
  // 0.6
  Section { text: "Hello", scale: Scale::uniform(25.0), ..<_>::default() }
  ```
* Section `color` & `z` are now part of an `Extra` struct which is the default `extra` type. This allows previous usage
  to continue but also downstream users to use different `Extra` data.
  This data is provided to the vertex generator function.
* To aid with refactoring from the previous version some equivalent versions of legacy structs are available
  `legacy::{VariedSection, Section, SectionText}`. So in some cases you can just slap `legacy::` in front of the old
  code and fix some `PxScale` usages and away you go. I'll deprecate and eventually remove this module.
  ```rust
  // 0.7
  legacy::Section { text: "Hello", scale: PxScale::from(25.0), ..<_>::default() }
  ```
* `pixel_bounds` has been removed, use `glyph_bounds` instead. Pixel bound info is not available for OpenType glyphs
  without calculating outlines. I've found it's almost always better to use `glyph_bounds` instead anyway,
  if not please open an issue with your use case.
* Rename `gpu_cache_*` methods to `draw_cache_*`. Also note, overriding the default _position_tolerance_ may be less necessary as the new
  draw cache features subpixel position error correction.
  ```rust
  // 0.7
  GlyphBrushBuilder::using_font(font).draw_cache_position_tolerance(0.1).build()
  ```
  ```rust
  // 0.6
  GlyphBrushBuilder::using_font(font).gpu_cache_position_tolerance(0.1).build()
  ```
* Rename `cache_glyph_drawing` to `cache_redraws` for clarity.
* New crate _glyph_brush_draw_cache_ takes rusttype's `gpu_cache` module into the ab_glyph world
  and starts to improve upon it.
* New _glyph_brush_layout_ now providers `section_index` & `byte_index` for all laid out glyphs. It no longer
  relies on any pixel bounding box info, which isn't fast to query with .otf fonts. This also means invisible glyphs, like ' ', are now generally included and hence will be returned by `GlyphCruncher::glyphs`.
* Add `GlyphBrush::is_draw_cached` which can be used to tell if a given queued & processed glyph is visible or not.
* It's faster. **~1.07-1.64x** faster than `0.6` (particularly in the worst case full-layout performance).
  Rasterization is also **~2-7x** faster using _ab_glyph_rasterizer_.

# 0.6.3
* Fix section color & alpha frame to frame changes to be incorrectly optimised as alpha only changes.

# 0.6.2
* Add `GlyphBrushBuilder::without_fonts()` for creating a brush without any fonts.

# 0.6.1
* Require `glyph_brush_layout` `0.1.8` to help ensure `rusttype` dependency convergence.

# 0.6
* `GlyphBrushBuilder` removed `initial_cache_size`, `gpu_cache_scale_tolerance`, `gpu_cache_position_tolerance`, `gpu_cache_align_4x4` public fields replaced by `gpu_cache_builder` field. This change allows the following changes.
* Add `GlyphBrush::to_builder` method to construct `GlyphBrushBuilder`s from `GlyphBrush`.
* Add `GlyphBrushBuilder::replace_fonts`, `GlyphBrushBuilder::rebuild` methods. Along with `to_builder` these may be used to rebuild a `GlyphBrush` with different fonts more conveniently.
* Replace `hashbrown` with `rustc-hash` + `std::collections` these are the same in 1.36.
* Update rusttype -> `0.8`. _Compatible with rusttype `0.6.5` & `0.7.9`._

# 0.5.4
* Semver trick re-export glyph_brush `0.6` without `GlyphBrushBuilder`.
* Export `GlyphBrushBuilderNext` returned by `GlyphBrush::to_builder`.

# 0.5.3
* Fix `queue_pre_positioned` cache check for position & scale changes.

# 0.5.2
* Removed screen dimensions from `process_queued` arguments. `to_vertex` function arguments also no longer include screen dimensions. Vertices should now be given pixel coordinates and use an appropriate projection matrix as a transform.
  <br/><br/>This approach simplifies glyph_brush code & allows the vertex cache to survive screen resolution changes. It also makes pre-projection custom transforms much easier to use. See usage changes in the opengl example & gfx_glyph.
* Add `GlyphCruncher::glyph_bounds` & `glyph_bounds_custom_layout` functions. These return section bounding boxes in terms of the font & glyph's size metrics, which can be more useful than the pixel rendering bounds provided by `pixel_bounds`.
* Add `GlyphBrushBuilder::gpu_cache_align_4x4` for rusttype gpu_cache `align_4x4` option. `delegate_glyph_brush_builder_fns!` includes this for downstream builders.
* Disallow `GlyphBrushBuilder` direct construction.
* Update hashbrown -> `0.5`.

# 0.5, 0.5.1
_yanked_

# 0.4.3
* Fix cached vertices erroneously remaining valid after screen dimension change.
* Update hashbrown -> `0.3`.

# 0.4.2
* Wasm32: Avoid using random state in the default hasher.

# 0.4.1
* Change default section hasher to xxHash as seahash has been shown to collide easily in 32bit environments.

# 0.4
* Use queue call counting & fine grained hashing to match up previous calls with current calls figuring out what has changed allowing optimised use of `recalculate_glyphs` for fast layouts.
  - Compute if just geometry (ie section position) has changed -> `GlyphChange::Geometry`.
  - Compute if just color has changed -> `GlyphChange::Color`.
  - Compute if just alpha has changed -> `GlyphChange::Alpha`.

  Provides much faster re-layout performance in these common change scenarios.
* `GlyphBrush` now generates & caches vertices avoiding regeneration of individual unchanged sections, when another section change forces regeneration of the complete vertex array. The user vertex type `V` is now in the struct signature.
  ```rust
  pub struct DownstreamGlyphBrush<'font, H = DefaultSectionHasher> {
      // previously: glyph_brush::GlyphBrush<'font, H>,
      inner: glyph_brush::GlyphBrush<'font, DownstreamGlyphVertex, H>,
      ...
  }
  ```

These changes result in a big performance improvement for changes to sections amongst other unchanging sections, which is a fairly common thing to want to do. Fully cached (everything unchanging) & worst-case (everything changing/new) are not significantly affected.

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
