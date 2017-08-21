#[cfg(test)] extern crate pretty_env_logger;
#[cfg(test)] #[macro_use] extern crate approx;

#[macro_use] extern crate log;
#[macro_use] extern crate gfx;
extern crate gfx_core;
extern crate rusttype;
extern crate unicode_normalization;
extern crate ordered_float;

mod section;
mod layout;

use gfx::traits::FactoryExt;
use rusttype::{FontCollection, point, vector};
use rusttype::gpu_cache::Cache;
use gfx::{handle, texture, format, preset, state};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use gfx_core::memory::Typed;
use std::i32;
use std::error::Error;
use std::collections::{HashMap, HashSet};

pub use section::*;
pub use layout::*;

pub type Font<'a> = rusttype::Font<'a>;
pub type Scale = rusttype::Scale;
pub type Rect<T> = rusttype::Rect<T>;
pub type Point<T> = rusttype::Point<T>;
pub type PositionedGlyph = rusttype::PositionedGlyph<'static>;

// Type for the generated glyph cache texture
type TexForm = format::U8Norm;
type TexSurface = <TexForm as format::Formatted>::Surface;
type TexChannel = <TexForm as format::Formatted>::Channel;
type TexFormView = <TexForm as format::Formatted>::View;

const FONT_CACHE_ID: usize = 0;
const FONT_CACHE_POSITION_TOLERANCE: f32 = 1.0;
const FONT_CACHE_SCALE_TOLERANCE: f32 = 0.5;

gfx_defines!{
    vertex GlyphVertex {
        pos: [f32; 2] = "pos",
        tex_pos: [f32; 2] = "tex_pos",
        color: [f32; 4] = "color",
    }
}

gfx_pipeline_base!( glyph_pipe {
    vbuf: gfx::VertexBuffer<GlyphVertex>,
    font_tex: gfx::TextureSampler<TexFormView>,
    out: gfx::RawRenderTarget,
});

impl<'a> glyph_pipe::Init<'a> {
    fn using_format(format: gfx::format::Format) -> Self {
        glyph_pipe::Init {
            vbuf: (),
            font_tex: "font_tex",
            out: ("Target0", format, state::ColorMask::all(), Some(preset::blend::ALPHA))
        }
    }
}

fn hash<H: Hash>(hashable: &H) -> u64 {
    let mut s = DefaultHasher::new();
    hashable.hash(&mut s);
    s.finish()
}

pub struct GlyphBrush<'a, R: gfx::Resources, F: gfx::Factory<R>>{
    font: rusttype::Font<'a>,
    font_cache: rusttype::gpu_cache::Cache,
    font_cache_tex: (gfx::handle::Texture<R, TexSurface>, gfx_core::handle::ShaderResourceView<R, f32>),
    factory: F,
    draw_cache: Option<DrawnGlyphBrush<R>>,

    // cache of section-layout hash -> computed glyphs, this avoid repeated glyph computation
    // for identical layout/sections common to repeated frame rendering
    calculate_glyph_cache: HashMap<u64, GlyphedSection>,

    // buffer of section-layout hashs (that must exist in the calculate_glyph_cache)
    // to be rendered on the next `draw_queued` call
    section_buffer: Vec<u64>,
}

impl<'font, R: gfx::Resources, F: gfx::Factory<R>> GlyphBrush<'font, R, F> {

    pub fn pixel_bounding_box<'a, S, L>(&self, section: S, layout: &L)
        -> Rect<i32>
        where L: GlyphPositioner + Hash,
              S: Into<Section<'a>>,
    {
        let section = section.into();
        let mut x = (i32::MAX, i32::MIN);
        let mut y = (i32::MAX, i32::MIN);
        let mut no_match = true;
        for g in layout.calculate_glyphs(&self.font, &section) {
            no_match = false;
            if let Some(Rect{ min, max }) = g.pixel_bounding_box() {
                if min.x < x.0 { x.0 = min.x; }
                if min.y < y.0 { y.0 = min.y; }
                if max.x > x.1 { x.1 = max.x; }
                if max.y > y.1 { y.1 = max.y; }
            }
        }

        if no_match {
            Rect {
                min: Point { x: 0, y: 0 },
                max: Point { x: 1, y: 1 },
            }
        }
        else {
            Rect {
                min: Point { x: x.0, y: y.0 },
                max: Point { x: x.1, y: y.1 },
            }
        }
    }

