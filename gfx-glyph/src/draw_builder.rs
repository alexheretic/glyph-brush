use crate::{
    default_transform, GlyphBrush, RawAndFormat, RawDepthStencilView, RawRenderTargetView,
};
use std::{hash::BuildHasher, marker::PhantomData};

#[must_use]
pub struct DrawBuilder<'a, 'font, R: gfx::Resources, F: gfx::Factory<R>, H, DV> {
    pub(crate) brush: &'a mut GlyphBrush<'font, R, F, H>,
    pub(crate) transform: Option<[[f32; 4]; 4]>,
    pub(crate) depth_target: Option<&'a DV>,
}

impl<'a, 'font, R: gfx::Resources, F: gfx::Factory<R>, H: BuildHasher, DV>
    DrawBuilder<'a, 'font, R, F, H, DV>
{
    #[inline]
    pub fn transform<M: Into<[[f32; 4]; 4]>>(mut self, transform: M) -> Self {
        self.transform = Some(transform.into());
        self
    }

    #[inline]
    pub fn depth_target<D>(self, depth: &'a D) -> DrawBuilder<'a, 'font, R, F, H, D> {
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

impl<
        'a,
        R: gfx::Resources,
        F: gfx::Factory<R>,
        H: BuildHasher,
        DV: RawAndFormat<Raw = RawDepthStencilView<R>>,
    > DrawBuilder<'a, '_, R, F, H, DV>
{
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

impl<'a, R: gfx::Resources, F: gfx::Factory<R>, H: BuildHasher> DrawBuilder<'a, '_, R, F, H, ()> {
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
