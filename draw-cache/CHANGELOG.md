# 0.1.6
* Clarify `Rectangle` docs.
* Update _rustc-hash_ to `2`.

# 0.1.5
* Micro-optimise avoid `.round()` during glyph drawing when converting pixel coverage to `u8`.

# 0.1.4
* Optimise frequent lower workload efficiency by only using multithreading code paths when a
  significant speedup can be expected.
* Remove concurrent outlining as it didn't provide enough of a speedup after more thorough analysis.

# 0.1.3
* Update _crossbeam-deque_ to 0.8, _crossbeam-channel_ to 0.5.

# 0.1.2
* Optimise empty cache `cache_queued` calls by bundling texture data into a single upload.

# 0.1.1
* Require _ab_glyph_ 0.2.2.

# 0.1
* Port _rusttype_ gpu cache to _ab_glyph_.
* Use exact texture rect position, adjusted for different sub-pixel matches.
* Use rayon for concurrent outline calculation & rasterization.
* Use crossbeam-channel for channelling.
* Implement local batch work stealing for rasterization tasks improving population performance by **1.1x**.