    pub fn queue<'a, S, L>(&mut self, section: S, layout: &L)
        where L: GlyphPositioner,
              S: Into<Section<'a>>,
    {
        let section = section.into();
        let section_hash = hash(&(&section, layout));

        if !self.calculate_glyph_cache.contains_key(&section_hash) {
            let glyphed = GlyphedSection {
                color: section.color,
                bounds: layout.bounds_rect(&section),
                glyphs: layout.calculate_glyphs(&self.font, &section),
            };
            self.calculate_glyph_cache.insert(section_hash, glyphed);
        }

        self.section_buffer.push(section_hash);
    }

    pub fn draw_queued<C, T>(
        &mut self,
        mut encoder: &mut gfx::Encoder<R, C>,
        target: &gfx::handle::RenderTargetView<R, T>)
        -> Result<(), String>
        where C: gfx::CommandBuffer<R>,
              T: format::RenderFormat,
    {
        let (screen_width, screen_height, _, _) = target.get_dimensions();
        let (screen_width, screen_height) = (screen_width as u32, screen_height as u32);

        let current_text_state = hash(&self.section_buffer);

        if self.draw_cache.is_none() ||
            self.draw_cache.as_ref().unwrap().texture_updated ||
            self.draw_cache.as_ref().unwrap().last_text_state != current_text_state
        {
            loop {
                let mut no_text = true;

                for section_hash in &self.section_buffer {
                    let GlyphedSection{ ref glyphs, .. } = self.calculate_glyph_cache[&section_hash];
                    for glyph in glyphs {
                        self.font_cache.queue_glyph(FONT_CACHE_ID, glyph.clone());
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
                    update_texture(&mut encoder, &tex, offset, size, &tex_data);
                }) {
                    let (width, height) = self.font_cache.dimensions();
                    let (new_width, new_height) = (width * 2, height * 2);

                    if let Some(ref mut cache) = self.draw_cache {
                        cache.texture_updated = true;
                    }
                    info!("Increasing glyph texture size {old:?} -> {new:?}, as {reason:?}. \
                        Consider building using `.initial_cache_size{new:?}` to avoid resizing",
                        old = (width, height), new = (new_width, new_height), reason = err);

                    let new_cache = Cache::new(new_width,
                                               new_height,
                                               FONT_CACHE_SCALE_TOLERANCE,
                                               FONT_CACHE_POSITION_TOLERANCE);

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

            let verts: Vec<GlyphVertex> = self.section_buffer.iter()
                .flat_map(|section_hash| {
                    let GlyphedSection{ ref glyphs, color, bounds }
                        = self.calculate_glyph_cache[&section_hash];
                    text_vertices(
                        glyphs,
                        &self.font_cache,
                        bounds,
                        color,
                        (screen_width as f32, screen_height as f32))
                })
                .collect();

            let (vbuf, slice) = self.factory.create_vertex_buffer_with_slice(&verts, ());

            let draw_cache = if let Some(mut cache) = self.draw_cache.take() {
                cache.pipe_data.vbuf = vbuf;
                cache.pipe_data.out = target.raw().clone();
                if cache.pso.0 != T::get_format() {
                    cache.pso = (T::get_format(), self.pso_using(T::get_format()));
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
                            out: target.raw().clone(),
                        }
                    },
                    pso: (T::get_format(), self.pso_using(T::get_format())),
                    slice,
                    last_text_state: 0,
                    texture_updated: false,
                }
            };

            self.draw_cache = Some(draw_cache);
        }

        if let Some(DrawnGlyphBrush{ ref pso, ref slice, ref pipe_data, .. }) = self.draw_cache {
            encoder.draw(slice, &pso.1, pipe_data);
        }

        self.clear_section_buffer();

        Ok(())
    }

    fn clear_section_buffer(&mut self) {
        // clear section_buffer & trim calculate_glyph_cache to active sections
        let mut active = HashSet::with_capacity(self.section_buffer.len());
        for h in self.section_buffer.drain(..) {
            active.insert(h);
        }
        self.calculate_glyph_cache.retain(|key, _| active.contains(key));
    }

