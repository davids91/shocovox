mod data;
mod pipeline;
mod render;
pub mod types;

pub use crate::octree::raytracing::{SvxRenderApp, Viewport};
use crate::spatial::math::vector::V3cf32;
use encase::UniformBuffer;
use std::sync::Arc;
use winit::window::Window;

impl<'a> SvxRenderApp {
    pub fn new(output_width: u32, output_height: u32) -> Self {
        Self {
            viewport: Viewport::default(),
            output_width,
            output_height,
            texture_extent: wgpu::Extent3d {
                width: output_width,
                height: output_height,
                depth_or_array_layers: 1,
            },
            can_render: false.into(),
            wgpu_instance: wgpu::Instance::default(),
            adapter: None,
            surface: None,
            device: None,
            pipeline: None,
            queue: None,
            dynamic_group: None,
            output_texture: None,
            output_texture_render: None,
            viewport_buffer: None,
        }
    }

    pub fn output_width(&self) -> u32 {
        self.output_width
    }

    pub fn output_height(&self) -> u32 {
        self.output_height
    }

    pub async fn set_output_size(&mut self, width: u32, height: u32, window: Arc<Window>) {
        self.output_width = width;
        self.output_height = height;
        self.rebuild_pipeline(window).await;
    }

    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    pub fn set_viewport(&mut self, viewport: Viewport) {
        if viewport == self.viewport {
            return;
        }

        let mut buffer = UniformBuffer::new(Vec::<u8>::new());
        buffer.write(&viewport).unwrap();
        self.viewport = viewport;
        self.queue
            .as_ref()
            .expect("Expected SvxRenderApp to have a vaild rendering queue!")
            .write_buffer(
                self.viewport_buffer
                    .as_ref()
                    .expect("Expected SvxRenderApp to have a vaild Viewport buffer!"),
                0,
                &buffer.into_inner(),
            )
    }

    pub fn set_viewport_origin(&mut self, origin: V3cf32) {
        self.viewport.origin = origin;
        let mut buffer = UniformBuffer::new(Vec::<u8>::new());
        buffer.write(&self.viewport).unwrap();
        self.queue
            .as_ref()
            .expect("Expected SvxRenderApp to have a vaild rendering queue!")
            .write_buffer(
                self.viewport_buffer
                    .as_ref()
                    .expect("Expected SvxRenderApp to have a vaild Viewport buffer!"),
                0,
                &buffer.into_inner(),
            )
    }

    pub fn update_viewport_origin(&mut self, delta: V3cf32) {
        self.viewport.origin = self.viewport.origin + delta;
        let mut buffer = UniformBuffer::new(Vec::<u8>::new());
        buffer.write(&self.viewport).unwrap();
        self.queue
            .as_ref()
            .expect("Expected SvxRenderApp to have a vaild rendering queue!")
            .write_buffer(
                self.viewport_buffer
                    .as_ref()
                    .expect("Expected SvxRenderApp to have a vaild Viewport buffer!"),
                0,
                &buffer.into_inner(),
            )
    }

    pub fn update_viewport_direction(&mut self, direction: V3cf32) {
        self.viewport.direction = direction;
        let mut buffer = UniformBuffer::new(Vec::<u8>::new());
        buffer.write(&self.viewport).unwrap();
        self.queue
            .as_ref()
            .expect("Expected SvxRenderApp to have a vaild rendering queue!")
            .write_buffer(
                self.viewport_buffer
                    .as_ref()
                    .expect("Expected SvxRenderApp to have a vaild Viewport buffer!"),
                0,
                &buffer.into_inner(),
            )
    }

    pub fn update_viewport_glass_width(&mut self, width: f32) {
        self.viewport.w_h_fov.x = width;
        let mut buffer = UniformBuffer::new(Vec::<u8>::new());
        buffer.write(&self.viewport).unwrap();
        self.queue
            .as_ref()
            .expect("Expected SvxRenderApp to have a vaild rendering queue!")
            .write_buffer(
                self.viewport_buffer
                    .as_ref()
                    .expect("Expected SvxRenderApp to have a vaild Viewport buffer!"),
                0,
                &buffer.into_inner(),
            )
    }

    pub fn update_viewport_glass_height(&mut self, height: f32) {
        self.viewport.w_h_fov.y = height;
        let mut buffer = UniformBuffer::new(Vec::<u8>::new());
        buffer.write(&self.viewport).unwrap();
        self.queue
            .as_ref()
            .expect("Expected SvxRenderApp to have a vaild rendering queue!")
            .write_buffer(
                self.viewport_buffer
                    .as_ref()
                    .expect("Expected SvxRenderApp to have a vaild Viewport buffer!"),
                0,
                &buffer.into_inner(),
            )
    }

    pub fn update_viewport_glass_fov(&mut self, fov: f32) {
        self.viewport.w_h_fov.z = fov;
        let mut buffer = UniformBuffer::new(Vec::<u8>::new());
        buffer.write(&self.viewport).unwrap();
        self.queue
            .as_ref()
            .expect("Expected SvxRenderApp to have a vaild rendering queue!")
            .write_buffer(
                self.viewport_buffer
                    .as_ref()
                    .expect("Expected SvxRenderApp to have a vaild Viewport buffer!"),
                0,
                &buffer.into_inner(),
            )
    }
}
