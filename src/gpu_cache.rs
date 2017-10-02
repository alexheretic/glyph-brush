//! Forked from `rusttype::gpu_cache`, with `rect_for` cache misses fixed.
//!
//! This module provides capabilities for managing a cache of rendered glyphs in GPU memory, with the goal of
//! minimisng the size and frequency of glyph uploads to GPU memory from the CPU.
//!
//! Typical applications that render directly with hardware graphics APIs (e.g. games) need text rendering.
//! There is not yet a performant solution for high quality text rendering directly on the GPU that isn't
//! experimental research work. Quality is often critical for legibility, so many applications use text or
//! individual characters that have been rendered on the CPU. This is done either ahead-of-time, giving a
//! fixed set of fonts, characters, and sizes that can be used at runtime, or dynamically as text is required.
//! This latter scenario is more flexible and the focus of this module.

//! To minimise the CPU load and texture upload bandwidth saturation, recently used glyphs should be cached
//! on the GPU for use by future frames. This module provides a mechanism for maintaining such a cache in the
//! form of a single packed 2D GPU texture. When a rendered glyph is requested, it is either retrieved from its
//! location in the texture if it is present or room is made in the cache (if necessary),
//! the CPU renders the glyph then it is uploaded into a gap in the texture to be available for GPU rendering.
//! This cache uses a Least Recently Used (LRU) cache eviction scheme - glyphs in the cache that have not been
//! used recently are as a rule of thumb not likely to be used again soon, so they are the best candidates for
//! eviction to make room for required glyphs.
//!
//! The API for the cache does not assume a particular graphics API. The intended usage is to queue up glyphs
//! that need to be present for the current frame using `Cache::queue_glyph`, update the cache to ensure that
//! the queued glyphs are present using `Cache::cache_queued` (providing a function for uploading pixel data),
//! then when it's time to render call `Cache::rect_for` to get the UV coordinates in the cache texture for
//! each glyph. For a concrete use case see the `gpu_cache` example.

use rusttype::{PositionedGlyph, Rect, Scale, GlyphId, Vector};
use std::collections::{HashMap, HashSet, BTreeMap};
use std::collections::Bound::{Included, Unbounded};
use linked_hash_map::LinkedHashMap;
use ordered_float::OrderedFloat;
use std::cmp::{PartialEq, Eq, Ord, PartialOrd, Ordering};

#[derive(Copy, Clone, Debug)]
struct PGlyphSpec {
    font_id: usize,
    glyph_id: GlyphId,
    scale: Scale,
    offset: Vector<f32>,
}

impl PartialEq for PGlyphSpec {
    fn eq(&self, other: &PGlyphSpec) -> bool {
        self.to_orderable() == other.to_orderable()
    }
}

impl Eq for PGlyphSpec {}

impl PartialOrd for PGlyphSpec {
    fn partial_cmp(&self, other: &PGlyphSpec) -> Option<Ordering> {
        self.to_orderable().partial_cmp(&other.to_orderable())
    }
}

impl Ord for PGlyphSpec {
    fn cmp(&self, other: &PGlyphSpec) -> Ordering {
        self.to_orderable().cmp(&other.to_orderable())
    }
}

impl PGlyphSpec {
    /// Returns if this cached glyph can be considered to match another at input tolerances
    fn matches(&self, other: &PGlyphSpec, scale_tolerance: f32, position_tolerance: f32)
        -> bool
    {
        self.font_id == other.font_id &&
            self.glyph_id == other.glyph_id &&
            (self.scale.x - other.scale.x).abs() < scale_tolerance &&
            (self.scale.y - other.scale.y).abs() < scale_tolerance &&
            (self.offset.x - other.offset.x).abs() < position_tolerance &&
            (self.offset.y - other.offset.y).abs() < position_tolerance
    }

