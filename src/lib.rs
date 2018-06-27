//! Fast GPU cached text rendering using gfx-rs & rusttype.
//!
//! Makes use of three kinds of caching to optimise frame performance.
//!
//! * Caching of glyph positioning output to avoid repeated cost of identical text
//! rendering on sequential frames.
//! * Caches draw calculations to avoid repeated cost of identical text rendering on
//! sequential frames.
//! * GPU cache logic to dynamically maintain a GPU texture of rendered glyphs.
//!
//! # Example
//!
//! ```no_run
//! # extern crate gfx;
//! # extern crate gfx_window_glutin;
//! # extern crate glutin;
//! extern crate gfx_glyph;
//! use gfx_glyph::{GlyphBrushBuilder, Section};
//! # fn main() {
//! # let events_loop = glutin::EventsLoop::new();
//! # let (_window, _device, mut gfx_factory, gfx_color, gfx_depth) =
//! #     gfx_window_glutin::init::<gfx::format::Srgba8, gfx::format::Depth>(
//! #         glutin::WindowBuilder::new(),
//! #         glutin::ContextBuilder::new(),
//! #         &events_loop);
//! # let mut gfx_encoder: gfx::Encoder<_, _> = gfx_factory.create_command_buffer().into();
//!
//! let dejavu: &[u8] = include_bytes!("../examples/DejaVuSans.ttf");
//! let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(dejavu).build(gfx_factory.clone());
//!
//! # let some_other_section = Section { text: "another", ..Section::default() };
//! let section = Section {
//!     text: "Hello gfx_glyph",
//!     ..Section::default()
//! };
//!
//! glyph_brush.queue(section);
//! glyph_brush.queue(some_other_section);
//!
//! glyph_brush
//!     .draw_queued(&mut gfx_encoder, &gfx_color, &gfx_depth)
//!     .unwrap();
//! # }
//! ```
#![allow(unknown_lints)]
#![warn(clippy)]
#![cfg_attr(feature = "bench", feature(test))]
#[cfg(test)]
#[macro_use]
extern crate approx;
#[cfg(test)]
extern crate env_logger;
#[cfg(test)]
#[macro_use]
extern crate lazy_static;
#[cfg(feature = "bench")]
extern crate test;

extern crate backtrace;
#[macro_use]
extern crate gfx;
extern crate gfx_core;
#[macro_use]
extern crate log;
extern crate ordered_float;
extern crate rustc_hash;
extern crate rusttype;
extern crate seahash;
extern crate xi_unicode;

mod builder;
mod layout;
mod linebreak;
mod pipe;
mod section;
#[macro_use]
mod trace;
mod glyph_calculator;
mod owned_section;
#[cfg(feature = "performance_stats")]
mod performance_stats;

pub use builder::*;
pub use glyph_calculator::*;
pub use layout::*;
pub use linebreak::*;
pub use owned_section::*;
pub use rusttype::{
    Font, Glyph, GlyphId, HMetrics, Point, PositionedGlyph, Rect, Scale, ScaledGlyph, SharedBytes,
    VMetrics,
};
pub use section::*;
use std::hash::BuildHasher;
use std::hash::BuildHasherDefault;

use gfx::traits::FactoryExt;
use gfx::{format, handle, texture};
use gfx_core::memory::Typed;
use pipe::*;
use rustc_hash::{FxHashMap, FxHashSet};
use rusttype::gpu_cache::{Cache, CacheBuilder};
use rusttype::point;
use std::borrow::{Borrow, Cow};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::i32;
use std::{fmt, slice};

/// An iterator over `PositionedGlyph`s from the `GlyphBrush`
pub type PositionedGlyphIter<'a, 'font> = std::iter::Map<
    slice::Iter<'a, (rusttype::PositionedGlyph<'font>, [f32; 4], section::FontId)>,
    fn(&'a (rusttype::PositionedGlyph<'font>, [f32; 4], section::FontId))
        -> &'a rusttype::PositionedGlyph<'font>,
>;

pub(crate) type Color = [f32; 4];

/// A hash of `Section` data
type SectionHash = u64;

// Type for the generated glyph cache texture
type TexForm = format::U8Norm;
type TexSurface = <TexForm as format::Formatted>::Surface;
type TexChannel = <TexForm as format::Formatted>::Channel;
type TexFormView = <TexForm as format::Formatted>::View;
type TexSurfaceHandle<R> = handle::Texture<R, TexSurface>;
type TexShaderView<R> = handle::ShaderResourceView<R, TexFormView>;

