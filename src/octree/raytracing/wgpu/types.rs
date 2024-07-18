use crate::octree::Albedo;
use crate::spatial::math::vector::V3cf32;
use encase::ShaderType;
use wgpu::{
    Adapter, BindGroup, Buffer, ComputePipeline, Device, Queue, RenderPipeline, Surface, Texture,
};

#[derive(ShaderType)]
pub(crate) struct Voxelement {
    pub(crate) albedo: Albedo,
    pub(crate) content: u32,
}

#[derive(ShaderType)]
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

#[derive(ShaderType)]
pub struct OctreeMetaData {
    pub(crate) octree_size: u32,
    pub(crate) voxel_brick_dim: u32,
    pub ambient_light_color: V3cf32,
    pub ambient_light_position: V3cf32,
}

#[derive(ShaderType, PartialEq)]
pub struct Viewport {
    pub origin: V3cf32,
    pub direction: V3cf32,
    pub w_h_fov: V3cf32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            origin: [0., 0., -1.].into(),
            direction: [0., 0., 1.].into(),
            w_h_fov: [1.5, 1., 45.].into(),
        }
    }
}

pub struct SvxRenderBackend {
    //render data and parameters
    pub(crate) viewport: Viewport,
    pub(crate) output_width: u32,
    pub(crate) output_height: u32,
    pub(crate) texture_extent: wgpu::Extent3d,

    // wgpu pipeline
    pub(crate) wgpu_instance: wgpu::Instance,
    pub(crate) adapter: Option<Adapter>,
    pub(crate) surface: Option<Surface<'static>>,
    pub(crate) device: Option<Device>,
    pub(crate) compute_pipeline: Option<ComputePipeline>,
    pub(crate) render_pipeline: Option<RenderPipeline>,
    pub(crate) queue: Option<Queue>,

    //layouts, textures and buffers
    pub(crate) dynamic_group: Option<BindGroup>,
    pub(crate) tree_group: Option<BindGroup>,
    pub(crate) output_texture: Option<Texture>,
    pub(crate) output_texture_render: Option<Texture>,
    pub(crate) viewport_buffer: Option<Buffer>,
    pub(crate) metadata_buffer: Option<Buffer>,
    pub(crate) nodes_buffer: Option<Buffer>,
    pub(crate) children_buffer: Option<Buffer>,
    pub(crate) voxels_buffer: Option<Buffer>,
}

#[cfg(test)]
mod types_wgpu_byte_compatibility_tests {
    use super::{OctreeMetaData, SizedNode, Viewport, Voxelement};
    use encase::ShaderType;

    #[test]
    fn test_wgpu_compatibility() {
        Viewport::assert_uniform_compat();
        OctreeMetaData::assert_uniform_compat();
        Voxelement::assert_uniform_compat();
        SizedNode::assert_uniform_compat();
    }
}
