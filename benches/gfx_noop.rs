extern crate gfx_core;

use gfx_core::{IndexType, Resources, VertexCount};
use gfx_core::{command, pso, shade, state, target, texture};

pub struct NoopCommandBuffer;
impl<R: Resources> command::Buffer<R> for NoopCommandBuffer {
    fn reset(&mut self) {}
    fn bind_pipeline_state(&mut self, _: R::PipelineStateObject) {}
    fn bind_vertex_buffers(&mut self, _: pso::VertexBufferSet<R>) {}
    fn bind_constant_buffers(&mut self, _: &[pso::ConstantBufferParam<R>]) {}
    fn bind_global_constant(&mut self, _: shade::Location, _: shade::UniformValue) {}
    fn bind_resource_views(&mut self, _: &[pso::ResourceViewParam<R>]) {}
    fn bind_unordered_views(&mut self, _: &[pso::UnorderedViewParam<R>]) {}
    fn bind_samplers(&mut self, _: &[pso::SamplerParam<R>]) {}
    fn bind_pixel_targets(&mut self, _: pso::PixelTargetSet<R>) {}
    fn bind_index(&mut self, _: R::Buffer, _: IndexType) {}
    fn set_scissor(&mut self, _: target::Rect) {}
    fn set_ref_values(&mut self, _: state::RefValues) {}
    fn copy_buffer(&mut self, _: R::Buffer, _: R::Buffer, _: usize, _: usize, _: usize) {}
    fn copy_buffer_to_texture(
        &mut self,
        _: R::Buffer,
        _: usize,
        _: texture::TextureCopyRegion<R::Texture>,
    ) {
    }
    fn copy_texture_to_buffer(
        &mut self,
        _: texture::TextureCopyRegion<R::Texture>,
        _: R::Buffer,
        _: usize,
    ) {
    }
    fn update_buffer(&mut self, _: R::Buffer, _: &[u8], _: usize) {}
    fn update_texture(&mut self, _: texture::TextureCopyRegion<R::Texture>, _: &[u8]) {}
    fn generate_mipmap(&mut self, _: R::ShaderResourceView) {}
    fn clear_color(&mut self, _: R::RenderTargetView, _: command::ClearColor) {}
    fn clear_depth_stencil(
        &mut self,
        _: R::DepthStencilView,
        _: Option<target::Depth>,
        _: Option<target::Stencil>,
    ) {
    }
    fn call_draw(&mut self, _: VertexCount, _: VertexCount, _: Option<command::InstanceParams>) {}
    fn call_draw_indexed(
        &mut self,
        _: VertexCount,
        _: VertexCount,
        _: VertexCount,
        _: Option<command::InstanceParams>,
    ) {
    }
    fn copy_texture_to_texture(
        &mut self,
        _: texture::TextureCopyRegion<R::Texture>,
        _: texture::TextureCopyRegion<R::Texture>,
    ) {
    }
}
