# master

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