    /// Returns a data view that is implicitly equal-able/hash-able/orderable
    fn to_orderable(&self) -> (
        usize,
        GlyphId,
        OrderedFloat<f32>,
        OrderedFloat<f32>,
        OrderedFloat<f32>,
        OrderedFloat<f32>,
    ) {
        let PGlyphSpec { font_id, glyph_id, scale, offset } = *self;
        (
            font_id,
            glyph_id,
            OrderedFloat(scale.x),
            OrderedFloat(scale.y),
            OrderedFloat(offset.x),
            OrderedFloat(offset.y),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ByteArray2d {
    inner_array: Vec<u8>,
    row: usize,
    col: usize,
}

impl ByteArray2d {
    pub fn zeros(row: usize, col: usize) -> Self {
        ByteArray2d {
            inner_array: vec![0; row * col],
            row: row,
            col: col,
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        self.inner_array.as_slice()
    }

    fn get_vec_index(&self, row: usize, col: usize) -> usize {
        if row >= self.row {
            panic!("row out of range: row={}, given={}", self.row, row);
        } else if col >= self.col {
            panic!("column out of range: col={}, given={}", self.col, col);
        } else {
            row * self.col + col
        }
    }
}

impl ::std::ops::Index<(usize, usize)> for ByteArray2d {
    type Output = u8;

    fn index(&self, (row, col): (usize, usize)) -> &u8 {
        &self.inner_array[self.get_vec_index(row, col)]
    }
}

impl ::std::ops::IndexMut<(usize, usize)> for ByteArray2d {
    fn index_mut(&mut self, (row, col): (usize, usize)) -> &mut u8 {
        let vec_index = self.get_vec_index(row, col);
        &mut self.inner_array[vec_index]
    }
}

struct Row {
    height: u32,
    width: u32,
    glyphs: Vec<(PGlyphSpec, Rect<u32>, ByteArray2d)>
}

/// An implementation of a dynamic GPU glyph cache. See the module documentation for more information.
pub struct Cache<'font> {
    scale_tolerance: f32,
    position_tolerance: f32,
    width: u32,
    height: u32,
    rows: LinkedHashMap<u32, Row>,
    space_start_for_end: HashMap<u32, u32>,
    space_end_for_start: HashMap<u32, u32>,
    queue: Vec<(usize, PositionedGlyph<'font>)>,
    queue_retry: bool,
    all_glyphs: BTreeMap<PGlyphSpec, (u32, u32)>
}

/// Returned from `Cache::rect_for`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CacheReadErr {
    /// Indicates that the requested glyph is not present in the cache
    GlyphNotCached
}
/// Returned from `Cache::cache_queued`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CacheWriteErr {
    /// At least one of the queued glyphs is too big to fit into the cache, even if all other glyphs are removed.
    GlyphTooLarge,
    /// Not all of the requested glyphs can fit into the cache, even if the cache is completely cleared before
    /// the attempt.
    NoRoomForWholeQueue
}
fn normalise_pixel_offset(mut offset: Vector<f32>) -> Vector<f32> {
    if offset.x > 0.5 {
        offset.x -= 1.0;
    } else if offset.x < -0.5 {
        offset.x += 1.0;
    }
    if offset.y > 0.5 {
        offset.y -= 1.0;
    } else if offset.y < -0.5 {
        offset.y += 1.0;
    }
    offset
}

#[allow(dead_code)] // from fork (keep in case we want to reuse this module)
impl<'font> Cache<'font> {
    /// Constructs a new cache. Note that this is just the CPU side of the cache. The GPU texture is managed
    /// by the user.
    ///
    /// `width` and `height` specify the dimensions of the 2D texture that will hold the cache contents on the
    /// GPU. This must match the dimensions of the actual texture used, otherwise `cache_queued` will try to
    /// cache into coordinates outside the bounds of the texture. If you need to change the dimensions of the
    /// cache texture (e.g. due to high cache pressure), construct a new `Cache` and discard the old one.
    ///
    /// `scale_tolerance` and `position_tolerance` specify the tolerances (maximum allowed difference)
    /// for judging whether an existing glyph
    /// in the cache is close enough to the requested glyph in scale and subpixel offset to be used in its
    /// place. Due to floating point inaccuracies that can affect user code it is not recommended to set these
    /// parameters too close to zero as effectively identical glyphs could end up duplicated in the cache.
    /// Both `scale_tolerance` and `position_tolerance` are measured in pixels. Note that since `pixel_tolerance`
    /// is a tolerance of subpixel offsets, setting it to 1.0 or higher is effectively a "don't care" option.
    /// A typical application will produce results with no perceptible inaccuracies with `scale_tolerance`
    /// and `position_tolerance` set to 0.1. Depending on the target DPI higher tolerance may be acceptable.
    ///
    /// # Panics
    ///
    /// `scale_tolerance` or `position_tolerance` are less than or equal to zero.
    pub fn new<'a>(
        width: u32,
        height: u32,
        scale_tolerance: f32,
        position_tolerance: f32,
    ) -> Cache<'a> {
        assert!(scale_tolerance >= 0.0);
        assert!(position_tolerance >= 0.0);
        let scale_tolerance = scale_tolerance.max(0.001);
        let position_tolerance = position_tolerance.max(0.001);
        Cache {
            scale_tolerance: scale_tolerance,
            position_tolerance: position_tolerance,
            width: width,
            height: height,
            rows: LinkedHashMap::new(),
            space_start_for_end: {let mut m = HashMap::new(); m.insert(height, 0); m},
            space_end_for_start: {let mut m = HashMap::new(); m.insert(0, height); m},
            queue: Vec::new(),
            queue_retry: false,
            all_glyphs: BTreeMap::new()
        }
    }

