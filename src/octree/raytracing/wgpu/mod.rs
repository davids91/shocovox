mod data;
mod pipeline;
mod render;
pub mod types;

pub use crate::octree::raytracing::{SvxRenderBackend, Viewport};
use crate::octree::Octree;
use crate::octree::VoxelData;
use crate::spatial::math::vector::V3cf32;
use encase::UniformBuffer;
use std::sync::Arc;
use winit::window::Window;

impl<'a> SvxRenderBackend {
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
            wgpu_instance: wgpu::Instance::default(),
            adapter: None,
            surface: None,
            device: None,
            compute_pipeline: None,
            render_pipeline: None,
            queue: None,
            dynamic_group: None,
            output_texture: None,
            output_texture_render: None,
            viewport_buffer: None,
            metadata_buffer: None,
            nodes_buffer: None,
            children_buffer: None,
            voxels_buffer: None,
            tree_group: None,
        }
    }

    pub fn output_width(&self) -> u32 {
        self.output_width
    }

    pub fn output_height(&self) -> u32 {
        self.output_height
    }

    pub async fn set_output_size<T, const DIM: usize>(
        &mut self,
        width: u32,
        height: u32,
        window: Arc<Window>,
        tree: Option<&Octree<T, DIM>>,
    ) where
        T: Default + Clone + VoxelData,
    {
        self.output_width = width;
        self.output_height = height;
        self.texture_extent = wgpu::Extent3d {
            width: self.output_width,
            height: self.output_height,
            depth_or_array_layers: 1,
        };
        self.viewport_buffer = None;
        self.metadata_buffer = None;
        self.nodes_buffer = None;
        self.children_buffer = None;
        self.voxels_buffer = None;
        self.rebuild_pipeline(window, tree).await;
    }

    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    pub fn with_viewport(mut self, viewport: Viewport) -> Self {
        if viewport == self.viewport {
            return self;
        }

        self.viewport = viewport;

        if let Some(viewport_buffer) = &self.viewport_buffer {
            let mut buffer = UniformBuffer::new(Vec::<u8>::new());
            buffer.write(&self.viewport).unwrap();
            self.queue
                .as_ref()
                .expect("Expected SvxRenderApp to have a vaild rendering queue!")
                .write_buffer(&viewport_buffer, 0, &buffer.into_inner())
        }
        self
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

    pub fn update_viewport_direction(&mut self, delta: V3cf32) {
        self.viewport.direction = (self.viewport.direction + delta).normalized();
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

    pub fn set_viewport_direction(&mut self, direction: V3cf32) {
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
