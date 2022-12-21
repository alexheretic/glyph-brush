use crate::{
    default_transform, GlyphBrush, RawAndFormat, RawDepthStencilView, RawRenderTargetView,
};
use glyph_brush::ab_glyph::*;
use std::{hash::BuildHasher, marker::PhantomData};

/// Short-lived builder for drawing glyphs, constructed from [`GlyphBrush::use_queue`](struct.GlyphBrush.html#method.use_queue).
///
/// # Example
///
/// ```no_run
/// # fn main() -> Result<(), String> {
/// # let glyph_brush: gfx_glyph::GlyphBrush<gfx_device_gl::Resources, gfx_device_gl::Factory> = unimplemented!();
/// # let gfx_color: gfx_core::handle::RenderTargetView<gfx_device_gl::Resources, gfx::format::Srgba8> = unimplemented!();
/// # let gfx_depth: gfx_core::handle::DepthStencilView<gfx_device_gl::Resources, gfx::format::Depth> = unimplemented!();
/// # let factory: gfx_device_gl::Factory = unimplemented!();
/// # let gfx_encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
///
/// glyph_brush
///     .use_queue()
///     .depth_target(&gfx_depth)
///     .draw(&mut gfx_encoder, &gfx_color)?;
/// # Ok(()) }
/// ```
#[must_use]
pub struct DrawBuilder<'a, F, R: gfx::Resources, GF: gfx::Factory<R>, H, DV> {
    pub(crate) brush: &'a mut GlyphBrush<R, GF, F, H>,
    pub(crate) transform: Option<[[f32; 4]; 4]>,
    pub(crate) depth_target: Option<&'a DV>,
}

