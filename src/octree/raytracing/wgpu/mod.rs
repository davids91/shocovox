mod data;
mod pipeline;
mod render;
mod types;

pub use crate::octree::raytracing::wgpu::types::{SvxRenderApp, Viewport};
use crate::octree::Octree;
use encase::UniformBuffer;
use std::sync::Arc;

use wgpu::util::DeviceExt;

impl SvxRenderApp {
    pub fn new(output_width: u32, output_height: u32) -> Self {
        let mut result = Self::default();
        result.output_width = output_width;
        result.output_height = output_height;
        result.wgpu_instance = wgpu::Instance::default();
        result.texture_extent = wgpu::Extent3d {
            width: output_width,
            height: output_height,
            depth_or_array_layers: 1,
        };

        result
    }

    pub fn update_viewport(&mut self, viewport: Viewport) {
        if viewport == self.viewport && self.viewport_buffer.is_some() {
            return;
        }

        let mut buffer = UniformBuffer::new(Vec::<u8>::new());
        buffer.write(&viewport).unwrap();
        self.viewport = viewport;
        self.viewport_buffer = Some(
            self.device
                .as_ref()
                .expect("Expected SvxRenderApp to have a vaild device!")
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Octree Metadata Buffer"),
                    contents: &buffer.into_inner(),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                }),
        );
    }

    // pub fn update_data(&mut self, tree: Arc<Octree<T, DIM>>) {}
}