const IDENTITY_MATRIX4: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// A "practically collision free" `Section` hasher
type DefaultSectionHasher = BuildHasherDefault<seahash::SeaHasher>;

/// Object allowing glyph drawing, containing cache state. Manages glyph positioning cacheing,
/// glyph draw caching & efficient GPU texture cache updating and re-sizing on demand.
///
/// Build using a [`GlyphBrushBuilder`](struct.GlyphBrushBuilder.html).
///
/// # Example
///
/// ```no_run
/// # extern crate gfx;
/// # extern crate gfx_window_glutin;
/// # extern crate glutin;
/// extern crate gfx_glyph;
/// # use gfx_glyph::{GlyphBrushBuilder};
/// use gfx_glyph::Section;
/// # fn main() {
/// # let events_loop = glutin::EventsLoop::new();
/// # let (_window, _device, mut gfx_factory, gfx_color, gfx_depth) =
/// #     gfx_window_glutin::init::<gfx::format::Srgba8, gfx::format::Depth>(
/// #         glutin::WindowBuilder::new(),
/// #         glutin::ContextBuilder::new(),
/// #         &events_loop);
/// # let mut gfx_encoder: gfx::Encoder<_, _> = gfx_factory.create_command_buffer().into();
///
/// # let dejavu: &[u8] = include_bytes!("../examples/DejaVuSans.ttf");
/// # let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(dejavu)
/// #     .build(gfx_factory.clone());
///
/// # let some_other_section = Section { text: "another", ..Section::default() };
/// let section = Section {
///     text: "Hello gfx_glyph",
///     ..Section::default()
/// };
///
/// glyph_brush.queue(section);
/// glyph_brush.queue(some_other_section);
///
/// glyph_brush
///     .draw_queued(&mut gfx_encoder, &gfx_color, &gfx_depth)
///     .unwrap();
/// # }
/// ```
///
/// # Caching behaviour
///
/// Calls to [`GlyphBrush::queue`](#method.queue),
/// [`GlyphBrush::pixel_bounds`](#method.pixel_bounds), [`GlyphBrush::glyphs`](#method.glyphs)
/// calculate the positioned glyphs for a section.
/// This is cached so future calls to any of the methods for the same section are much
/// cheaper. In the case of [`GlyphBrush::queue`](#method.queue) the calculations will also be
/// used for actual drawing.
///
/// The cache for a section will be **cleared** after a
/// [`GlyphBrush::draw_queued`](#method.draw_queued) call when that section has not been used since
/// the previous draw call.
pub struct GlyphBrush<'font, R: gfx::Resources, F: gfx::Factory<R>, H = DefaultSectionHasher> {
    fonts: FxHashMap<FontId, rusttype::Font<'font>>,
    font_cache: Cache<'font>,
    font_cache_tex: (
        gfx::handle::Texture<R, TexSurface>,
        gfx_core::handle::ShaderResourceView<R, f32>,
    ),
    texture_filter_method: texture::FilterMethod,
    factory: F,
    program: gfx::handle::Program<R>,
    draw_cache: Option<DrawnGlyphBrush<R>>,

    // cache of section-layout hash -> computed glyphs, this avoid repeated glyph computation
    // for identical layout/sections common to repeated frame rendering
    calculate_glyph_cache: FxHashMap<SectionHash, GlyphedSection<'font>>,
    // calculate_glyph_cache2: FxHashMap<(), GlyphedSection<'font>>,

    // buffer of section-layout hashs (that must exist in the calculate_glyph_cache)
    // to be rendered on the next `draw_queued` call
    section_buffer: Vec<SectionHash>,

    // Set of section hashs to keep in the glyph cache this frame even if they haven't been drawn
    keep_in_cache: FxHashSet<SectionHash>,

    // config
    gpu_cache_scale_tolerance: f32,
    gpu_cache_position_tolerance: f32,
    cache_glyph_positioning: bool,
    cache_glyph_drawing: bool,

    depth_test: gfx::state::Depth,
    section_hasher: H,

    #[cfg(feature = "performance_stats")]
    perf: performance_stats::PerformanceStats,
}