    /// Sets the scale tolerance for the cache. See the documentation for `Cache::new` for more information.
    ///
    /// # Panics
    ///
    /// `tolerance` is less than or equal to zero.
    pub fn set_scale_tolerance(&mut self, tolerance: f32) {
        assert!(tolerance >= 0.0);
        let tolerance = tolerance.max(0.001);
        self.scale_tolerance = tolerance;
    }
    /// Returns the current scale tolerance for the cache.
    pub fn scale_tolerance(&self) -> f32 {
        self.scale_tolerance
    }
    /// Sets the subpixel position tolerance for the cache. See the documentation for `Cache::new` for more
    /// information.
    ///
    /// # Panics
    ///
    /// `tolerance` is less than or equal to zero.
    pub fn set_position_tolerance(&mut self, tolerance: f32) {
        assert!(tolerance >= 0.0);
        let tolerance = tolerance.max(0.001);
        self.position_tolerance = tolerance;
    }
    /// Returns the current subpixel position tolerance for the cache.
    pub fn position_tolerance(&self) -> f32 {
        self.position_tolerance
    }
    /// Returns the cache texture dimensions assumed by the cache. For proper operation this should
    /// match the dimensions of the used GPU texture.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    /// Queue a glyph for caching by the next call to `cache_queued`. `font_id` is used to
    /// disambiguate glyphs from different fonts. The user should ensure that `font_id` is unique to the
    /// font the glyph is from.
    pub fn queue_glyph(&mut self, font_id: usize, glyph: PositionedGlyph<'font>) {
        if glyph.pixel_bounding_box().is_some() {
            self.queue.push((font_id, glyph));
        }
    }
    /// Clears the cache. Does not affect the glyph queue.
    pub fn clear(&mut self) {
        self.rows.clear();
        self.space_end_for_start.clear();
        self.space_end_for_start.insert(0, self.height);
        self.space_start_for_end.clear();
        self.space_start_for_end.insert(self.height, 0);
        self.all_glyphs.clear();
    }
    /// Clears the glyph queue.
    pub fn clear_queue(&mut self) {
        self.queue.clear();
    }
    /// Caches the queued glyphs. If this is unsuccessful, the queue is untouched.
    /// Any glyphs cached by previous calls to this function may be removed from the cache to make
    /// room for the newly queued glyphs. Thus if you want to ensure that a glyph is in the cache,
    /// the most recently cached queue must have contained that glyph.
    ///
    /// `uploader` is the user-provided function that should perform the texture uploads to the GPU.
    /// The information provided is the rectangular region to insert the pixel data into, and the pixel data
    /// itself. This data is provided in horizontal scanline format (row major), with stride equal to the
    /// rectangle width.
    pub fn cache_queued<F: FnMut(Rect<u32>, &[u8])>(&mut self, mut uploader: F) -> Result<(), CacheWriteErr> {
        use vector;
        use point;
        let mut in_use_rows = HashSet::new();
        // tallest first gives better packing
        // can use 'sort_unstable' as order of equal elements is unimportant
        self.queue.sort_unstable_by_key(|&(_, ref glyph)| {
            -glyph.pixel_bounding_box().unwrap().height()
        });
        let mut queue_success = true;
        'per_glyph: for &(font_id, ref glyph) in &self.queue {
            // Check to see if it's already cached, or a close enough version is:
            // (Note that the search for "close enough" here is conservative - there may be
            // a close enough glyph that isn't found; identical glyphs however will always be found)
            let p = glyph.position();
            let pfract = normalise_pixel_offset(vector(p.x.fract(), p.y.fract()));
            let spec = PGlyphSpec {
                font_id: font_id,
                glyph_id: glyph.id(),
                scale: glyph.scale(),
                offset: pfract
            };
            let lower = self.all_glyphs.range((Unbounded, Included(&spec))).rev().next()
                .and_then(|(l, &(lrow, _))| {
                    if l.font_id == spec.font_id &&
                        l.glyph_id == spec.glyph_id &&
                        (l.scale.x - spec.scale.x).abs() < self.scale_tolerance &&
                        (l.scale.y - spec.scale.y).abs() < self.scale_tolerance &&
                        (spec.offset.x - l.offset.x).abs() < self.position_tolerance &&
                        (spec.offset.y - l.offset.y).abs() < self.position_tolerance
                    {
                        Some((l.scale, l.offset, lrow))
                    } else {
                        None
                    }
                });
            let upper = self.all_glyphs.range((Included(&spec), Unbounded)).next()
                .and_then(|(u, &(urow, _))| {
                    if u.font_id == spec.font_id &&
                        u.glyph_id == spec.glyph_id &&
                        (u.scale.x - spec.scale.x).abs() < self.scale_tolerance &&
                        (u.scale.y - spec.scale.y).abs() < self.scale_tolerance &&
                        (spec.offset.x - u.offset.x).abs() < self.position_tolerance &&
                        (spec.offset.y - u.offset.y).abs() < self.position_tolerance
                    {
                        Some((u.scale, u.offset, urow))
                    } else {
                        None
                    }
                });
            match (lower, upper) {
                (None, None) => {} // No match
                (None, Some((_, _, row))) |
                (Some((_, _, row)), None) => {
                    // just one match
                    self.rows.get_refresh(&row);
                    in_use_rows.insert(row);
                    continue 'per_glyph;
                }
                (Some((_, _, row1)), Some((_, _, row2))) if row1 == row2 => {
                    // two matches, but the same row
                    self.rows.get_refresh(&row1);
                    in_use_rows.insert(row1);
                    continue 'per_glyph;
                }
                (Some((scale1, offset1, row1)), Some((scale2, offset2, row2))) => {
                    // two definitely distinct matches
                    let v1 =
                        ((scale1.x - spec.scale.x) / self.scale_tolerance).abs()
                        + ((scale1.y - spec.scale.y) / self.scale_tolerance).abs()
                        + ((offset1.x - spec.offset.x) / self.position_tolerance).abs()
                        + ((offset1.y - spec.offset.y) / self.position_tolerance).abs();
                    let v2 =
                        ((scale2.x - spec.scale.x) / self.scale_tolerance).abs()
                        + ((scale2.y - spec.scale.y) / self.scale_tolerance).abs()
                        + ((offset2.x - spec.offset.x) / self.position_tolerance).abs()
                        + ((offset2.y - spec.offset.y) / self.position_tolerance).abs();
                    let row = if v1 < v2 { row1 } else { row2 };
                    self.rows.get_refresh(&row);
                    in_use_rows.insert(row);
                    continue 'per_glyph;
                }
            }
            // Not cached, so add it:
            let bb = glyph.pixel_bounding_box().unwrap();
            let (width, height) = (bb.width() as u32, bb.height() as u32);
            if width >= self.width || height >= self.height {
                return Result::Err(CacheWriteErr::GlyphTooLarge);
            }
            // find row to put the glyph in, most used rows first
            let mut row_top = None;
            for (top, row) in self.rows.iter().rev() {
                if row.height >= height && self.width - row.width >= width {
                    // found a spot on an existing row
                    row_top = Some(*top);
                    break;
                }
            }

            if row_top.is_none() {
                let mut gap = None;
                // See if there is space for a new row
                for (start, end) in &self.space_end_for_start {
                    if end - start >= height {
                        gap = Some((*start, *end));
                        break;
                    }
                }
                if gap.is_none() {
                    // Remove old rows until room is available
                    while !self.rows.is_empty() {
                        // check that the oldest row isn't also in use
                        if !in_use_rows.contains(self.rows.front().unwrap().0) {
                            // Remove row
                            let (top, row) = self.rows.pop_front().unwrap();
                            for (spec, _, _) in row.glyphs {
                                self.all_glyphs.remove(&spec);
                            }
                            let (mut new_start, mut new_end) = (top, top + row.height);
                            // Update the free space maps
                            if let Some(end) = self.space_end_for_start.remove(&new_end) {
                                new_end = end;
                            }
                            if let Some(start) = self.space_start_for_end.remove(&new_start) {
                                new_start = start;
                            }
                            self.space_start_for_end.insert(new_end, new_start);
                            self.space_end_for_start.insert(new_start, new_end);
                            if new_end - new_start >= height {
                                // The newly formed gap is big enough
                                gap = Some((new_start, new_end));
                                break
                            }
                        }
                        // all rows left are in use
                        // try a clean insert of all needed glyphs
                        // if that doesn't work, fail
                        else if self.queue_retry {
                            // already trying a clean insert, don't do it again
                            return Err(CacheWriteErr::NoRoomForWholeQueue);
                        }
                        else { // signal that a retry is needed
                            queue_success = false;
                            break 'per_glyph;
                        }
                    }
                }
                let (gap_start, gap_end) = gap.unwrap();
                // fill space for new row
                let new_space_start = gap_start + height;
                self.space_end_for_start.remove(&gap_start);
                if new_space_start == gap_end {
                    self.space_start_for_end.remove(&gap_end);
                } else {
                    self.space_end_for_start.insert(new_space_start, gap_end);
                    self.space_start_for_end.insert(gap_end, new_space_start);
                }
                // add the row
                self.rows.insert(gap_start, Row {
                    width: 0,
                    height: height,
                    glyphs: Vec::new()
                });
                row_top = Some(gap_start);
            }
            let row_top = row_top.unwrap();
            // calculate the target rect
            let row = self.rows.get_refresh(&row_top).unwrap();
            let rect = Rect {
                min: point(row.width, row_top),
                max: point(row.width + width, row_top + height)
            };
            // draw the glyph into main memory
            let mut pixels = ByteArray2d::zeros(height as usize, width as usize);
            glyph.draw(|x, y, v| {
                let v = ((v * 255.0) + 0.5).floor().max(0.0).min(255.0) as u8;
                pixels[(y as usize, x as usize)] = v;
            });
            // transfer
            uploader(rect, pixels.as_slice());
            // add the glyph to the row
            row.glyphs.push((spec, rect, pixels));
            row.width += width;
            in_use_rows.insert(row_top);
            self.all_glyphs.insert(spec, (row_top, row.glyphs.len() as u32 - 1));
        }
        if queue_success {
            self.queue.clear();
            Ok(())
        } else { // clear the cache then try again
            self.clear();
            self.queue_retry = true;
            let result = self.cache_queued(uploader);
            self.queue_retry = false;
            result
        }
    }

    /// Retrieves the (floating point) texture coordinates of the quad for a glyph in the cache,
    /// as well as the pixel-space (integer) coordinates that this region should be drawn at.
    /// In the majority of cases these pixel-space coordinates should be identical to the bounding box of the
    /// input glyph. They only differ if the cache has returned a substitute glyph that is deemed close enough
    /// to the requested glyph as specified by the cache tolerance parameters.
    ///
    /// A sucessful result is `Some` if the glyph is not an empty glyph (no shape, and thus no rect to return).
    ///
    /// Ensure that `font_id` matches the `font_id` that was passed to `queue_glyph` with this `glyph`.
    pub fn rect_for<'a>(
        &'a self,
        font_id: usize,
        glyph: &PositionedGlyph)
        -> Result<Option<(Rect<f32>, Rect<i32>)>, CacheReadErr>
    {
        use vector;
        use point;
        let glyph_bb = match glyph.pixel_bounding_box() {
            Some(bb) => bb,
            None => return Ok(None)
        };
        let target_position = glyph.position();
        let target_offset = normalise_pixel_offset(vector(target_position.x.fract(), target_position.y.fract()));
        let target_spec = PGlyphSpec {
            font_id: font_id,
            glyph_id: glyph.id(),
            scale: glyph.scale(),
            offset: target_offset
        };

        let (lower, upper) = {
            let mut left_range = self.all_glyphs.range((Unbounded, Included(&target_spec))).rev();
            let mut right_range = self.all_glyphs.range((Included(&target_spec), Unbounded));

            let mut left = left_range.next().map(|(s, &(r, i))| (s, r, i));
            let mut right = right_range.next().map(|(s, &(r, i))| (s, r, i));

            while left.is_some() || right.is_some() {
                left = left.and_then(|(spec, row, index)| {
                    if spec.matches(&target_spec, self.scale_tolerance, self.position_tolerance) {
                        Some((spec, row, index))
                    }
                    else { None }
                });
                right = right.and_then(|(spec, row, index)| {
                    if spec.matches(&target_spec, self.scale_tolerance, self.position_tolerance) {
                        Some((spec, row, index))
                    }
                    else { None }
                });

                if left.is_none() && right.is_none() {
                    // continue searching for a match
                    left = left_range.next().map(|(s, &(r, i))| (s, r, i));
                    right = right_range.next().map(|(s, &(r, i))| (s, r, i));
                }
                else { break; }
            }
            (left, right)
        };

        let (width, height) = (self.width as f32, self.height as f32);
        let (match_spec, row, index) = match (lower, upper) {
            (None, None) => return Err(CacheReadErr::GlyphNotCached),
            (Some(match_), None) |
            (None, Some(match_)) => match_, // one match
            (Some((lmatch_spec, lrow, lindex)), Some((umatch_spec, urow, uindex))) => {
                if lrow == urow && lindex == uindex {
                    // both matches are really the same one, and match the input
                    let tex_rect = self.rows[&lrow].glyphs[lindex as usize].1;
                    let uv_rect = Rect {
                        min: point(tex_rect.min.x as f32 / width, tex_rect.min.y as f32 / height),
                        max: point(tex_rect.max.x as f32 / width, tex_rect.max.y as f32 / height)
                    };
                    return Ok(Some((uv_rect, glyph_bb)))
                } else {
                    // Two close-enough matches. Figure out which is closest.
                    let l_measure =
                        ((lmatch_spec.scale.x - target_spec.scale.x) / self.scale_tolerance).abs()
                        + ((lmatch_spec.scale.y - target_spec.scale.y) / self.scale_tolerance).abs()
                        + ((lmatch_spec.offset.x - target_spec.offset.x) / self.position_tolerance).abs()
                        + ((lmatch_spec.offset.y - target_spec.offset.y) / self.position_tolerance).abs();
                    let u_measure =
                        ((umatch_spec.scale.x - target_spec.scale.x) / self.scale_tolerance).abs()
                        + ((umatch_spec.scale.y - target_spec.scale.y) / self.scale_tolerance).abs()
                        + ((umatch_spec.offset.x - target_spec.offset.x) / self.position_tolerance).abs()
                        + ((umatch_spec.offset.y - target_spec.offset.y) / self.position_tolerance).abs();
                    if l_measure < u_measure {
                        (lmatch_spec, lrow, lindex)
                    } else {
                        (umatch_spec, urow, uindex)
                    }
                }
            }
        };
        let tex_rect = self.rows[&row].glyphs[index as usize].1;
        let uv_rect = Rect {
            min: point(tex_rect.min.x as f32 / width, tex_rect.min.y as f32 / height),
            max: point(tex_rect.max.x as f32 / width, tex_rect.max.y as f32 / height)
        };
        let local_bb = glyph
            .unpositioned().clone()
            .positioned(point(0.0, 0.0) + match_spec.offset).pixel_bounding_box().unwrap();
        let min_from_origin =
            point(local_bb.min.x as f32, local_bb.min.y as f32)
                    - (point(0.0, 0.0) + match_spec.offset);
        let ideal_min = min_from_origin + target_position;
        let min = point(ideal_min.x.round() as i32, ideal_min.y.round() as i32);
        let bb_offset = min - local_bb.min;
        let bb = Rect {
            min: min,
            max: local_bb.max + bb_offset
        };
        Ok(Some((uv_rect, bb)))
    }
}

