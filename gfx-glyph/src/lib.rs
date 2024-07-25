#![allow(
    clippy::cast_lossless,
    clippy::too_many_arguments,
    clippy::cognitive_complexity,
    clippy::redundant_closure
)]

//! Fast GPU cached text rendering using gfx-rs & ab_glyph.
//!
//! Makes use of three kinds of caching to optimise frame performance.
//!
//! * Caching of glyph positioning output to avoid repeated cost of identical text
//!   rendering on sequential frames.
//! * Caches draw calculations to avoid repeated cost of identical text rendering on
//!   sequential frames.
//! * GPU cache logic to dynamically maintain a GPU texture of rendered glyphs.
//!
//! # Example
//!
//! ```no_run
//! use gfx_glyph::{ab_glyph::FontArc, GlyphBrushBuilder, Section, Text};
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let glyph_brush: gfx_glyph::GlyphBrush<gfx_device_gl::Resources, gfx_device_gl::Factory> = unimplemented!();
//! # let gfx_color: gfx_core::handle::RenderTargetView<gfx_device_gl::Resources, gfx::format::Srgba8> = unimplemented!();
//! # let gfx_depth: gfx_core::handle::DepthStencilView<gfx_device_gl::Resources, gfx::format::Depth> = unimplemented!();
//! # let gfx_factory: gfx_device_gl::Factory = unimplemented!();
//! # let gfx_encoder: gfx::Encoder<_, _> = gfx_factory.create_command_buffer().into();
//!
//! let dejavu = FontArc::try_from_slice(include_bytes!("../../fonts/DejaVuSans.ttf"))?;
//! let mut glyph_brush = GlyphBrushBuilder::using_font(dejavu).build(gfx_factory.clone());
//!
//! # let some_other_section = Section::default();
//! let section = Section::default().add_text(Text::new("Hello gfx_glyph"));
//!
//! glyph_brush.queue(section);
//! glyph_brush.queue(some_other_section);
//!
//! glyph_brush.use_queue().draw(&mut gfx_encoder, &gfx_color)?;
//! # Ok(()) }
//! ```
mod builder;
mod pipe;
#[macro_use]
mod trace;
mod draw_builder;

pub use crate::{builder::*, draw_builder::*};
pub use glyph_brush::{
    ab_glyph, legacy, BuiltInLineBreaker, Extra, FontId, GlyphCruncher, GlyphPositioner,
    HorizontalAlign, Layout, LineBreak, LineBreaker, OwnedSection, OwnedText, Section,
    SectionGeometry, SectionGlyph, SectionGlyphIter, SectionText, Text, VerticalAlign,
};

use crate::pipe::{glyph_pipe, GlyphVertex, IntoDimensions, RawAndFormat};
use gfx::{
    format,
    handle::{self, RawDepthStencilView, RawRenderTargetView},
    texture,
    traits::FactoryExt,
};
use glyph_brush::{ab_glyph::*, BrushAction, BrushError, DefaultSectionHasher};
use log::{log_enabled, warn};
use std::{
    borrow::Cow,
    error::Error,
    fmt,
    hash::{BuildHasher, Hash},
};

// Type for the generated glyph cache texture
type TexForm = format::U8Norm;
type TexSurface = <TexForm as format::Formatted>::Surface;
type TexChannel = <TexForm as format::Formatted>::Channel;
type TexFormView = <TexForm as format::Formatted>::View;
type TexSurfaceHandle<R> = handle::Texture<R, TexSurface>;
type TexShaderView<R> = handle::ShaderResourceView<R, TexFormView>;

