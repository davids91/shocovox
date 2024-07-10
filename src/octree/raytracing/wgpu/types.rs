use bytemuck::{Pod, Zeroable};

use std::sync::Arc;
use wgpu::{
    Adapter, BindGroup, BindGroupLayout, Buffer, Device, Queue, RenderPipeline, Surface, Texture,
};
use winit::window::Window;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Voxelement {
    pub(crate) albedo: [f32; 4],
    pub(crate) content: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct SizedNode {
    /// Composite field:
    /// - Byte 1: Boolean value, true in case node is a leaf
    /// - In case of internal nodes:
    ///   - Byte 2: TBD
    ///   - Byte 3: TBD
    ///   - Byte 4: Lvl2 Occupancy bitmap
    /// - In case of leaf nodes:
    ///   - Byte 2: TBD
    ///   - Byte 3: TBD
    ///   - Byte 4: TBD
    pub(crate) sized_node_meta: u32,

    /// index of where the data about this node is found in children_buffer
    /// - In case of internal nodes:
    ///   - 8 Index value of node children
    /// - In case of leaf nodes:
    ///   - Byte 1-4: Occupancy bitmap MSB
    ///   - Byte 5-8: Occupancy bitmap LSB
    ///   - Byte 9-12: TBD
    ///   - Byte 13-16: TBD
    ///   - Byte 17-20: TBD
    ///   - Byte 21-24: TBD
    ///   - Byte 25-28: TBD
    ///   - Byte 29-32: TBD
    pub(crate) children_start_at: u32,

    /// index of where the voxel values contained in the node start inside the voxels buffer,
    /// or a "none_value". Should the field contain an index, the next voxel_brick_dim^3 elements
    /// inside the @voxels array count as part of the voxels associated with the node
    pub(crate) voxels_start_at: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct OctreeMetaData {
    pub(crate) octree_size: u32,
    pub(crate) voxel_brick_dim: u32,
    pub ambient_light_color: [f32; 3],
    pub ambient_light_position: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, PartialEq)]
pub struct Viewport {
    pub origin: [f32; 3],
    pub direction: [f32; 3],
    pub size: [f32; 2],
    pub fov: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            origin: [0., 0., -1.],
            direction: [0., 0., 1.],
            size: [1.5, 1.],
            fov: 45.,
        }
    }
}

#[derive(Default)] //TODO: This isn't good to be exposed
pub struct SvxRenderApp {
    //render data and parameters
    pub(crate) output_width: u32,
    pub(crate) output_height: u32,
    pub(crate) texture_extent: wgpu::Extent3d,
    pub viewport: Viewport,

    // wgpu pipeline
    pub(crate) wgpu_instance: wgpu::Instance,
    pub(crate) adapter: Option<Adapter>,
    pub(crate) window: Option<Arc<Window>>,
    pub(crate) surface: Option<Surface<'static>>,
    pub(crate) device: Option<Device>,
    pub(crate) pipeline: Option<RenderPipeline>,
    pub(crate) queue: Option<Queue>,
    //layouts, textures and buffers
    pub(crate) dynamic_group: Option<BindGroup>,
    pub(crate) tree_group: Option<BindGroup>,
    pub(crate) output_texture: Option<Texture>,
    pub(crate) output_texture_render: Option<Texture>,
    pub(crate) viewport_buffer: Option<Buffer>,
    // pub(crate) metadata_buffer: Option<Buffer>,
    // pub(crate) nodes_buffer: Option<Buffer>,
    // pub(crate) children_buffer: Option<Buffer>,
    // pub(crate) voxels_buffer: Option<Buffer>,
}