#[cfg(test)]
#[test]
fn cache_test() {
    use ::FontCollection;
    use ::Scale;
    use ::point;
    let font_data = include_bytes!("../examples/Arial Unicode.ttf");
    let font = FontCollection::from_bytes(font_data as &[u8]).into_font().unwrap();
    let mut cache = Cache::new(32, 32, 0.1, 0.1);
    let strings = [
        ("Hello World!", 15.0),
        ("Hello World!", 14.0),
        ("Hello World!", 10.0),
        ("Hello World!", 15.0),
        ("Hello World!", 14.0),
        ("Hello World!", 10.0)
            ];
    for i in 0..strings.len() {
        println!("Caching {:?}", strings[i]);
        for glyph in font.layout(strings[i].0, Scale { x: strings[i].1, y: strings[i].1 }, point(0.0, 0.0)) {
            cache.queue_glyph(0, glyph);
        }
        cache.cache_queued(|_, _| {}).unwrap();
    }
}

#[cfg(test)]
#[test]
fn need_to_check_whole_cache() {
    use ::FontCollection;
    use ::Scale;
    use ::point;
    let font_data = include_bytes!("../examples/Arial Unicode.ttf");
    let font = FontCollection::from_bytes(font_data as &[u8]).into_font().unwrap();

    let glyph = font.glyph('l').unwrap();

    let small = glyph.clone().scaled(Scale::uniform(10.0));
    let large = glyph.clone().scaled(Scale::uniform(10.05));

    let small_left = small.clone().positioned(point(0.0, 0.0));
    let large_left = large.clone().positioned(point(0.0, 0.0));
    let large_right = large.clone().positioned(point(-0.2, 0.0));

    let mut cache = Cache::new(32, 32, 0.1, 0.1);

    cache.queue_glyph(0, small_left.clone());
    cache.queue_glyph(0, large_left.clone()); // Noop since it's within the scale tolerance of small_left
    cache.queue_glyph(0, large_right.clone());

    cache.cache_queued(|_, _| {}).unwrap();

    cache.rect_for(0, &small_left).unwrap();
    cache.rect_for(0, &large_left).unwrap();
    cache.rect_for(0, &large_right).unwrap();
}