/// Returns the default 4 dimensional matrix orthographic projection used for drawing.
///
/// # Example
///
/// ```
/// # let (screen_width, screen_height) = (1f32, 2f32);
/// let projection = gfx_glyph::default_transform((screen_width, screen_height));
/// ```
///
/// # Example
///
/// ```no_run
/// # let gfx_color: gfx_core::handle::RenderTargetView<gfx_device_gl::Resources, gfx::format::Srgba8> = unimplemented!();
/// let projection = gfx_glyph::default_transform(&gfx_color);
/// ```
#[inline]
pub fn default_transform<D: IntoDimensions>(d: D) -> [[f32; 4]; 4] {
    let (w, h) = d.into_dimensions();
    [
        [2.0 / w, 0.0, 0.0, 0.0],
        [0.0, 2.0 / h, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [-1.0, -1.0, 0.0, 1.0],
    ]
}

/// Object allowing glyph drawing, containing cache state. Manages glyph positioning caching,
/// glyph draw caching & efficient GPU texture cache updating and re-sizing on demand.
///
/// Build using a [`GlyphBrushBuilder`](struct.GlyphBrushBuilder.html).
///
/// # Example
/// ```no_run
/// # use gfx_glyph::{GlyphBrushBuilder};
/// use gfx_glyph::{Section, Text};
/// # fn main() -> Result<(), String> {
/// # let glyph_brush: gfx_glyph::GlyphBrush<gfx_device_gl::Resources, gfx_device_gl::Factory> = unimplemented!();
/// # let gfx_color: gfx_core::handle::RenderTargetView<gfx_device_gl::Resources, gfx::format::Srgba8> = unimplemented!();
/// # let factory: gfx_device_gl::Factory = unimplemented!();
/// # let gfx_encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
/// # let some_other_section = Section::default();
///
/// let section = Section::default().add_text(Text::new("Hello gfx_glyph"));
///
/// glyph_brush.queue(section);
/// glyph_brush.queue(some_other_section);
///
/// glyph_brush.use_queue().draw(&mut gfx_encoder, &gfx_color)?;
/// # Ok(()) }
/// ```
///
/// # Caching behaviour
///
/// Calls to [`GlyphBrush::queue`](#method.queue),
/// [`GlyphBrush::glyph_bounds`](#method.glyph_bounds), [`GlyphBrush::glyphs`](#method.glyphs)
/// calculate the positioned glyphs for a section.
/// This is cached so future calls to any of the methods for the same section are much
/// cheaper. In the case of [`GlyphBrush::queue`](#method.queue) the calculations will also be
/// used for actual drawing.
///
/// The cache for a section will be **cleared** after a
/// [`.use_queue().draw(..)`](struct.DrawBuilder.html#method.draw) call when that section has not been used since
/// the previous draw call.
pub struct GlyphBrush<R: gfx::Resources, GF: gfx::Factory<R>, F = FontArc, H = DefaultSectionHasher>
{
    font_cache_tex: (
        gfx::handle::Texture<R, TexSurface>,
        gfx_core::handle::ShaderResourceView<R, f32>,
    ),
    texture_filter_method: texture::FilterMethod,
    factory: GF,
    program: gfx::handle::Program<R>,
    draw_cache: Option<DrawnGlyphBrush<R>>,
    glyph_brush: glyph_brush::GlyphBrush<GlyphVertex, Extra, F, H>,

    // config
    depth_test: gfx::state::Depth,
}

impl<R: gfx::Resources, GF: gfx::Factory<R>, F, H> fmt::Debug for GlyphBrush<R, GF, F, H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GlyphBrush")
    }
}

impl<R, GF, F, H> GlyphBrush<R, GF, F, H>
where
    R: gfx::Resources,
    GF: gfx::Factory<R>,
    H: BuildHasher,
    F: Font,
{
    /// Adds an additional font to the one(s) initially added on build.
    ///
    /// Returns a new [`FontId`](struct.FontId.html) to reference this font.
    pub fn add_font(&mut self, font: F) -> FontId {
        self.glyph_brush.add_font(font)
    }
}

impl<R, GF, F, H> GlyphCruncher<F, Extra> for GlyphBrush<R, GF, F, H>
where
    F: Font,
    R: gfx::Resources,
    GF: gfx::Factory<R>,
    H: BuildHasher,
{
    #[inline]
    fn glyphs_custom_layout<'a, 'b, S, L>(
        &'b mut self,
        section: S,
        custom_layout: &L,
    ) -> SectionGlyphIter<'b>
    where
        L: GlyphPositioner + Hash,
        S: Into<Cow<'a, Section<'a>>>,
    {
        self.glyph_brush
            .glyphs_custom_layout(section, custom_layout)
    }

    #[inline]
    fn fonts(&self) -> &[F] {
        self.glyph_brush.fonts()
    }

    #[inline]
    fn glyph_bounds_custom_layout<'a, S, L>(
        &mut self,
        section: S,
        custom_layout: &L,
    ) -> Option<Rect>
    where
        L: GlyphPositioner + Hash,
        S: Into<Cow<'a, Section<'a>>>,
    {
        self.glyph_brush
            .glyph_bounds_custom_layout(section, custom_layout)
    }
}