impl<'a, F, R, GF, H, DV> DrawBuilder<'a, F, R, GF, H, DV>
where
    F: Font,
    R: gfx::Resources,
    GF: gfx::Factory<R>,
    H: BuildHasher,
{
    /// Use a custom position transform (e.g. a projection) replacing the [`default_transform`](fn.default_transform.html).
    ///
    /// # Example
    /// ```no_run
    /// # fn main() -> Result<(), String> {
    /// # let glyph_brush: gfx_glyph::GlyphBrush<gfx_device_gl::Resources, gfx_device_gl::Factory> = unimplemented!();
    /// # let gfx_color: gfx_core::handle::RenderTargetView<gfx_device_gl::Resources, gfx::format::Srgba8> = unimplemented!();
    /// let projection = gfx_glyph::default_transform(&gfx_color);
    ///
    /// glyph_brush.use_queue().transform(projection)
    /// # ;
    /// # Ok(()) }
    /// ```
    #[inline]
    pub fn transform<M: Into<[[f32; 4]; 4]>>(mut self, transform: M) -> Self {
        self.transform = Some(transform.into());
        self
    }

    /// Set a depth buffer target to perform depth testing against.
    ///
    /// # Example
    /// ```no_run
    /// # fn main() -> Result<(), String> {
    /// # let glyph_brush: gfx_glyph::GlyphBrush<gfx_device_gl::Resources, gfx_device_gl::Factory> = unimplemented!();
    /// # let gfx_depth: gfx_core::handle::DepthStencilView<gfx_device_gl::Resources, gfx::format::Depth> = unimplemented!();
    /// glyph_brush.use_queue().depth_target(&gfx_depth)
    /// # ;
    /// # Ok(()) }
    /// ```
    ///
    /// # Raw usage
    /// Can also be used with gfx raw depth views if necessary. The `Format` must also be provided.
    /// ```no_run
    /// # use gfx::format::{self, Formatted};
    /// # use gfx::memory::Typed;
    /// # fn main() -> Result<(), String> {
    /// # let glyph_brush: gfx_glyph::GlyphBrush<gfx_device_gl::Resources, gfx_device_gl::Factory> = unimplemented!();
    /// # let gfx_depth: gfx_core::handle::DepthStencilView<gfx_device_gl::Resources, gfx::format::Depth> = unimplemented!();
    /// # let raw_depth_view = gfx_depth.raw();
    /// glyph_brush
    ///     .use_queue()
    ///     .depth_target(&(raw_depth_view, format::Depth::get_format()))
    /// # ;
    /// # Ok(()) }
    /// ```
    #[inline]
    pub fn depth_target<D>(self, depth: &'a D) -> DrawBuilder<'a, F, R, GF, H, D> {
        let Self {
            brush, transform, ..
        } = self;
        DrawBuilder {
            depth_target: Some(depth),
            brush,
            transform,
        }
    }
}

impl<'a, F, R, GF, H, DV> DrawBuilder<'a, F, R, GF, H, DV>
where
    F: Font + Sync,
    R: gfx::Resources,
    GF: gfx::Factory<R>,
    H: BuildHasher,
    DV: RawAndFormat<Raw = RawDepthStencilView<R>>,
{
    /// Draws all queued sections onto a render target.
    /// See [`queue`](struct.GlyphBrush.html#method.queue).
    ///
    /// Trims the cache, see [caching behaviour](#caching-behaviour).
    ///
    /// # Example
    /// ```no_run
    /// # fn main() -> Result<(), String> {
    /// # let glyph_brush: gfx_glyph::GlyphBrush<gfx_device_gl::Resources, gfx_device_gl::Factory> = unimplemented!();
    /// # let gfx_color: gfx_core::handle::RenderTargetView<gfx_device_gl::Resources, gfx::format::Srgba8> = unimplemented!();
    /// # let factory: gfx_device_gl::Factory = unimplemented!();
    /// # let gfx_encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    /// glyph_brush.use_queue().draw(&mut gfx_encoder, &gfx_color)?;
    /// # Ok(()) }
    /// ```
    ///
    /// # Raw usage
    /// Can also be used with gfx raw render views if necessary. The `Format` must also be provided.
    /// ```no_run
    /// # use gfx::format::{self, Formatted};
    /// # use gfx::memory::Typed;
    /// # fn main() -> Result<(), String> {
    /// # let glyph_brush: gfx_glyph::GlyphBrush<gfx_device_gl::Resources, gfx_device_gl::Factory> = unimplemented!();
    /// # let gfx_color: gfx_core::handle::RenderTargetView<gfx_device_gl::Resources, gfx::format::Srgba8> = unimplemented!();;
    /// # let factory: gfx_device_gl::Factory = unimplemented!();
    /// # let gfx_encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    /// # let raw_render_view = gfx_color.raw();
    /// glyph_brush.use_queue().draw(
    ///     &mut gfx_encoder,
    ///     &(raw_render_view, format::Srgba8::get_format()),
    /// )?;
    /// # Ok(()) }
    /// ```
    #[inline]
    pub fn draw<C, CV>(self, encoder: &mut gfx::Encoder<R, C>, target: &CV) -> Result<(), String>
    where
        C: gfx::CommandBuffer<R>,
        CV: RawAndFormat<Raw = RawRenderTargetView<R>>,
    {
        let Self {
            brush,
            transform,
            depth_target,
        } = self;
        let transform = transform.unwrap_or_else(|| default_transform(target));
        brush.draw(transform, encoder, target, depth_target)
    }
}

struct NoDepth<R: gfx::Resources>(PhantomData<R>);
impl<R: gfx::Resources> RawAndFormat for NoDepth<R> {
    type Raw = RawDepthStencilView<R>;
    fn as_raw(&self) -> &Self::Raw {
        unreachable!()
    }
    fn format(&self) -> gfx::format::Format {
        unreachable!()
    }
}

impl<'a, F, R, GF, H> DrawBuilder<'a, F, R, GF, H, ()>
where
    F: Font + Sync,
    R: gfx::Resources,
    GF: gfx::Factory<R>,
    H: BuildHasher,
{
    #[inline]
    pub fn draw<C, CV>(self, encoder: &mut gfx::Encoder<R, C>, target: &CV) -> Result<(), String>
    where
        C: gfx::CommandBuffer<R>,
        CV: RawAndFormat<Raw = RawRenderTargetView<R>>,
    {
        let Self {
            brush, transform, ..
        } = self;
        let transform = transform.unwrap_or_else(|| default_transform(target));
        brush.draw::<C, CV, NoDepth<R>>(transform, encoder, target, None)
    }
}