#[cfg(feature = "bench")]
#[cfg(test)]
mod cache_bench_tests {
    use super::*;
    use ::{FontCollection, Scale, Font, point};

    const FONT_ID: usize = 0;
    const TEST_STR: &str = include_str!("../tests/lipsum.txt");

    lazy_static! {
        static ref FONT: Font<'static> = FontCollection
            ::from_bytes(include_bytes!("../examples/Arial Unicode.ttf") as &[u8])
            .into_font()
            .unwrap();
    }

    /// Reproduces Err(GlyphNotCached) issue & serves as a general purpose cache benchmark
    #[bench]
    fn cache_bench_tolerance_p1(b: &mut ::test::Bencher) {
        let glyphs = test_glyphs();
        let mut cache = Cache::new(512, 512, 0.1, 0.1);

        b.iter(|| {
            for glyph in &glyphs {
                cache.queue_glyph(FONT_ID, glyph.clone());
            }

            cache.cache_queued(|_, _| {}).expect("cache_queued");

            for (index, glyph) in glyphs.iter().enumerate() {
                let rect = cache.rect_for(FONT_ID, glyph);
                assert!(rect.is_ok(),
                    "Gpu cache rect lookup failed ({:?}) for glyph index {}, id {}",
                        rect, index, glyph.id().0);
            }
        });
    }