impl<R, GF, F, H> GlyphBrush<R, GF, F, H>
where
    F: Font + Sync,
    R: gfx::Resources,
    GF: gfx::Factory<R>,
    H: BuildHasher,
{
    /// Queues a section/layout to be drawn by the next call of
    /// [`.use_queue().draw(..)`](struct.DrawBuilder.html#method.draw). Can be called multiple times
    /// to queue multiple sections for drawing.
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    #[inline]
    pub fn queue<'a, S>(&mut self, section: S)
    where
        S: Into<Cow<'a, Section<'a>>>,
    {
        self.glyph_brush.queue(section)
    }

    /// Returns a [`DrawBuilder`](struct.DrawBuilder.html) allowing the queued glyphs to be drawn.
    ///
    /// Drawing will trim the cache, see [caching behaviour](#caching-behaviour).
    /// # Example
    ///
    /// ```no_run
    /// # fn main() -> Result<(), String> {
    /// # let glyph_brush: gfx_glyph::GlyphBrush<gfx_device_gl::Resources, gfx_device_gl::Factory> = unimplemented!();
    /// # let gfx_color: gfx_core::handle::RenderTargetView<gfx_device_gl::Resources, gfx::format::Srgba8> = unimplemented!();
    /// # let factory: gfx_device_gl::Factory = unimplemented!();
    /// # let gfx_encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    /// glyph_brush.use_queue().draw(&mut gfx_encoder, &gfx_color)?;
    /// # Ok(()) }
    /// ```
    #[inline]
    pub fn use_queue(&mut self) -> DrawBuilder<'_, F, R, GF, H, ()> {
        DrawBuilder {
            brush: self,
            transform: None,
            depth_target: None,
        }
    }

    /// Queues a section/layout to be drawn by the next call of
    /// [`.use_queue().draw(..)`](struct.DrawBuilder.html#method.draw). Can be called multiple times
    /// to queue multiple sections for drawing.
    ///
    /// Used to provide custom `GlyphPositioner` logic, if using built-in
    /// [`Layout`](enum.Layout.html) simply use [`queue`](struct.GlyphBrush.html#method.queue)
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    #[inline]
    pub fn queue_custom_layout<'a, S, G>(&mut self, section: S, custom_layout: &G)
    where
        G: GlyphPositioner,
        S: Into<Cow<'a, Section<'a>>>,
    {
        self.glyph_brush.queue_custom_layout(section, custom_layout)
    }

    /// Queues pre-positioned glyphs to be processed by the next call of
    /// [`.use_queue().draw(..)`](struct.DrawBuilder.html#method.draw). Can be called multiple times.
    #[inline]
    pub fn queue_pre_positioned(
        &mut self,
        glyphs: Vec<SectionGlyph>,
        extra: Vec<Extra>,
        bounds: Rect,
    ) {
        self.glyph_brush.queue_pre_positioned(glyphs, extra, bounds)
    }

    /// Retains the section in the cache as if it had been used in the last draw-frame.
    ///
    /// Should not be necessary unless using multiple draws per frame with distinct transforms,
    /// see [caching behaviour](#caching-behaviour).
    #[inline]
    pub fn keep_cached_custom_layout<'a, S, G>(&mut self, section: S, custom_layout: &G)
    where
        S: Into<Cow<'a, Section<'a>>>,
        G: GlyphPositioner,
    {
        self.glyph_brush
            .keep_cached_custom_layout(section, custom_layout)
    }

    /// Retains the section in the cache as if it had been used in the last draw-frame.
    ///
    /// Should not be necessary unless using multiple draws per frame with distinct transforms,
    /// see [caching behaviour](#caching-behaviour).
    #[inline]
    pub fn keep_cached<'a, S>(&mut self, section: S)
    where
        S: Into<Cow<'a, Section<'a>>>,
    {
        self.glyph_brush.keep_cached(section)
    }

    /// Returns the available fonts.
    ///
    /// The `FontId` corresponds to the index of the font data.
    #[inline]
    pub fn fonts(&self) -> &[F] {
        self.glyph_brush.fonts()
    }

    /// Draws all queued sections
    pub(crate) fn draw<C, CV, DV>(
        &mut self,
        transform: [[f32; 4]; 4],
        encoder: &mut gfx::Encoder<R, C>,
        target: &CV,
        depth_target: Option<&DV>,
    ) -> Result<(), String>
    where
        C: gfx::CommandBuffer<R>,
        CV: RawAndFormat<Raw = RawRenderTargetView<R>>,
        DV: RawAndFormat<Raw = RawDepthStencilView<R>>,
    {
        let mut brush_action;

        loop {
            let tex = self.font_cache_tex.0.clone();

            brush_action = self.glyph_brush.process_queued(
                |rect, tex_data| {
                    let offset = [rect.min[0] as u16, rect.min[1] as u16];
                    let size = [rect.width() as u16, rect.height() as u16];
                    update_texture(encoder, &tex, offset, size, tex_data);
                },
                to_vertex,
            );

            match brush_action {
                Ok(_) => break,
                Err(BrushError::TextureTooSmall { suggested }) => {
                    let max_image_dimension =
                        self.factory.get_capabilities().max_texture_size as u32;
                    let (new_width, new_height) = if (suggested.0 > max_image_dimension
                        || suggested.1 > max_image_dimension)
                        && (self.glyph_brush.texture_dimensions().0 < max_image_dimension
                            || self.glyph_brush.texture_dimensions().1 < max_image_dimension)
                    {
                        (max_image_dimension, max_image_dimension)
                    } else {
                        suggested
                    };

                    if log_enabled!(log::Level::Warn) {
                        warn!(
                            "Increasing glyph texture size {old:?} -> {new:?}. \
                             Consider building with `.initial_cache_size({new:?})` to avoid \
                             resizing. Called from:\n{trace}",
                            old = self.glyph_brush.texture_dimensions(),
                            new = (new_width, new_height),
                            trace = outer_backtrace!()
                        );
                    }

                    match create_texture(&mut self.factory, new_width, new_height) {
                        Ok((new_tex, tex_view)) => {
                            self.glyph_brush.resize_texture(new_width, new_height);

                            if let Some(ref mut cache) = self.draw_cache {
                                cache.pipe_data.font_tex.0 = tex_view.clone();
                            }

                            self.font_cache_tex.1 = tex_view;
                            self.font_cache_tex.0 = new_tex;
                        }
                        Err(_) => {
                            return Err(format!(
                                "Failed to create {new_width}x{new_height} glyph texture"
                            ));
                        }
                    }
                }
            }
        }

        // refresh pipe data
        // - pipe targets may have changed, or had resolutions changes
        // - format may have changed
        if let Some(mut cache) = self.draw_cache.take() {
            if &cache.pipe_data.out != target.as_raw() {
                cache.pipe_data.out.clone_from(target.as_raw());
            }
            if let Some(depth_target) = depth_target {
                if cache.pipe_data.out_depth.as_ref() != Some(depth_target.as_raw()) {
                    cache
                        .pipe_data
                        .out_depth
                        .clone_from(&Some(depth_target.as_raw().clone()));
                }
            } else {
                cache.pipe_data.out_depth.take();
            }
            if cache.pso.0 != target.format() {
                cache.pso = (
                    target.format(),
                    self.pso_using(target.format(), depth_target.map(|d| d.format())),
                );
            }
            self.draw_cache = Some(cache);
        }

        match brush_action.unwrap() {
            BrushAction::Draw(verts) => {
                let draw_cache = if let Some(mut cache) = self.draw_cache.take() {
                    if cache.pipe_data.vbuf.len() < verts.len() {
                        cache.pipe_data.vbuf =
                            new_vertex_buffer(&mut self.factory, encoder, &verts);
                    } else {
                        encoder
                            .update_buffer(&cache.pipe_data.vbuf, &verts, 0)
                            .unwrap();
                    }
                    cache.slice.instances.as_mut().unwrap().0 = verts.len() as _;
                    cache
                } else {
                    let vbuf = new_vertex_buffer(&mut self.factory, encoder, &verts);

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
                                out: target.as_raw().clone(),
                                out_depth: depth_target.map(|d| d.as_raw().clone()),
                            }
                        },
                        pso: (
                            target.format(),
                            self.pso_using(target.format(), depth_target.map(|d| d.format())),
                        ),
                        slice: gfx::Slice {
                            instances: Some((verts.len() as _, 0)),
                            ..Self::empty_slice()
                        },
                    }
                };

                self.draw_cache = Some(draw_cache);
            }
            BrushAction::ReDraw => {}
        };

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

        Ok(())
    }

    fn pso_using(
        &mut self,
        color_format: gfx::format::Format,
        depth_format: Option<gfx::format::Format>,
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
}