impl<'font, R: gfx::Resources, F: gfx::Factory<R>, H> fmt::Debug for GlyphBrush<'font, R, F, H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GlyphBrush")
    }
}

impl<'font, R: gfx::Resources, F: gfx::Factory<R>, H: BuildHasher> GlyphCruncher<'font>
    for GlyphBrush<'font, R, F, H>
{
    fn pixel_bounds_custom_layout<'a, S, L>(
        &mut self,
        section: S,
        custom_layout: &L,
    ) -> Option<Rect<i32>>
    where
        L: GlyphPositioner + Hash,
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();
        let mut x = (0, 0);
        let mut y = (0, 0);
        let mut no_match = true;

        let section_hash = self.cache_glyphs(section.borrow(), custom_layout);
        self.keep_in_cache.insert(section_hash);

        for Rect { min, max } in self.calculate_glyph_cache[&section_hash]
            .glyphs
            .iter()
            .filter_map(|&(ref g, ..)| g.pixel_bounding_box())
        {
            if no_match || min.x < x.0 {
                x.0 = min.x;
            }
            if no_match || min.y < y.0 {
                y.0 = min.y;
            }
            if no_match || max.x > x.1 {
                x.1 = max.x;
            }
            if no_match || max.y > y.1 {
                y.1 = max.y;
            }

            no_match = false;
        }

        if no_match {
            None
        }
        else {
            Some(Rect {
                min: Point { x: x.0, y: y.0 },
                max: Point { x: x.1, y: y.1 },
            })
        }
    }

    fn glyphs_custom_layout<'a, 'b, S, L>(
        &'b mut self,
        section: S,
        custom_layout: &L,
    ) -> PositionedGlyphIter<'b, 'font>
    where
        L: GlyphPositioner + Hash,
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();
        let section_hash = self.cache_glyphs(section.borrow(), custom_layout);
        self.keep_in_cache.insert(section_hash);
        self.calculate_glyph_cache[&section_hash]
            .glyphs
            .iter()
            .map(|(g, ..)| g)
    }
}

