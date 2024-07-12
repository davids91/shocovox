use crate::octree::{Albedo, V3cf32, VoxelData};
use bevy::{
    asset::Handle,
    ecs::system::Resource,
    reflect::TypePath,
    render::{
        extract_resource::ExtractResource,
        prelude::Image,
        render_graph::RenderLabel,
        render_resource::{
            AsBindGroup, BindGroup, BindGroupLayout, CachedComputePipelineId, ShaderType,
        },
    },
};

#[derive(Clone, ShaderType)]
pub(crate) struct Voxelement {
    pub(crate) albedo: Albedo,
    pub(crate) content: u32,
    _padding: V3cf32,
}

impl Voxelement {
    pub fn new(albedo: Albedo, content: u32) -> Self {
        Voxelement {
            albedo,
            content,
            _padding: V3cf32::new(0., 0., 0.),
        }
    }
}

#[derive(Clone, ShaderType)]
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
#[derive(Clone, ShaderType)]
pub struct OctreeMetaData {
    pub ambient_light_color: V3cf32,
    pub(crate) voxel_brick_dim: u32,
    pub ambient_light_position: V3cf32,
    pub(crate) octree_size: u32,
}

#[repr(C)]
#[derive(Clone, Copy, ShaderType)]
pub struct Viewport {
    pub origin: V3cf32,
    pub direction: V3cf32,
    pub w_h_fov: V3cf32,
}

pub struct ShocoVoxRenderPlugin {
    pub resolution: [u32; 2],
}

#[derive(Resource, Clone, AsBindGroup, TypePath, ExtractResource)]
#[type_path = "shocovox::gpu::ShocoVoxViewingGlass"]
pub struct ShocoVoxViewingGlass {
    #[storage_texture(0, image_format = Rgba8Unorm, access = ReadWrite)]
    pub output_texture: Handle<Image>,

    #[uniform(1, visibility(compute))]
    pub viewport: Viewport,
}

#[derive(Resource, Clone, AsBindGroup, TypePath, ExtractResource)]
#[type_path = "shocovox::gpu::ShocoVoxRenderData"]
pub struct ShocoVoxRenderData {
    #[uniform(0, visibility(compute))]
    pub(crate) meta: OctreeMetaData,

    #[storage(1, visibility(compute))]
    pub(crate) nodes: Vec<SizedNode>,

    #[storage(2, visibility(compute))]
    pub(crate) children_buffer: Vec<u32>,

    #[storage(3, visibility(compute))]
    pub(crate) voxels: Vec<Voxelement>,
}

#[derive(Resource)]
pub(crate) struct ShocoVoxRenderPipeline {
    pub update_tree: bool,
    pub(crate) viewing_glass_bind_group_layout: BindGroupLayout,
    pub(crate) render_data_bind_group_layout: BindGroupLayout,
    pub(crate) update_pipeline: CachedComputePipelineId,
    pub(crate) viewing_glass_bind_group: Option<BindGroup>,
    pub(crate) tree_bind_group: Option<BindGroup>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub(crate) struct ShocoVoxLabel;

pub(crate) struct ShocoVoxRenderNode {
    pub(crate) ready: bool,
    pub(crate) resolution: [u32; 2],
}

#[cfg(test)]
mod types_wgpu_byte_compatibility_tests {
    use super::{OctreeMetaData, SizedNode, Viewport, Voxelement};
    use bevy::render::render_resource::{encase::ShaderType, ShaderSize};

    #[test]
    fn test_wgpu_compatibility() {
        Viewport::assert_uniform_compat();
        assert_eq!(
            std::mem::size_of::<Viewport>(),
            Viewport::SHADER_SIZE.get() as usize
        );

        OctreeMetaData::assert_uniform_compat();
        assert_eq!(
            std::mem::size_of::<OctreeMetaData>(),
            OctreeMetaData::SHADER_SIZE.get() as usize
        );

        Voxelement::assert_uniform_compat();
        assert_eq!(
            std::mem::size_of::<Voxelement>(),
            Voxelement::SHADER_SIZE.get() as usize
        );

        SizedNode::assert_uniform_compat();
        assert_eq!(
            std::mem::size_of::<SizedNode>(),
            SizedNode::SHADER_SIZE.get() as usize
        );
    }
}
