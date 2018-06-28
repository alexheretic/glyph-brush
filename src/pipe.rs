use super::*;
use gfx::format::{Format, Formatted};
use gfx::handle::{DepthStencilView, RawDepthStencilView, RawRenderTargetView, RenderTargetView};
use gfx::memory::Typed;
use gfx::pso::*;
use gfx::*;
use gfx_core::pso;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawDepthTarget;

impl<'a> DataLink<'a> for RawDepthTarget {
    type Init = (format::Format, state::Depth);
    fn new() -> Self {
        RawDepthTarget
    }
    fn is_active(&self) -> bool {
        true
    }
    fn link_depth_stencil(&mut self, init: &Self::Init) -> Option<pso::DepthStencilDesc> {
        Some((init.0, init.1.into()))
    }
}

impl<R: Resources> DataBind<R> for RawDepthTarget {
    type Data = handle::RawDepthStencilView<R>;
    fn bind_to(
        &self,
        out: &mut RawDataSet<R>,
        data: &Self::Data,
        man: &mut handle::Manager<R>,
        _: &mut AccessInfo<R>,
    ) {
        let dsv = data;
        out.pixel_targets
            .add_depth_stencil(man.ref_dsv(dsv), true, false, dsv.get_dimensions());
    }
}

gfx_defines!{
    vertex GlyphVertex {
        /// screen position
        left_top: [f32; 3] = "left_top",
        right_bottom: [f32; 2] = "right_bottom",
        /// texture position
        tex_left_top: [f32; 2] = "tex_left_top",
        tex_right_bottom: [f32; 2] = "tex_right_bottom",
        /// text color
        color: [f32; 4] = "color",
    }
}

gfx_pipeline_base!( glyph_pipe {
    vbuf: InstanceBuffer<GlyphVertex>,
    font_tex: gfx::pso::resource::TextureSampler<TexFormView>,
    transform: Global<[[f32; 4]; 4]>,
    out: RawRenderTarget,
    out_depth: RawDepthTarget,
});

impl<'a> glyph_pipe::Init<'a> {
    pub fn new(
        color_format: format::Format,
        depth_format: format::Format,
        depth_test: state::Depth,
    ) -> Self {
        glyph_pipe::Init {
            vbuf: (),
            font_tex: "font_tex",
            transform: "transform",
            out: (
                "Target0",
                color_format,
                state::ColorMask::all(),
                Some(preset::blend::ALPHA),
            ),
            out_depth: (depth_format, depth_test),
        }
    }
}

/// A view that can produce an inner "raw" view & a `Format`.
pub trait RawAndFormat {
    type Raw;
    fn as_raw(&self) -> &Self::Raw;
    fn format(&self) -> Format;
}

impl<R: Resources, T: Formatted> RawAndFormat for RenderTargetView<R, T> {
    type Raw = RawRenderTargetView<R>;
    fn as_raw(&self) -> &Self::Raw {
        self.raw()
    }

    fn format(&self) -> Format {
        T::get_format()
    }
}

impl<R: Resources, T: Formatted> RawAndFormat for DepthStencilView<R, T> {
    type Raw = RawDepthStencilView<R>;
    fn as_raw(&self) -> &Self::Raw {
        self.raw()
    }

    fn format(&self) -> Format {
        T::get_format()
    }
}

impl<'a, R> RawAndFormat for (&'a R, Format) {
    type Raw = R;
    fn as_raw(&self) -> &Self::Raw {
        self.0
    }

    fn format(&self) -> Format {
        self.1
    }
}
