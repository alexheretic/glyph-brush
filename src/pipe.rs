use super::*;
use gfx::*;
use gfx::pso::*;
use gfx_core::pso;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawDepthTarget;

impl<'a> DataLink<'a> for RawDepthTarget {
    type Init = (format::Format, state::Depth);
    fn new() -> Self { RawDepthTarget }
    fn is_active(&self) -> bool { true }
    fn link_depth_stencil(&mut self, init: &Self::Init) -> Option<pso::DepthStencilDesc> {
        Some((init.0, init.1.into()))
    }
}

impl<R: Resources> DataBind<R> for RawDepthTarget {
    type Data = handle::RawDepthStencilView<R>;
    fn bind_to(&self,
               out: &mut RawDataSet<R>,
               data: &Self::Data,
               man: &mut handle::Manager<R>,
               _: &mut AccessInfo<R>) {
        let dsv = data;
        out.pixel_targets.add_depth_stencil(man.ref_dsv(dsv), true, false, dsv.get_dimensions());
    }
}

gfx_defines!{
    vertex GlyphVertex {
        pos: [f32; 3] = "pos",
        tex_pos: [f32; 2] = "tex_pos",
        color: [f32; 4] = "color",
    }
}

gfx_pipeline_base!( glyph_pipe {
    vbuf: VertexBuffer<GlyphVertex>,
    font_tex: gfx::pso::resource::TextureSampler<TexFormView>,
    transform: Global<[[f32; 4]; 4]>,
    out: RawRenderTarget,
    out_depth: RawDepthTarget,
});

impl<'a> glyph_pipe::Init<'a> {
    pub fn new(
        color_format: format::Format,
        depth_format: format::Format,
        depth_test: state::Depth)
        -> Self
    {
        glyph_pipe::Init {
            vbuf: (),
            font_tex: "font_tex",
            transform: "transform",
            out: ("Target0", color_format, state::ColorMask::all(), Some(preset::blend::ALPHA)),
            out_depth: (depth_format, depth_test),
        }
    }
}