impl<'font, R: gfx::Resources, F: gfx::Factory<R>, H: BuildHasher> GlyphBrush<'font, R, F, H> {
    /// Queues a section/layout to be drawn by the next call of
    /// [`draw_queued`](struct.GlyphBrush.html#method.draw_queued). Can be called multiple times
    /// to queue multiple sections for drawing.
    ///
    /// Used to provide custom `GlyphPositioner` logic, if using built-in
    /// [`Layout`](enum.Layout.html) simply use [`queue`](struct.GlyphBrush.html#method.queue)
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    pub fn queue_custom_layout<'a, S, G>(&mut self, section: S, custom_layout: &G)
    where
        G: GlyphPositioner,
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();
        if cfg!(debug_assertions) {
            for text in &section.text {
                assert!(self.fonts.contains_key(&text.font_id));
            }
        }
        let section_hash = self.cache_glyphs(&section, custom_layout);
        self.section_buffer.push(section_hash);
    }

    /// Queues a section/layout to be drawn by the next call of
    /// [`draw_queued`](struct.GlyphBrush.html#method.draw_queued). Can be called multiple times
    /// to queue multiple sections for drawing.
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    pub fn queue<'a, S>(&mut self, section: S)
    where
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();
        let layout = section.layout;
        self.queue_custom_layout(section, &layout)
    }

    fn hash<T: Hash>(&self, hashable: &T) -> SectionHash {
        let mut s = self.section_hasher.build_hasher();
        hashable.hash(&mut s);
        s.finish()
    }

    /// Returns the calculate_glyph_cache key for this sections glyphs
    fn cache_glyphs<L>(&mut self, section: &VariedSection, layout: &L) -> SectionHash
    where
        L: GlyphPositioner,
    {
        let section_hash = self.hash(&(section, layout));

        if self.cache_glyph_positioning {
            if let Entry::Vacant(entry) = self.calculate_glyph_cache.entry(section_hash) {
                #[cfg(feature = "performance_stats")]
                self.perf.layout_start();
                entry.insert(GlyphedSection {
                    bounds: layout.bounds_rect(section),
                    glyphs: layout.calculate_glyphs(&self.fonts, section),
                    z: section.z,
                });
                #[cfg(feature = "performance_stats")]
                self.perf.layout_finished();
            }
        }
        else {
            #[cfg(feature = "performance_stats")]
            self.perf.layout_start();
            self.calculate_glyph_cache.insert(
                section_hash,
                GlyphedSection {
                    bounds: layout.bounds_rect(section),
                    glyphs: layout.calculate_glyphs(&self.fonts, section),
                    z: section.z,
                },
            );
            #[cfg(feature = "performance_stats")]
            self.perf.layout_finished();
        }
        section_hash
    }

    /// Draws all queued sections onto a render target, applying a position transform (e.g.
    /// a projection).
    /// See [`queue`](struct.GlyphBrush.html#method.queue).
    ///
    /// Trims the cache, see [caching behaviour](#caching-behaviour).
    pub fn draw_queued<C, T, D>(
        &mut self,
        encoder: &mut gfx::Encoder<R, C>,
        target: &gfx::handle::RenderTargetView<R, T>,
        depth_target: &gfx::handle::DepthStencilView<R, D>,
    ) -> Result<(), String>
    where
        C: gfx::CommandBuffer<R>,
        T: format::RenderFormat,
        D: format::DepthFormat,
    {
        self.draw_queued_with_transform(IDENTITY_MATRIX4, encoder, target, depth_target)
    }

    /// Draws all queued sections onto a render target, applying a position transform (e.g.
    /// a projection).
    /// See [`queue`](struct.GlyphBrush.html#method.queue).
    ///
    /// Trims the cache, see [caching behaviour](#caching-behaviour).
    pub fn draw_queued_with_transform<C, T, D>(
        &mut self,
        transform: [[f32; 4]; 4],
        mut encoder: &mut gfx::Encoder<R, C>,
        target: &gfx::handle::RenderTargetView<R, T>,
        depth_target: &gfx::handle::DepthStencilView<R, D>,
    ) -> Result<(), String>
    where
        C: gfx::CommandBuffer<R>,
        T: format::RenderFormat,
        D: format::DepthFormat,
    {
        #[cfg(feature = "performance_stats")]
        self.perf.draw_start();

        let (screen_width, screen_height, ..) = target.get_dimensions();
        let (screen_width, screen_height) = (u32::from(screen_width), u32::from(screen_height));

        let current_text_state = self.hash(&(&self.section_buffer, screen_width, screen_height));

        if !self.cache_glyph_drawing
            || self.draw_cache.is_none()
            || self.draw_cache.as_ref().unwrap().texture_updated
            || self.draw_cache.as_ref().unwrap().last_text_state != current_text_state
        {
            loop {
                let mut no_text = true;

                for section_hash in &self.section_buffer {
                    let GlyphedSection { ref glyphs, .. } =
                        self.calculate_glyph_cache[section_hash];
                    for &(ref glyph, _, font_id) in glyphs {
                        self.font_cache.queue_glyph(font_id.0, glyph.clone());
                        no_text = false;
                    }
                }

                if no_text {
                    self.clear_section_buffer();
                    return Ok(());
                }

                let tex = self.font_cache_tex.0.clone();
                if let Err(err) = self.font_cache.cache_queued(|rect, tex_data| {
                    let offset = [rect.min.x as u16, rect.min.y as u16];
                    let size = [rect.width() as u16, rect.height() as u16];
                    update_texture(&mut encoder, &tex, offset, size, tex_data);
                }) {
                    let (width, height) = self.font_cache.dimensions();
                    let (new_width, new_height) = (width * 2, height * 2);

                    if let Some(ref mut cache) = self.draw_cache {
                        cache.texture_updated = true;
                    }

                    if log_enabled!(log::Level::Warn) {
                        warn!(
                            "Increasing glyph texture size {old:?} -> {new:?}, as {reason:?}. \
                             Consider building with `.initial_cache_size({new:?})` to avoid \
                             resizing. Called from:\n{trace}",
                            old = (width, height),
                            new = (new_width, new_height),
                            reason = err,
                            trace = outer_backtrace!()
                        );
                    }

                    let new_cache = CacheBuilder {
                        width: new_width,
                        height: new_height,
                        scale_tolerance: self.gpu_cache_scale_tolerance,
                        position_tolerance: self.gpu_cache_position_tolerance,
                        ..CacheBuilder::default()
                    }.build();

                    match create_texture(&mut self.factory, new_width, new_height) {
                        Ok((new_tex, tex_view)) => {
                            self.font_cache = new_cache;
                            self.font_cache_tex.1 = tex_view;
                            self.font_cache_tex.0 = new_tex;
                            continue;
                        }
                        Err(_) => {
                            self.section_buffer.clear();
                            return Err(format!(
                                "Failed to create {}x{} glyph texture",
                                new_width, new_height
                            ));
                        }
                    }
                }

                break;
            }
            #[cfg(feature = "performance_stats")]
            self.perf.gpu_cache_done();

            let verts: Vec<GlyphVertex> = {
                let sections: Vec<_> = self
                    .section_buffer
                    .iter()
                    .map(|hash| &self.calculate_glyph_cache[hash])
                    .collect();

                let mut verts = Vec::with_capacity(
                    sections
                        .iter()
                        .map(|section| section.glyphs.len())
                        .sum::<usize>(),
                );

                for &GlyphedSection {
                    ref glyphs,
                    bounds,
                    z,
                } in sections
                {
                    verts.extend(glyphs.into_iter().filter_map(|(glyph, color, font_id)| {
                        vertex(
                            glyph,
                            *color,
                            *font_id,
                            &self.font_cache,
                            bounds,
                            z,
                            (screen_width as f32, screen_height as f32),
                        )
                    }));
                }

                verts
            };
            #[cfg(feature = "performance_stats")]
            self.perf.vertex_generation_done();

            let vbuf = self.factory.create_vertex_buffer(&verts);

            let draw_cache = if let Some(mut cache) = self.draw_cache.take() {
                cache.pipe_data.vbuf = vbuf;
                cache.pipe_data.out = target.raw().clone();
                cache.pipe_data.out_depth = depth_target.raw().clone();
                if cache.pso.0 != T::get_format() {
                    cache.pso = (
                        T::get_format(),
                        self.pso_using(T::get_format(), D::get_format()),
                    );
                }
                cache.slice.instances.as_mut().unwrap().0 = verts.len() as _;
                cache.last_text_state = current_text_state;
                if cache.texture_updated {
                    cache.pipe_data.font_tex.0 = self.font_cache_tex.1.clone();
                    cache.texture_updated = false;
                }
                cache
            }
            else {
                DrawnGlyphBrush {
                    pipe_data: {
                        let sampler = self.factory.create_sampler(texture::SamplerInfo::new(
                            self.texture_filter_method,
                            texture::WrapMode::Clamp,
                        ));
                        glyph_pipe::Data {
                            vbuf,
                            font_tex: (self.font_cache_tex.1.clone(), sampler),
                            transform,
                            out: target.raw().clone(),
                            out_depth: depth_target.raw().clone(),
                        }
                    },
                    pso: (
                        T::get_format(),
                        self.pso_using(T::get_format(), D::get_format()),
                    ),
                    slice: gfx::Slice {
                        instances: Some((verts.len() as _, 0)),
                        ..Self::empty_slice()
                    },
                    last_text_state: 0,
                    texture_updated: false,
                }
            };

            self.draw_cache = Some(draw_cache);
        }

        if let Some(&mut DrawnGlyphBrush {
            ref pso,
            ref slice,
            ref mut pipe_data,
            ..
        }) = self.draw_cache.as_mut()
        {
            pipe_data.transform = transform;
            encoder.draw(slice, &pso.1, pipe_data);
        }

        self.clear_section_buffer();

        #[cfg(feature = "performance_stats")]
        {
            self.perf.draw_finished();
            self.perf.log_sluggishness();
        }

        Ok(())
    }

    /// Returns `FontId` -> `Font` map of available fonts.
    pub fn fonts(&self) -> &FxHashMap<FontId, Font<'font>> {
        &self.fonts
    }

    fn clear_section_buffer(&mut self) {
        if self.cache_glyph_positioning {
            // clear section_buffer & trim calculate_glyph_cache to active sections
            let mut active =
                HashSet::with_capacity(self.section_buffer.len() + self.keep_in_cache.len());

            for h in self.section_buffer.drain(..) {
                active.insert(h);
            }
            for h in self.keep_in_cache.drain() {
                active.insert(h);
            }
            self.calculate_glyph_cache
                .retain(|key, _| active.contains(key));
        }
        else {
            self.section_buffer.clear();
            self.calculate_glyph_cache.clear();
            self.keep_in_cache.clear();
        }
    }

    fn pso_using(
        &mut self,
        color_format: gfx::format::Format,
        depth_format: gfx::format::Format,
    ) -> gfx::PipelineState<R, glyph_pipe::Meta> {
        self.factory
            .create_pipeline_from_program(
                &self.program,
                gfx::Primitive::TriangleStrip,
                gfx::state::Rasterizer::new_fill(),
                glyph_pipe::Init::new(color_format, depth_format, self.depth_test),
            )
            .unwrap()
    }

    fn empty_slice() -> gfx::Slice<R> {
        gfx::Slice {
            start: 0,
            end: 4,
            buffer: gfx::IndexBuffer::Auto,
            base_vertex: 0,
            instances: None,
        }
    }

    /// Adds an additional font to the one(s) initially added on build.
    ///
    /// Returns a new [`FontId`](struct.FontId.html) to reference this font.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # extern crate gfx;
    /// # extern crate gfx_window_glutin;
    /// # extern crate glutin;
    /// extern crate gfx_glyph;
    /// use gfx_glyph::{GlyphBrushBuilder, Section};
    /// # fn main() {
    /// # let events_loop = glutin::EventsLoop::new();
    /// # let (_window, _device, mut gfx_factory, gfx_color, gfx_depth) =
    /// #     gfx_window_glutin::init::<gfx::format::Srgba8, gfx::format::Depth>(
    /// #         glutin::WindowBuilder::new(),
    /// #         glutin::ContextBuilder::new(),
    /// #         &events_loop);
    /// # let mut gfx_encoder: gfx::Encoder<_, _> = gfx_factory.create_command_buffer().into();
    ///
    /// // dejavu is built as default `FontId(0)`
    /// let dejavu: &[u8] = include_bytes!("../examples/DejaVuSans.ttf");
    /// let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(dejavu).build(gfx_factory.clone());
    ///
    /// // some time later, add another font referenced by a new `FontId`
    /// let open_sans_italic: &[u8] = include_bytes!("../examples/OpenSans-Italic.ttf");
    /// let open_sans_italic_id = glyph_brush.add_font_bytes(open_sans_italic);
    /// # glyph_brush.draw_queued(&mut gfx_encoder, &gfx_color, &gfx_depth).unwrap();
    /// # let _ = open_sans_italic_id;
    /// # }
    /// ```
    pub fn add_font_bytes<'a: 'font, B: Into<SharedBytes<'a>>>(&mut self, font_data: B) -> FontId {
        self.add_font(Font::from_bytes(font_data.into()).unwrap())
    }

    /// Adds an additional font to the one(s) initially added on build.
    ///
    /// Returns a new [`FontId`](struct.FontId.html) to reference this font.
    pub fn add_font<'a: 'font>(&mut self, font_data: Font<'a>) -> FontId {
        let next_id = FontId(self.fonts.keys().max().unwrap().0 + 1);
        self.fonts.insert(next_id, font_data);
        next_id
    }
}

