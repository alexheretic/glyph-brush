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
//! use gfx_glyph::{Section, GlyphBrushBuilder};
//! # fn main() {
//! # let events_loop = glutin::EventsLoop::new();
//! # let (_window, _device, mut gfx_factory, gfx_color, gfx_depth) =
//! #     gfx_window_glutin::init::<gfx::format::Srgba8, gfx::format::Depth>(
//! #         glutin::WindowBuilder::new(),
//! #         glutin::ContextBuilder::new(),
//! #         &events_loop);
//! # let mut gfx_encoder: gfx::Encoder<_, _> = gfx_factory.create_command_buffer().into();
//!
//! let arial: &[u8] = include_bytes!("../examples/Arial Unicode.ttf");
//! let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(arial)
//!     .build(gfx_factory.clone());
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
//! glyph_brush.draw_queued(&mut gfx_encoder, &gfx_color, &gfx_depth).unwrap();
//! # }
//! ```
#![cfg_attr(feature = "bench", feature(test))]
#[cfg(feature = "bench")]
extern crate test;
#[cfg(test)] extern crate pretty_env_logger;
#[cfg(test)] #[macro_use] extern crate approx;
#[cfg(test)] #[macro_use] extern crate lazy_static;

#[macro_use] extern crate log;
#[macro_use] extern crate gfx;
extern crate gfx_core;
extern crate rusttype;
extern crate unicode_normalization;
extern crate ordered_float;
extern crate xi_unicode;
extern crate linked_hash_map;
extern crate backtrace;

mod section;
mod layout;
mod gpu_cache;
mod linebreak;
mod pipe;
mod builder;
#[macro_use]
mod trace;
mod headless;
mod glyph_calculator;

use gfx::traits::FactoryExt;
use rusttype::{FontCollection, point, vector};
use gpu_cache::Cache;
use gfx::{handle, texture, format};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use gfx_core::memory::Typed;
use std::i32;
use std::borrow::{Cow, Borrow};
use std::error::Error;
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::iter;
use std::slice;
use std::time::*;
use pipe::*;
use log::LogLevel;

pub use section::*;
pub use layout::*;
pub use linebreak::*;
pub use builder::*;
pub use headless::*;
pub use glyph_calculator::*;

/// Aliased type to allow lib usage without declaring underlying **rusttype** lib
pub type Font<'a> = rusttype::Font<'a>;
/// Aliased type to allow lib usage without declaring underlying **rusttype** lib
pub type Scale = rusttype::Scale;
/// Aliased type to allow lib usage without declaring underlying **rusttype** lib
pub type Rect<T> = rusttype::Rect<T>;
/// Aliased type to allow lib usage without declaring underlying **rusttype** lib
pub type Point<T> = rusttype::Point<T>;
/// Aliased type to allow lib usage without declaring underlying **rusttype** lib
pub type PositionedGlyph<'font> = rusttype::PositionedGlyph<'font>;
/// Aliased type to allow lib usage without declaring underlying **rusttype** lib
pub type ScaledGlyph<'font> = rusttype::ScaledGlyph<'font>;
/// Aliased type to allow lib usage without declaring underlying **rusttype** lib
pub type Glyph<'font> = rusttype::Glyph<'font>;
/// Aliased type to allow lib usage without declaring underlying **rusttype** lib
pub type SharedBytes<'a> = rusttype::SharedBytes<'a>;
/// Aliased type to allow lib usage without declaring underlying **rusttype** lib
pub type HMetrics = rusttype::HMetrics;
/// Aliased type to allow lib usage without declaring underlying **rusttype** lib
pub type VMetrics = rusttype::VMetrics;
/// Aliased type to allow lib usage without declaring underlying **rusttype** lib
pub type GlyphId = rusttype::GlyphId;

/// An iterator over `PositionedGlyph`s from the `GlyphBrush`
pub type PositionedGlyphIter<'a, 'font> =
    iter::FlatMap<
        slice::Iter<'a, GlyphedSectionText<'font>>,
        slice::Iter<'a, rusttype::PositionedGlyph<'font>>,
        fn(&'a GlyphedSectionText<'font>) -> slice::Iter<'a, PositionedGlyph<'font>>
    >;

