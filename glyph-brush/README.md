glyph_brush
[![crates.io](https://img.shields.io/crates/v/glyph_brush.svg)](https://crates.io/crates/glyph_brush)
[![Documentation](https://docs.rs/glyph_brush/badge.svg)](https://docs.rs/glyph_brush)
================
Fast cached text render library using [rusttype](https://gitlab.redox-os.org/redox-os/rusttype).

This crate has render API agnostic logic split of from [gfx-glyph](https://github.com/alexheretic/gfx-glyph). The aim is to eventually provide all the logic gfx-glyph has to provide very fast dynamically rasterized & cached text rendering without any render API allegiance.