    fn pso_using(&mut self, format: gfx::format::Format) -> gfx::PipelineState<R, glyph_pipe::Meta> {
        self.factory.create_pipeline_simple(
            include_bytes!("shader/vert.glsl"),
            include_bytes!("shader/frag.glsl"),
            glyph_pipe::Init::using_format(format)).unwrap()
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
struct GlyphedSection {
    color: [f32; 4],
    bounds: Rect<f32>,
    glyphs: Vec<PositionedGlyph>,
}

#[derive(Debug)]
pub struct GlyphBrushBuilder<'a> {
    font: &'a [u8],
    initial_cache_size: (u32, u32),
    log_perf_stats: Option<&'static str>,
}

impl<'a> GlyphBrushBuilder<'a> {
    pub fn using_font(font: &'a [u8]) -> Self {
        GlyphBrushBuilder {
            font: font,
            initial_cache_size: (256, 256),
            log_perf_stats: None,
        }
    }

    /// Initial size of 2D texture used as a gpu cache, pixels (width, height)
    pub fn initial_cache_size(mut self, size: (u32, u32)) -> Self {
        self.initial_cache_size = size;
        self
    }

    pub fn log_perf_stats(mut self, name: &'static str) -> Self {
        self.log_perf_stats = Some(name);
        self
    }

    pub fn build<R, F>(self, mut factory: F) -> GlyphBrush<'a, R, F>
        where R: gfx::Resources, F: gfx::Factory<R> + Clone
    {
        assert!(!self.font.is_empty(), "Empty font data");
        let font = FontCollection::from_bytes(self.font as &[u8]).into_font()
            .expect("Could not create rusttype::Font");

        let (cache_width, cache_height) = self.initial_cache_size;
        let font_cache_tex = create_texture(&mut factory, cache_width, cache_height).unwrap();

        GlyphBrush {
            font,
            font_cache: Cache::new(cache_width,
                                   cache_height,
                                   FONT_CACHE_SCALE_TOLERANCE,
                                   FONT_CACHE_POSITION_TOLERANCE),
            font_cache_tex,

            factory,
            draw_cache: None,
            section_buffer: Vec::new(),
            calculate_glyph_cache: HashMap::new(),
        }
    }
}

#[inline]
fn text_vertices(glyphs: &[PositionedGlyph],
                 cache: &Cache,
                 bounds: Rect<f32>,
                 color: [f32; 4],
                 (screen_width, screen_height): (f32, f32)) -> Vec<GlyphVertex> {
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
        if let Ok(Some((mut uv_rect, screen_rect))) = cache.rect_for(FONT_CACHE_ID, g) {
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
                    pos: [gl_rect.min.x, gl_rect.max.y],
                    tex_pos: [uv_rect.min.x, uv_rect.max.y],
                    color,
                },
                GlyphVertex {
                    pos: [gl_rect.min.x, gl_rect.min.y],
                    tex_pos: [uv_rect.min.x, uv_rect.min.y],
                    color,
                },
                GlyphVertex {
                    pos: [gl_rect.max.x, gl_rect.min.y],
                    tex_pos: [uv_rect.max.x, uv_rect.min.y],
                    color,
                },
                GlyphVertex {
                    pos: [gl_rect.max.x, gl_rect.min.y],
                    tex_pos: [uv_rect.max.x, uv_rect.min.y],
                    color,
                },
                GlyphVertex {
                    pos: [gl_rect.max.x, gl_rect.max.y],
                    tex_pos: [uv_rect.max.x, uv_rect.max.y],
                    color,
                },
                GlyphVertex {
                    pos: [gl_rect.min.x, gl_rect.max.y],
                    tex_pos: [uv_rect.min.x, uv_rect.max.y],
                    color,
                }]);
        }
    }
    vertices
}

// Creates a gfx texture with the given data
fn create_texture<F, R>(factory: &mut F, width: u32, height: u32)
    -> Result<(handle::Texture<R, TexSurface>, handle::ShaderResourceView<R, TexFormView>),
              Box<Error>>
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