pub(crate) type Color = [f32; 4];

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

fn hash<H: Hash>(hashable: &H) -> u64 {
    let mut s = DefaultHasher::new();
    hashable.hash(&mut s);
    s.finish()
}

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
/// # let arial: &[u8] = include_bytes!("../examples/Arial Unicode.ttf");
/// # let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(arial)
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
/// glyph_brush.draw_queued(&mut gfx_encoder, &gfx_color, &gfx_depth).unwrap();
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
/// The cache for a section will be **cleared** after a [`GlyphBrush::draw_queued`](#method.draw_queued)
/// call when that section has not been used since the previous draw call.
pub struct GlyphBrush<'font, R: gfx::Resources, F: gfx::Factory<R>> {
    fonts: HashMap<FontId, rusttype::Font<'font>>,
    font_cache: Cache<'font>,
    font_cache_tex: (gfx::handle::Texture<R, TexSurface>, gfx_core::handle::ShaderResourceView<R, f32>),
    factory: F,
    draw_cache: Option<DrawnGlyphBrush<R>>,

    // cache of section-layout hash -> computed glyphs, this avoid repeated glyph computation
    // for identical layout/sections common to repeated frame rendering
    calculate_glyph_cache: HashMap<u64, GlyphedSection<'font>>,

    // buffer of section-layout hashs (that must exist in the calculate_glyph_cache)
    // to be rendered on the next `draw_queued` call
    section_buffer: Vec<u64>,

    // Set of section hashs to keep in the glyph cache this frame even if they haven't been drawn
    keep_in_cache: HashSet<u64>,

    // config
    gpu_cache_scale_tolerance: f32,
    gpu_cache_position_tolerance: f32,
    cache_glyph_positioning: bool,
    cache_glyph_drawing: bool,

    depth_test: gfx::state::Depth,
}

impl<'font, R: gfx::Resources, F: gfx::Factory<R>> GlyphCalculator<'font> for GlyphBrush<'font, R, F> {
    fn pixel_bounds_custom_layout<'a, S, L>(&mut self, section: S, custom_layout: &L)
        -> Option<Rect<i32>>
        where L: GlyphPositioner + Hash,
              S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();
        let mut x = (0, 0);
        let mut y = (0, 0);
        let mut no_match = true;

        let section_hash = self.cache_glyphs(section.borrow(), custom_layout);
        self.keep_in_cache.insert(section_hash);

        for g in self.calculate_glyph_cache[&section_hash]
            .glyphs.iter()
            .flat_map(|&GlyphedSectionText(ref g, ..)| g.iter())
        {
            if let Some(Rect{ min, max }) = g.pixel_bounding_box() {
                if no_match || min.x < x.0 { x.0 = min.x; }
                if no_match || min.y < y.0 { y.0 = min.y; }
                if no_match || max.x > x.1 { x.1 = max.x; }
                if no_match || max.y > y.1 { y.1 = max.y; }

                no_match = false;
            }
        }

        if no_match { None }
        else {
            Some(Rect {
                min: Point { x: x.0, y: y.0 },
                max: Point { x: x.1, y: y.1 },
            })
        }
    }

    fn glyphs_custom_layout<'a, 'b, S, L>(&'b mut self, section: S, custom_layout: &L)
        -> PositionedGlyphIter<'b, 'font>
        where L: GlyphPositioner + Hash,
              S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();
        let section_hash = self.cache_glyphs(section.borrow(), custom_layout);
        self.keep_in_cache.insert(section_hash);
        self.calculate_glyph_cache[&section_hash]
            .glyphs.iter()
            .flat_map(|&GlyphedSectionText(ref g, ..)| g.iter())
    }
}

