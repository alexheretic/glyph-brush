# 0.1
* Port rusttype gpu cache to ab_glyph.
* Use exact texture rect position, adjusted for different sub-pixel matches.
* Use rayon for concurrent outline calculation & rasterization.
* Use crossbeam-channel for channelling.
* Implement local batch work stealing for rasterization tasks improving population performance by **1.1x**.
