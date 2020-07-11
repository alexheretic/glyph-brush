# 0.1.1
* Require _ab_glyph_ 0.2.2.

# 0.1
* Port _rusttype_ gpu cache to _ab_glyph_.
* Use exact texture rect position, adjusted for different sub-pixel matches.
* Use rayon for concurrent outline calculation & rasterization.
* Use crossbeam-channel for channelling.
* Implement local batch work stealing for rasterization tasks improving population performance by **1.1x**.