struct DrawnGlyphBrush<R: gfx::Resources> {
    pipe_data: glyph_pipe::Data<R>,
    pso: (gfx::format::Format, gfx::PipelineState<R, glyph_pipe::Meta>),
    slice: gfx::Slice<R>,
    last_text_state: u64,
    texture_updated: bool,
}

#[derive(Clone)]
struct GlyphedSection<'font> {
    bounds: Rect<f32>,
    glyphs: Vec<(PositionedGlyph<'font>, Color, FontId)>,
    z: f32,
}

#[derive(Clone)]
pub struct GlyphedSectionText<'font>(pub Vec<PositionedGlyph<'font>>, pub Color, pub FontId);

#[inline]
fn vertex(
    glyph: &PositionedGlyph,
    color: Color,
    font_id: FontId,
    cache: &Cache,
    bounds: Rect<f32>,
    z: f32,
    (screen_width, screen_height): (f32, f32),
) -> Option<GlyphVertex> {
    let gl_bounds = Rect {
        min: point(
            2.0 * (bounds.min.x / screen_width - 0.5),
            2.0 * (0.5 - bounds.min.y / screen_height),
        ),
        max: point(
            2.0 * (bounds.max.x / screen_width - 0.5),
            2.0 * (0.5 - bounds.max.y / screen_height),
        ),
    };

    let rect = cache.rect_for(font_id.0, glyph);
    if let Ok(Some((mut uv_rect, screen_rect))) = rect {
        if screen_rect.min.x as f32 > bounds.max.x
            || screen_rect.min.y as f32 > bounds.max.y
            || bounds.min.x > screen_rect.max.x as f32
            || bounds.min.y > screen_rect.max.y as f32
        {
            // glyph is totally outside the bounds
            return None;
        }

        let mut gl_rect = Rect {
            min: point(
                2.0 * (screen_rect.min.x as f32 / screen_width - 0.5),
                2.0 * (0.5 - screen_rect.min.y as f32 / screen_height),
            ),
            max: point(
                2.0 * (screen_rect.max.x as f32 / screen_width - 0.5),
                2.0 * (0.5 - screen_rect.max.y as f32 / screen_height),
            ),
        };

        // handle overlapping bounds, modify uv_rect to preserve texture aspect
        if gl_rect.max.x > gl_bounds.max.x {
            let old_width = gl_rect.width();
            gl_rect.max.x = gl_bounds.max.x;
            uv_rect.max.x = uv_rect.min.x + uv_rect.width() * gl_rect.width() / old_width;
        }
        if gl_rect.min.x < gl_bounds.min.x {
            let old_width = gl_rect.width();
            gl_rect.min.x = gl_bounds.min.x;
            uv_rect.min.x = uv_rect.max.x - uv_rect.width() * gl_rect.width() / old_width;
        }
        // note: y access is flipped gl compared with screen,
        // texture is not flipped (ie is a headache)
        if gl_rect.max.y < gl_bounds.max.y {
            let old_height = gl_rect.height();
            gl_rect.max.y = gl_bounds.max.y;
            uv_rect.max.y = uv_rect.min.y + uv_rect.height() * gl_rect.height() / old_height;
        }
        if gl_rect.min.y > gl_bounds.min.y {
            let old_height = gl_rect.height();
            gl_rect.min.y = gl_bounds.min.y;
            uv_rect.min.y = uv_rect.max.y - uv_rect.height() * gl_rect.height() / old_height;
        }

        Some(GlyphVertex {
            left_top: [gl_rect.min.x, gl_rect.max.y, z],
            right_bottom: [gl_rect.max.x, gl_rect.min.y],
            tex_left_top: [uv_rect.min.x, uv_rect.max.y],
            tex_right_bottom: [uv_rect.max.x, uv_rect.min.y],
            color,
        })
    }
    else {
        if rect.is_err() {
            warn!("Cache miss?: {:?}", rect);
        }
        None
    }
}