    #[bench]
    fn cache_bench_tolerance_1(b: &mut ::test::Bencher) {
        let glyphs = test_glyphs();
        let mut cache = Cache::new(512, 512, 0.1, 1.0);

        b.iter(|| {
            for glyph in &glyphs {
                cache.queue_glyph(FONT_ID, glyph.clone());
            }

            cache.cache_queued(|_, _| {}).expect("cache_queued");

            for (index, glyph) in glyphs.iter().enumerate() {
                let rect = cache.rect_for(FONT_ID, glyph);
                assert!(rect.is_ok(),
                    "Gpu cache rect lookup failed ({:?}) for glyph index {}, id {}",
                        rect, index, glyph.id().0);
            }
        });
    }

    fn test_glyphs() -> Vec<PositionedGlyph<'static>> {
        let mut glyphs = vec![];
        // Set of scales, found through brute force, to reproduce GlyphNotCached issue
        // Cache settings also affect this, it occurs when position_tolerance is < 1.0
        for scale in &[25_f32, 24.5, 25.01, 24.7, 24.99] {
            for glyph in layout_paragraph(&FONT, Scale::uniform(*scale), 500, TEST_STR) {
                glyphs.push(glyph);
            }
        }
        glyphs
    }

    fn layout_paragraph<'a>(font: &'a Font,
                            scale: Scale,
                            width: u32,
                            text: &str) -> Vec<PositionedGlyph<'a>> {
        use unicode_normalization::UnicodeNormalization;
        let mut result = Vec::new();
        let v_metrics = font.v_metrics(scale);
        let advance_height = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;
        let mut caret = point(0.0, v_metrics.ascent);
        let mut last_glyph_id = None;
        for c in text.nfc() {
            if c.is_control() {
                match c {
                    '\n' => { caret = point(0.0, caret.y + advance_height) },
                    _ => {}
                }
                continue;
            }
            let base_glyph = if let Some(glyph) = font.glyph(c) {
                glyph
            } else {
                continue;
            };
            if let Some(id) = last_glyph_id.take() {
                caret.x += font.pair_kerning(scale, id, base_glyph.id());
            }
            last_glyph_id = Some(base_glyph.id());
            let mut glyph = base_glyph.scaled(scale).positioned(caret);
            if let Some(bb) = glyph.pixel_bounding_box() {
                if bb.max.x > width as i32 {
                    caret = point(0.0, caret.y + advance_height);
                    glyph = glyph.into_unpositioned().positioned(caret);
                    last_glyph_id = None;
                }
            }
            caret.x += glyph.unpositioned().h_metrics().advance_width;
            result.push(glyph);
        }
        result
    }
}