impl<'font, R: gfx::Resources, F: gfx::Factory<R>> GlyphBrush<'font, R, F> {
    /// Queues a section/layout to be drawn by the next call of
    /// [`draw_queued`](struct.GlyphBrush.html#method.draw_queued). Can be called multiple times
    /// to queue multiple sections for drawing.
    ///
    /// Used to provide custom `GlyphPositioner` logic, if using built-in
    /// [`Layout`](enum.Layout.html) simply use [`queue`](struct.GlyphBrush.html#method.queue)
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    pub fn queue_custom_layout<'a, S, G>(&mut self, section: S, custom_layout: &G)
        where G: GlyphPositioner,
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
        where S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();
        let layout = section.layout;
        self.queue_custom_layout(section, &layout)
    }

    /// Returns the calculate_glyph_cache key for this sections glyphs
    fn cache_glyphs<L>(&mut self, section: &VariedSection, layout: &L) -> u64
        where L: GlyphPositioner,
    {
        let section_hash = hash(&(section, layout));

        if self.cache_glyph_positioning {
            if let Entry::Vacant(entry) = self.calculate_glyph_cache.entry(section_hash) {
                entry.insert(GlyphedSection {
                    bounds: layout.bounds_rect(section),
                    glyphs: layout.calculate_glyphs(&self.fonts, section),
                    z: section.z,
                });
            }
        }
        else {
            self.calculate_glyph_cache.insert(section_hash, GlyphedSection {
                bounds: layout.bounds_rect(section),
                glyphs: layout.calculate_glyphs(&self.fonts, section),
                z: section.z,
            });
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
        depth_target: &gfx::handle::DepthStencilView<R, D>)
        -> Result<(), String>
        where C: gfx::CommandBuffer<R>,
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
        let start = Instant::now();

        let mut verts_created = start.elapsed();
        let mut gpu_cache_finished = start.elapsed();

        let (screen_width, screen_height, _, _) = target.get_dimensions();
        let (screen_width, screen_height) = (u32::from(screen_width), u32::from(screen_height));

        let current_text_state = hash(&(&self.section_buffer, screen_width, screen_height));

        if !self.cache_glyph_drawing ||
            self.draw_cache.is_none() ||
            self.draw_cache.as_ref().unwrap().texture_updated ||
            self.draw_cache.as_ref().unwrap().last_text_state != current_text_state
        {
            loop {
                let mut no_text = true;

                for section_hash in &self.section_buffer {
                    let GlyphedSection { ref glyphs, .. } =
                        self.calculate_glyph_cache[section_hash];
                    for &GlyphedSectionText(ref glyphs, _, font_id) in glyphs {
                        for glyph in glyphs {
                            self.font_cache.queue_glyph(font_id.0, glyph.clone());
                            no_text = false;
                        }
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

                    if log_enabled!(LogLevel::Warn) {
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

                    let new_cache = Cache::new(
                        new_width,
                        new_height,
                        self.gpu_cache_scale_tolerance,
                        self.gpu_cache_position_tolerance,
                    );

                    match create_texture(&mut self.factory, new_width, new_height) {
                        Ok((new_tex, tex_view)) => {
                            self.font_cache = new_cache;
                            self.font_cache_tex.1 = tex_view;
                            self.font_cache_tex.0 = new_tex;
                            continue;
                        }
                        Err(_) => {
                            self.section_buffer.clear();
                            return Err(format!("Failed to create {}x{} glyph texture",
                                            new_width, new_height))
                        }
                    }
                }

                break;
            }
            gpu_cache_finished = start.elapsed();
            let gpu_cache_finished_time = start + gpu_cache_finished;

            let verts: Vec<GlyphVertex> = self.section_buffer.iter()
                .flat_map(|section_hash| {
                    let GlyphedSection{ ref glyphs, bounds, z }
                        = self.calculate_glyph_cache[section_hash];

                    glyphs.iter()
                        .flat_map(|&GlyphedSectionText(ref glyphs, color, font_id)| {
                            text_vertices(
                                glyphs,
                                color,
                                font_id,
                                &self.font_cache,
                                bounds,
                                z,
                                (screen_width as f32, screen_height as f32))
                        })
                        .collect::<Vec<_>>()
                })
                .collect();

            verts_created = gpu_cache_finished_time.elapsed();

            let (vbuf, slice) = self.factory.create_vertex_buffer_with_slice(&verts, ());

            let draw_cache = if self.draw_cache.is_some() {
                let mut cache = self.draw_cache.take().unwrap();
                cache.pipe_data.vbuf = vbuf;
                cache.pipe_data.out = target.raw().clone();
                cache.pipe_data.out_depth = depth_target.raw().clone();
                if cache.pso.0 != T::get_format() {
                    cache.pso = (T::get_format(), self.pso_using(T::get_format(), D::get_format()));
                }
                cache.slice = slice;
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
                            texture::FilterMethod::Scale,
                            texture::WrapMode::Clamp));
                        glyph_pipe::Data {
                            vbuf,
                            font_tex: (self.font_cache_tex.1.clone(), sampler),
                            transform: transform,
                            out: target.raw().clone(),
                            out_depth: depth_target.raw().clone(),
                        }
                    },
                    pso: (T::get_format(), self.pso_using(T::get_format(), D::get_format())),
                    slice,
                    last_text_state: 0,
                    texture_updated: false,
                }
            };

            self.draw_cache = Some(draw_cache);
        }

        if let Some(&mut DrawnGlyphBrush{ ref pso, ref slice, ref mut pipe_data, .. }) =
            self.draw_cache.as_mut()
        {
            pipe_data.transform = transform;
            encoder.draw(slice, &pso.1, pipe_data);
        }

        let draw_finished = {
            let elapsed = start.elapsed();
            if elapsed > verts_created + gpu_cache_finished {
                elapsed - (verts_created + gpu_cache_finished)
            }
            else {
                Duration::from_secs(0)
            }
        };

        self.clear_section_buffer();

        trace!("draw in {:.3}ms (gpu cache {:.3}ms, vertices {:.3}ms, draw-call {:.3}ms)",
            f64::from(start.elapsed().subsec_nanos()) / 1_000_000_f64,
            f64::from(gpu_cache_finished.subsec_nanos()) / 1_000_000_f64,
            f64::from(verts_created.subsec_nanos()) / 1_000_000_f64,
            f64::from(draw_finished.subsec_nanos()) / 1_000_000_f64);

        Ok(())
    }

    /// Returns `FontId` -> `Font` map of available fonts.
    pub fn fonts(&self) -> &HashMap<FontId, Font<'font>> {
        &self.fonts
    }

    fn clear_section_buffer(&mut self) {
        if self.cache_glyph_positioning {
            // clear section_buffer & trim calculate_glyph_cache to active sections
            let mut active = HashSet::with_capacity(
                self.section_buffer.len() + self.keep_in_cache.len()
            );

            for h in self.section_buffer.drain(..) {
                active.insert(h);
            }
            for h in self.keep_in_cache.drain() {
                active.insert(h);
            }
            self.calculate_glyph_cache.retain(|key, _| active.contains(key));
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
        depth_format: gfx::format::Format) -> gfx::PipelineState<R, glyph_pipe::Meta>
    {
        self.factory.create_pipeline_simple(
            include_bytes!("shader/vert.glsl"),
            include_bytes!("shader/frag.glsl"),
            glyph_pipe::Init::new(color_format, depth_format, self.depth_test)).unwrap()
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
    glyphs: Vec<GlyphedSectionText<'font>>,
    z: f32,
}

#[derive(Clone)]
pub struct GlyphedSectionText<'font>(pub Vec<PositionedGlyph<'font>>, pub Color, pub FontId);

