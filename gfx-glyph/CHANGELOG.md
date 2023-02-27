# Unreleased (0.17.2)
* Up minimum _gfx_ version to `0.18.3`.

# 0.17.1
* Specify `#[repr(C)]` for vertex structs.

# 0.17
* **OpenType (.otf) fonts are now supported** in addition to .ttf fonts.
* Rework crate switching from rusttype to ab_glyph. See [glyph_brush changelog](https://github.com/alexheretic/glyph-brush/blob/master/glyph-brush/CHANGELOG.md#07).

# 0.16
* Remove deprecated `GlyphBrush::draw_queued` (use `use_queue()`).
* Update glyph_brush -> `0.6`.

# 0.15
* New API for drawing queued glyphs. Depth buffer usage is now optional.
  ```rust
  // v0.14
  glyph_brush.draw_queued(encoder, color, depth)?;
  // v0.15
  glyph_brush.use_queue().depth_target(depth).draw(encoder, color)?;
  ```
* Depth test now defaults to _Only draw when the fragment's output depth is less than or equal to the current depth buffer value, and update the buffer_. Instead of _Always pass, never write_. This is because depth buffer interaction is now optional.
* Custom transform usages are now expected to provide the orthographic projection, whereas before this projection was pre-baked. The shader also now inverts the y-axis to be more in-line with other APIs. Previous usages can be technically converted with:
  ```rust
  // v0.14
  glyph_brush.draw_queued_with_transform(custom_transform, ..);

  // v0.15
  glyph_brush
      .use_queue()
      .transform(invert_y * custom_transform * gfx_glyph::default_transform(&gfx_color))
      .draw(..);
  ```
  The new style allows easier pre-projection transformations, like rotation, as before only post-projection transforms were possible. Draws without custom transforms are unchanged, they now internally use `gfx_glyph::default_transform`.
* Deprecated `GlyphBrush::draw_queued` in favour of `use_queue()` draw builder usage.
* **Removed** `GlyphBrush::draw_queued_with_transform` in favour of `use_queue()` draw builder usage.
* Update glyph_brush -> `0.5`.

# 0.14.1
* Enlarge textures within `GL_MAX_TEXTURE_SIZE` if possible.

# 0.14
* Update gfx -> `0.18`.

# 0.13.3
* Update glyph_brush -> `0.4`, big performance improvements for changing sections.

# 0.13.2
* Optimise vertex updating by declaring 'Dynamic' usage & using explicit update calls.
* Add `GlyphBrush::queue_pre_positioned` and example *pre_positioned*.
* Add `SectionGeometry`, `GlyphPositioner` to glyph_brush re-exported types.
* Update glyph_brush -> `0.3`.

# 0.13.1
* Add `GlyphBrush::keep_cached`.

# 0.13
* Split crate creating layout project _glyph-brush-layout_ and render API agnostic _glyph-brush_. _gfx-glyph_ becomes a gfx-rs wrapper of _glyph-brush_. See [glyph_brush changes](../glyph-brush/CHANGELOG.md) & [glyph_brush_layout changes](../glyph-brush-layout/CHANGELOG.md).
  ```
  gfx-glyph
  └── glyph-brush
      └── glyph-brush-layout
  ```

# 0.12.2
* Update rusttype -> `0.7` bringing multithreaded rasterization in the texture cache. This brings a significant reduction in worst case latency in multicore environments.
   ```
   name                                    0.6.4 ns/iter    0.7 ns/iter     diff ns/iter   diff %  speedup
   cache::multi_font_population            8,239,309        2,570,034         -5,669,275  -68.81%   x 3.21
   cache_bad_cases::moving_text_thrashing  21,589,054       6,691,719        -14,897,335  -69.00%   x 3.23
   cache_bad_cases::resizing               15,162,054       4,607,499        -10,554,555  -69.61%   x 3.29
   ```
* Improve cache resizing performance using the new rusttype API.

_This release is semver compatible with rusttype `0.6.5` & `0.7`._

# 0.12.1
* Filter out of bounds glyphs in `VerticalAlign::Center` & `VerticalAlign::Bottom` layouts before texture cache phase as an extra step that reduces later work & gpu texture cache max size requirements.

New benchmarks added for the v-align center & bottom worst-case performance of a very large section only showing a partial amount. Filtering yields a 1.2x speedup.
```
name                                                control ns/iter  change ns/iter  diff ns/iter   diff %  speedup
no_cache_render_v_bottom_1_large_section_partially  12,412,793       10,342,991        -2,069,802  -16.67%   x 1.20
no_cache_render_v_center_1_large_section_partially  12,408,500       10,305,646        -2,102,854  -16.95%   x 1.20
render_v_bottom_1_large_section_partially           3,727            3,747                     20    0.54%   x 0.99
render_v_center_1_large_section_partially           3,727            3,726                     -1   -0.03%   x 1.00
```

# 0.12
* Layout code rework to a much cleaner implementation of layered iterators (#28)
  - Fixes issues with varied sections having inherent soft-breaks between `SectionText`s.
  - Remove built in unicode normalization
  - **Breaks** `GlyphPositioner` implementations _(not much implemented outside this crate afaik)_. But is now simpler to implement.
  - Add `VerticalAlign::Bottom`, `VerticalAlign::Center` (#33)
  - Fix single word larger than bounds issue (#34)
* Fix `BuiltInLineBreaker::AnyCharLineBreaker` mishandling byte-indices in some cases.
* Remove deprecated functions.
* Support raw gfx render & depth views (#30)
* Use generic section hashing for `GlyphBrush` & `GlyphCalculator` caches. This means the default section hashing can be overridden to a different algorithm if desired _(similarly to `HashMap`)_.
* Use `seahash` by default for section hashing. Previously this was done with an xxHash. Seahash is a little slower for large sections, but faster for small ones. General usage see many small sections & few large ones so seahash seems a better default.

## Performance
Worst-case _(cache miss)_ benchmark performance, which is the most important area to improve, is **hugely** improved by the layout rework. **1.55-2.16x** faster than `0.11`.

```
name                                       control ns/iter  change ns/iter  diff ns/iter   diff %  speedup
no_cache_render_100_small_sections_fully   7,267,231        4,691,164         -2,576,067  -35.45%   x 1.55
no_cache_render_1_large_section_partially  1,566,127        725,086             -841,041  -53.70%   x 2.16
no_cache_render_3_medium_sections_fully    4,051,124        1,963,114         -2,088,010  -51.54%   x 2.06
```

Best-case _(cached)_ performance changes are generally less important, but the affect of moving from xxHash to seahash can be seen.
```
name                                       control ns/iter  change ns/iter  diff ns/iter   diff %  speedup
render_100_small_sections_fully            34,219           24,757                -9,462  -27.65%   x 1.38
render_1_large_section_partially           2,634            3,972                  1,338   50.80%   x 0.66
render_3_medium_sections_fully             1,584            1,504                    -80   -5.05%   x 1.05
```

# 0.11
* Optimise vertex generation using instanced rendering. Improves worst-case performance by 18-50%.
* Update rusttype -> `0.6` including large texture cache performance improvements.

Overall worst-case _(cache miss)_ benchmark performance is improved by **42-74%** compared with gfx-glyph `0.10.2`.

```
name                                       control ns/iter  change ns/iter  diff ns/iter   diff %  speedup
no_cache_render_100_small_sections_fully   13,989,975       8,051,112         -5,938,863  -42.45%   x 1.74
no_cache_render_1_large_section_partially  2,377,767        1,643,650           -734,117  -30.87%   x 1.45
no_cache_render_3_medium_sections_fully    6,116,924        4,318,639         -1,798,285  -29.40%   x 1.42
```

# 0.10.2
* Add `GlyphBrush::add_font` & `GlyphBrush::add_font_bytes`

# 0.10.1
* Use rusttype gpu-cache glyph padding to avoid glyph texture artifacts after transforms
* Use default bilinear filtering to improve transformed glyph rendering
* Remove unused dependencies

# 0.10
* Update rusttype -> `0.5`, see [rusttype changelog](https://github.com/redox-os/rusttype/blob/master/CHANGELOG.md#050).
  Brings performance improvements.
```
name                                       control ns/iter  change ns/iter  diff ns/iter  diff %  speedup
no_cache_render_100_small_sections_fully   16,510,001       16,022,255          -487,746  -2.95%   x 1.03
no_cache_render_1_large_section_partially  4,404,936        4,381,983            -22,953  -0.52%   x 1.01
no_cache_render_3_medium_sections_fully    11,041,238       10,963,063           -78,175  -0.71%   x 1.01
```

# 0.9.1
* Switch to xxHashing for section caches
* Use upstream rusttype::gpu_cache _(All changes are upstreamed and released)_

_Bench change since 0.9.0_
```
name                                       control.stdout ns/iter  change.stdout ns/iter  diff ns/iter   diff %  speedup
render_100_small_sections_fully            34,236                  33,354                         -882   -2.58%   x 1.03
render_1_large_section_partially           6,970                   2,535                        -4,435  -63.63%   x 2.75
render_3_medium_sections_fully             2,165                   1,549                          -616  -28.45%   x 1.40

```

# 0.9
* Fix backtraces in warn logging when re-sizing the glyph texture cache
* Update rusttype 0.4

# 0.8.2
* Support multi-byte unicode characters in built-in layouts _(only partially supported before)_.
* Optimise vertex generation allocation; gives a small worst-case performance boost.
```
name                                                   control.stdout ns/iter  change.stdout ns/iter  diff ns/iter  diff %  speedup
no_cache_render_100_small_sections_fully               19,016,459              17,711,135               -1,305,324  -6.86%   x 1.07
no_cache_render_3_medium_sections_fully                12,896,250              12,053,503                 -842,747  -6.53%   x 1.07
no_cache_render_1_large_section_partially              4,897,027               4,705,228                  -191,799  -3.92%   x 1.04
```

# 0.8.1
* Improve GPU texture cache performance by partitioning the glyph lookup table by font & glyph id.
```
name                                                    control.stdout ns/iter  change.stdout ns/iter  diff ns/iter   diff %  speedup
gpu_cache::cache_bench_tests::cache_bench_tolerance_1   3,004,912               2,502,683                  -502,229  -16.71%   x 1.20
gpu_cache::cache_bench_tests::cache_bench_tolerance_p1  5,081,960               4,638,536                  -443,424   -8.73%   x 1.10
```

# 0.8
* Update to gfx `0.17`
* Update to log `0.4`

# 0.7
* `GlyphCalculator` allows font calculations / pixel bounds etc without actually being able to draw anything, or the need for gfx objects. Using a scoped caching system.
* Update to rusttype `0.3.0`
* Cache / font lifetime changes made to rusttype allow removing the `.standalone()` calls when adding glyphs to the gpu-cache. This and other rusttype optimisations can result in **up to ~25% faster worst case performance** (worst case being no position/draw caching / text changes every frame).
* `OwnedVariedSection` & `OwnedSectionText` to help some edge cases where borrowing is annoying.
* Simple `Debug` implementations to allow end users to derive more easily.

# 0.6.4
* Switch to OpenGL 3.2 / glsl 150 requirement to fix MacOS issues with glsl 130

I'm publishing as a minor increment even though this _may_ break your setup if you relied on OpenGL < 3.2 support, but I don't think anyone actually does. **Please get into contact if this broke your setup.**

# 0.6.3
* Fix `GlyphBrush` being able to use cached vertices after a render resolution change.
* When dynamically increasing the gpu glyph texture warn and show a backtrace to allow this to be addressed when using many `GlyphBrush` instances.

# 0.6.2
* Fix `VariedSection`s with multiple `SectionText` parts ignoring end-of-line hard breaks.

# 0.6.1
* Add `GlyphBrush#glyphs` & `#glyphs_custom_layout` methods to allow access to the `PositionedGlyphs` of a section.

# 0.6
* `GlyphBrushBuilder` supports `Font` types with `using_font`/`add_font`
* Renamed existing `GlyphBrushBuilder` methods -> `using_font_bytes`/`add_font_bytes`

# 0.5.1
* Fix rare glyph ordering panic in gpu cache code

# 0.5
* Allow sections with multiple diverse font/scale/color parts within the same layout & bounds.
  * New `VariedSection` available, see *varied* example.
* A single `GlyphBrush` can now support multiple fonts in a single GPU texture-cache.
* Improve `Layout` to be more flexible to change when using non-default.
  * E.g. `let layout = Layout::default().h_align(HorizontalAlign::Right)`
* Remove generics from `Section`.
* Improve glyph fragment shader to discard when alpha is zero. This improves out-of-order transparency of glyph holes, demonstrated in the *depth* example.
* Remove changelog from readme.

# 0.4.2
* Accept generic gfx depth formats, e.g `DepthStencil`

# 0.4
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

# 0.3.3
* Fix another GPU caching issue that could cause missing glyphs
* Fix a layout issue that could miss a character immediately preceding EOF
* Optimise GPU cache sorting performance

# 0.3.2
* Move fixed GPU caching logic into crate replacing `rusttype::gpu_cache`
* `Section` & `StaticSection` implement `Copy`

# 0.3
* Use `Into<SharedBytes>` instead of explicit `&[u8]` for font byte input to improve flexibility.

# 0.2
* Adopt default line breaking logic according to the Unicode Standard Annex \#14 with `StandardLineBreaker` (included in `Layout::default()`). A `LineBreaker` implementation can be provided instead of using one of these.

# 0.1
* Initial release.
