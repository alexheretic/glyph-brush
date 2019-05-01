use super::TexFormView;
use gfx::{
    self,
    format::{Format, Formatted},
    handle::{DepthStencilView, RawDepthStencilView, RawRenderTargetView, RenderTargetView},
    memory::Typed,
    pso::*,
    *,
};
use gfx_core::pso;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawDepthTarget {
    active: bool,
}

impl DataLink<'_> for RawDepthTarget {
    type Init = Option<(format::Format, state::Depth)>;
    fn new() -> Self {
        RawDepthTarget { active: false }
    }
    fn is_active(&self) -> bool {
        self.active
    }
    fn link_depth_stencil(&mut self, init: &Self::Init) -> Option<pso::DepthStencilDesc> {
        self.active = init.is_some();

        init.map(|(format, depth)| (format, depth.into()))
    }
}

impl<R: Resources> DataBind<R> for RawDepthTarget {
    type Data = Option<handle::RawDepthStencilView<R>>;

    fn bind_to(
        &self,
        out: &mut RawDataSet<R>,
        data: &Self::Data,
        man: &mut handle::Manager<R>,
        _: &mut AccessInfo<R>,
    ) {
        if let Some(dsv) = data {
            out.pixel_targets.add_depth_stencil(
                man.ref_dsv(dsv),
                true,
                false,
                dsv.get_dimensions(),
            );
        }
    }
}

gfx_defines! {
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

impl glyph_pipe::Init<'_> {
    pub fn new(
        color_format: format::Format,
        depth_format: Option<format::Format>,
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
            out_depth: depth_format.map(|d| (d, depth_test)),
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
    #[inline]
    fn as_raw(&self) -> &Self::Raw {
        self.raw()
    }
    #[inline]
    fn format(&self) -> Format {
        T::get_format()
    }
}

impl<R: Resources, T: Formatted> RawAndFormat for DepthStencilView<R, T> {
    type Raw = RawDepthStencilView<R>;
    #[inline]
    fn as_raw(&self) -> &Self::Raw {
        self.raw()
    }
    #[inline]
    fn format(&self) -> Format {
        T::get_format()
    }
}

impl<R> RawAndFormat for (&R, Format) {
    type Raw = R;
    #[inline]
    fn as_raw(&self) -> &Self::Raw {
        self.0
    }
    #[inline]
    fn format(&self) -> Format {
        self.1
    }
}

pub trait IntoDimensions {
    /// Returns (width, height)
    fn into_dimensions(self) -> (f32, f32);
}

impl<R, CV> IntoDimensions for &CV
where
    R: Resources,
    CV: RawAndFormat<Raw = RawRenderTargetView<R>>,
{
    #[inline]
    fn into_dimensions(self) -> (f32, f32) {
        let (width, height, ..) = self.as_raw().get_dimensions();
        (f32::from(width), f32::from(height))
    }
}

impl<T: Into<f32>> IntoDimensions for [T; 2] {
    #[inline]
    fn into_dimensions(self) -> (f32, f32) {
        let [w, h] = self;
        (w.into(), h.into())
    }
}

impl<T: Into<f32>> IntoDimensions for (T, T) {
    #[inline]
    fn into_dimensions(self) -> (f32, f32) {
        (self.0.into(), self.1.into())
    }
}