/// Returns a Font from font bytes info or an error reason.
pub fn font<'a, B: Into<SharedBytes<'a>>>(font_bytes: B) -> Result<Font<'a>, &'static str> {
    let font_bytes = font_bytes.into();
    if font_bytes.is_empty() {
        return Err("Empty font data");
    }
    FontCollection::from_bytes(font_bytes)
        .into_font()
        .ok_or("Font not supported by rusttype")
}

#[inline]
fn text_vertices(
    glyphs: &[PositionedGlyph],
    color: Color,
    font_id: FontId,
    cache: &Cache,
    bounds: Rect<f32>,
    z: f32,
    (screen_width, screen_height): (f32, f32),
) -> Vec<GlyphVertex> {

    let origin = point(0.0, 0.0);
    let mut vertices = Vec::with_capacity(glyphs.len() * 6);

    let gl_bounds = Rect {
        min: origin
            + (vector(bounds.min.x as f32 / screen_width - 0.5,
                      1.0 - bounds.min.y as f32 / screen_height - 0.5)) * 2.0,
        max: origin
            + (vector(bounds.max.x as f32 / screen_width - 0.5,
                      1.0 - bounds.max.y as f32 / screen_height - 0.5)) * 2.0
    };

    for g in glyphs {
        let rect = cache.rect_for(font_id.0, g);
        if let Ok(Some((mut uv_rect, screen_rect))) = rect {
            if screen_rect.min.x as f32 > bounds.max.x ||
                screen_rect.min.y as f32 > bounds.max.y ||
                bounds.min.x > screen_rect.max.x as f32 ||
                bounds.min.y > screen_rect.max.y as f32 {
                // glyph is totally outside the bounds
                continue;
            }

            let mut gl_rect = Rect {
                min: origin
                    + (vector(screen_rect.min.x as f32 / screen_width - 0.5,
                              1.0 - screen_rect.min.y as f32 / screen_height - 0.5)) * 2.0,
                max: origin
                    + (vector(screen_rect.max.x as f32 / screen_width - 0.5,
                              1.0 - screen_rect.max.y as f32 / screen_height - 0.5)) * 2.0
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

            vertices.extend_from_slice(&[
                GlyphVertex {
                    pos: [gl_rect.min.x, gl_rect.max.y, z],
                    tex_pos: [uv_rect.min.x, uv_rect.max.y],
                    color,
                },
                GlyphVertex {
                    pos: [gl_rect.min.x, gl_rect.min.y, z],
                    tex_pos: [uv_rect.min.x, uv_rect.min.y],
                    color,
                },
                GlyphVertex {
                    pos: [gl_rect.max.x, gl_rect.min.y, z],
                    tex_pos: [uv_rect.max.x, uv_rect.min.y],
                    color,
                },
                GlyphVertex {
                    pos: [gl_rect.max.x, gl_rect.min.y, z],
                    tex_pos: [uv_rect.max.x, uv_rect.min.y],
                    color,
                },
                GlyphVertex {
                    pos: [gl_rect.max.x, gl_rect.max.y, z],
                    tex_pos: [uv_rect.max.x, uv_rect.max.y],
                    color,
                },
                GlyphVertex {
                    pos: [gl_rect.min.x, gl_rect.max.y, z],
                    tex_pos: [uv_rect.min.x, uv_rect.max.y],
                    color,
                }]);
        }
        else if rect.is_err() {
            warn!("Cache miss?: {:?}", rect);
        }
    }
    vertices
}

// Creates a gfx texture with the given data
fn create_texture<F, R>(factory: &mut F, width: u32, height: u32)
    -> Result<(TexSurfaceHandle<R>, TexShaderView<R>), Box<Error>>
    where R: gfx::Resources, F: gfx::Factory<R>
{
    let kind = texture::Kind::D2(
        width as texture::Size,
        height as texture::Size,
        texture::AaMode::Single);

    let tex = factory.create_texture(
        kind,
        1 as texture::Level,
        gfx::memory::SHADER_RESOURCE,
        gfx::memory::Usage::Dynamic,
        Some(<TexChannel as format::ChannelTyped>::get_channel_type()))?;

    let view = factory.view_texture_as_shader_resource::<TexForm>(
        &tex,
        (0, 0),
        format::Swizzle::new())?;

    Ok((tex, view))
}

// Updates a texture with the given data (used for updating the GlyphCache texture)
fn update_texture<R, C>(encoder: &mut gfx::Encoder<R, C>,
                        texture: &handle::Texture<R, TexSurface>,
                        offset: [u16; 2],
                        size: [u16; 2],
                        data: &[u8]) where R: gfx::Resources, C: gfx::CommandBuffer<R> {
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
    encoder.update_texture::<TexSurface, TexForm>(texture, None, info, data).unwrap();
}