struct DrawnGlyphBrush<R: gfx::Resources> {
    pipe_data: glyph_pipe::Data<R>,
    pso: (gfx::format::Format, gfx::PipelineState<R, glyph_pipe::Meta>),
    slice: gfx::Slice<R>,
}

/// Allocates a vertex buffer 1 per glyph that will be updated on text changes
#[inline]
fn new_vertex_buffer<R: gfx::Resources, F: gfx::Factory<R>, C: gfx::CommandBuffer<R>>(
    factory: &mut F,
    encoder: &mut gfx::Encoder<R, C>,
    verts: &[GlyphVertex],
) -> gfx::handle::Buffer<R, GlyphVertex> {
    let buf = factory
        .create_buffer(
            verts.len(),
            gfx::buffer::Role::Vertex,
            gfx::memory::Usage::Dynamic,
            gfx::memory::Bind::empty(),
        )
        .unwrap();
    encoder.update_buffer(&buf, verts, 0).unwrap();
    buf
}

#[inline]
fn to_vertex(
    glyph_brush::GlyphVertex {
        mut tex_coords,
        pixel_coords,
        bounds,
        extra,
    }: glyph_brush::GlyphVertex,
) -> GlyphVertex {
    let gl_bounds = bounds;

    let mut gl_rect = Rect {
        min: point(pixel_coords.min.x, pixel_coords.min.y),
        max: point(pixel_coords.max.x, pixel_coords.max.y),
    };

    // handle overlapping bounds, modify uv_rect to preserve texture aspect
    if gl_rect.max.x > gl_bounds.max.x {
        let old_width = gl_rect.width();
        gl_rect.max.x = gl_bounds.max.x;
        tex_coords.max.x = tex_coords.min.x + tex_coords.width() * gl_rect.width() / old_width;
    }
    if gl_rect.min.x < gl_bounds.min.x {
        let old_width = gl_rect.width();
        gl_rect.min.x = gl_bounds.min.x;
        tex_coords.min.x = tex_coords.max.x - tex_coords.width() * gl_rect.width() / old_width;
    }
    if gl_rect.max.y > gl_bounds.max.y {
        let old_height = gl_rect.height();
        gl_rect.max.y = gl_bounds.max.y;
        tex_coords.max.y = tex_coords.min.y + tex_coords.height() * gl_rect.height() / old_height;
    }
    if gl_rect.min.y < gl_bounds.min.y {
        let old_height = gl_rect.height();
        gl_rect.min.y = gl_bounds.min.y;
        tex_coords.min.y = tex_coords.max.y - tex_coords.height() * gl_rect.height() / old_height;
    }

    GlyphVertex {
        left_top: [gl_rect.min.x, gl_rect.max.y, extra.z],
        right_bottom: [gl_rect.max.x, gl_rect.min.y],
        tex_left_top: [tex_coords.min.x, tex_coords.max.y],
        tex_right_bottom: [tex_coords.max.x, tex_coords.min.y],
        color: extra.color,
    }
}

// Creates a gfx texture with the given data
fn create_texture<GF, R>(
    factory: &mut GF,
    width: u32,
    height: u32,
) -> Result<(TexSurfaceHandle<R>, TexShaderView<R>), Box<dyn Error>>
where
    R: gfx::Resources,
    GF: gfx::Factory<R>,
{
    let kind = texture::Kind::D2(
        width as texture::Size,
        height as texture::Size,
        texture::AaMode::Single,
    );

    let tex = factory.create_texture(
        kind,
        1,
        gfx::memory::Bind::SHADER_RESOURCE,
        gfx::memory::Usage::Dynamic,
        Some(<TexChannel as format::ChannelTyped>::get_channel_type()),
    )?;

    let view =
        factory.view_texture_as_shader_resource::<TexForm>(&tex, (0, 0), format::Swizzle::new())?;

    Ok((tex, view))
}

// Updates a texture with the given data (used for updating the GlyphCache texture)
#[inline]
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