// Creates a gfx texture with the given data
fn create_texture<F, R>(
    factory: &mut F,
    width: u32,
    height: u32,
) -> Result<(TexSurfaceHandle<R>, TexShaderView<R>), Box<Error>>
where
    R: gfx::Resources,
    F: gfx::Factory<R>,
{
    let kind = texture::Kind::D2(
        width as texture::Size,
        height as texture::Size,
        texture::AaMode::Single,
    );

    let tex = factory.create_texture(
        kind,
        1 as texture::Level,
        gfx::memory::Bind::SHADER_RESOURCE,
        gfx::memory::Usage::Dynamic,
        Some(<TexChannel as format::ChannelTyped>::get_channel_type()),
    )?;

    let view =
        factory.view_texture_as_shader_resource::<TexForm>(&tex, (0, 0), format::Swizzle::new())?;

    Ok((tex, view))
}

// Updates a texture with the given data (used for updating the GlyphCache texture)
fn update_texture<R, C>(
    encoder: &mut gfx::Encoder<R, C>,
    texture: &handle::Texture<R, TexSurface>,
    offset: [u16; 2],
    size: [u16; 2],
    data: &[u8],
) where
    R: gfx::Resources,
    C: gfx::CommandBuffer<R>,
{
    let info = texture::ImageInfoCommon {
        xoffset: offset[0],
        yoffset: offset[1],
        zoffset: 0,
        width: size[0],
        height: size[1],
        depth: 0,
        format: (),
        mipmap: 0,
    };
    encoder
        .update_texture::<TexSurface, TexForm>(texture, None, info, data)
        .unwrap();
}
