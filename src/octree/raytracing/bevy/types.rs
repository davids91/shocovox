use crate::octree::V3cf32;
use bevy::{
    asset::Handle,
    ecs::system::Resource,
    math::Vec4,
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
    pub(crate) albedo_index: u32, // in color palette
    pub(crate) content: u32,
}

#[derive(Clone, ShaderType)]
pub struct OctreeMetaData {
    pub ambient_light_color: V3cf32,
    pub ambient_light_position: V3cf32,
    pub(crate) octree_size: u32,
    pub(crate) voxel_brick_dim: u32,
}

#[derive(Debug, Clone, Copy, ShaderType)]
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

    /// Composite field containing the properties of Nodes
    /// Structure is the following:
    ///  _===================================================================_
    /// | Byte 0  | Node properties                                           |
    /// |---------------------------------------------------------------------|
    /// |  bit 0  | unused - potentially: "node in use do not delete" bit     |
    /// |  bit 1  | unused - potentially: "brick in use do not delete" bit    |
    /// |  bit 2  | 1 in case node is a leaf                                  |
    /// |  bit 3  | 1 in case node is uniform                                 |
    /// |  bit 4  | unused - potentially: 1 if node has voxels                |
    /// |  bit 5  | unused - potentially: voxel brick size: 1, full or sparse |
    /// |  bit 6  | unused - potentially: voxel brick size: 1, full or sparse |
    /// |  bit 7  | unused                                                    |
    /// |=====================================================================|
    /// | Byte 1  | Child occupied                                            |
    /// |---------------------------------------------------------------------|
    /// | If Leaf | each bit is 0 if child brick is empty at octant *(1)      |
    /// | If Node | lvl1 occupancy bitmap                                     |
    /// |=====================================================================|
    /// | Byte 2  | Child structure                                           |
    /// |---------------------------------------------------------------------|
    /// | If Leaf | each bit is 0 if child brick is solid, 1 if parted *(1)   |
    /// | If Node | unused                                                    |
    /// |=====================================================================|
    /// | Byte 3  | unused                                                    |
    /// `=====================================================================`
    /// *(1) Only first bit is used in case leaf is uniform
    #[storage(1, visibility(compute))]
    pub(crate) nodes: Vec<u32>,

    /// Index values for Nodes, 8 value per @SizedNode entry. Each value points to:
    /// In case of Internal Nodes
    /// -----------------------------------------
    ///
    /// In case of Leaf Nodes:
    /// -----------------------------------------
    /// index of where the voxel brick start inside the @voxels buffer.
    /// Leaf node might contain 1 or 8 bricks according to @sized_node_meta, while
    #[storage(2, visibility(compute))]
    pub(crate) children_buffer: Vec<u32>,

    /// Buffer of Voxel Bricks. Each brick contains voxel_brick_dim^3 elements.
    /// Each Brick has a corresponding 64 bit occupancy bitmap in the @voxel_maps buffer.
    #[storage(3, visibility(compute))]
    pub(crate) voxels: Vec<Voxelement>,

    /// Buffer of Voxel brick occupancy bitmaps. Each brick has a 64 bit bitmap,
    /// which is stored in 2 * u32 values
    #[storage(4, visibility(compute))]
    pub(crate) voxel_maps: Vec<u32>,

    /// Stores each unique color, it is references in @voxels
    /// and in @children_buffer as well( in case of solid bricks )
    #[storage(5, visibility(compute))]
    pub(crate) color_palette: Vec<Vec4>,
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
    use super::{OctreeMetaData, Viewport, Voxelement};
    use bevy::render::render_resource::encase::ShaderType;

    #[test]
    fn test_wgpu_compatibility() {
        Viewport::assert_uniform_compat();
        OctreeMetaData::assert_uniform_compat();
        Voxelement::assert_uniform_compat();
    }
}
