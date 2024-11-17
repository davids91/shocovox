use crate::octree::{Albedo, V3cf32};
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
            AsBindGroup, BindGroup, BindGroupLayout, Buffer, CachedComputePipelineId, ShaderType,
        },
        renderer::RenderQueue,
    },
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
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

pub struct SvxRenderPlugin {
    pub resolution: [u32; 2],
}

#[derive(Resource, Clone, AsBindGroup, TypePath, ExtractResource)]
#[type_path = "shocovox::gpu::OctreeGPUView"]
pub struct OctreeGPUView {
    // +++ DEBUG +++
    pub do_the_thing: bool,
    pub read_back: u32,
    // --- DEBUG ---
    pub viewing_glass: SvxViewingGlass,
    pub(crate) data_handler: Arc<Mutex<OctreeGPUDataHandler>>,
}

#[derive(Debug, Clone)]
pub(crate) struct VictimPointer {
    pub(crate) max_meta_len: usize, //TODO: should be private to type
    pub(crate) stored_items: usize,
    pub(crate) meta_index: usize,
    pub(crate) child: usize,
}

#[derive(Resource, Clone, AsBindGroup, TypePath, ExtractResource)]
#[type_path = "shocovox::gpu::OctreeGPUDataHandler"]
pub struct OctreeGPUDataHandler {
    pub(crate) render_data: SvxRenderData,
    pub(crate) victim_node: VictimPointer,
    pub(crate) victim_brick: VictimPointer,
    pub(crate) map_to_node_index_in_metadata: HashMap<usize, usize>,
    pub(crate) map_to_color_index_in_palette: HashMap<Albedo, usize>,
    pub(crate) debug_gpu_interface: Option<Buffer>,
    pub(crate) readable_debug_gpu_interface: Option<Buffer>,
    //TODO: Maybe this?
    // Buffers for the RenderData
    // pub(crate) octree_meta_buffer: Buffer,
    // pub(crate) metadata_buffer: Buffer,
    // pub(crate) node_children_buffer: Buffer,
    // pub(crate) node_ocbits_buffer: Buffer,
    // pub(crate) voxels_buffer: Buffer,
    // pub(crate) color_palette_buffer: Buffer,
    // pub(crate) readable_metadata_buffer: Buffer,
}

#[derive(Clone, AsBindGroup, TypePath)]
#[type_path = "shocovox::gpu::ShocoVoxViewingGlass"]
pub struct SvxViewingGlass {
    #[storage_texture(0, image_format = Rgba8Unorm, access = ReadWrite)]
    pub output_texture: Handle<Image>,

    #[uniform(1, visibility(compute))]
    pub viewport: Viewport,
}

#[derive(Clone, AsBindGroup, TypePath)]
#[type_path = "shocovox::gpu::ShocoVoxRenderData"]
pub struct SvxRenderData {
    // +++ DEBUG +++
    #[storage(6, visibility(compute))]
    pub(crate) debug_gpu_interface: u32,
    // --- DEBUG ---
    /// Contains the properties of the Octree
    #[uniform(0, visibility(compute))]
    pub(crate) octree_meta: OctreeMetaData,

    /// Contains the properties of Nodes and Voxel Bricks
    ///  _===================================================================_
    /// | Byte 0  | Node properties                                           |
    /// |---------------------------------------------------------------------|
    /// |  bit 0  | 1 if node is used by the raytracing algorithm *(2) *(4)   |
    /// |  bit 1  | unused                                                    |
    /// |  bit 2  | 1 in case node is a leaf                                  |
    /// |  bit 3  | 1 in case node is uniform                                 |
    /// |  bit 4  | unused - potentially: 1 if node has voxels                |
    /// |  bit 5  | unused - potentially: voxel brick size: 1, full or sparse |
    /// |  bit 6  | unused - potentially: voxel brick size: 1, full or sparse |
    /// |  bit 7  | unused                                                    |
    /// |=====================================================================|
    /// | Byte 1  | Node Child occupied                                       |
    /// |---------------------------------------------------------------------|
    /// | If Leaf | each bit is 0 if child brick is empty at octant *(1)      |
    /// | If Node | unused                                                    |
    /// |=====================================================================|
    /// | Byte 2  | Node Child structure                                      |
    /// |---------------------------------------------------------------------|
    /// | If Leaf | each bit is 0 if child brick is solid, 1 if parted *(1)   |
    /// | If Node | unused                                                    |
    /// |=====================================================================|
    /// | Byte 3  | Voxel Bricks used *(3)                                    |
    /// |---------------------------------------------------------------------|
    /// | each bit is 1 if voxel brick is used by the raytracing algorithm    |
    /// `=====================================================================`
    /// *(1) Only first bit is used in case uniform leaf nodes
    /// *(2) The same bit is used for node_children and node_occupied_bits
    /// *(3) One index in the array covers 8 bricks, which is the theoretical maximum
    ///      number of bricks for one node. In practice however the number of bricks
    ///      are only 4-5 times more, than the number of nodes, because of the internal nodes.
    /// *(4) Root node does not have this bit used, because it will never be overwritten
    ///      due to the victim pointer logic
    #[storage(1, visibility(compute))]
    pub(crate) metadata: Vec<u32>,

    /// Index values for Nodes, 8 value per @SizedNode entry. Each value points to:
    /// In case of Internal Nodes
    /// -----------------------------------------
    ///
    /// In case of Leaf Nodes:
    /// -----------------------------------------
    /// index of where the voxel brick start inside the @voxels buffer.
    /// Leaf node might contain 1 or 8 bricks according to @sized_node_meta, while
    #[storage(2, visibility(compute))]
    pub node_children: Vec<u32>,

    /// Buffer of Node occupancy bitmaps. Each node has a 64 bit bitmap,
    /// which is stored in 2 * u32 values
    #[storage(3, visibility(compute))]
    pub(crate) node_ocbits: Vec<u32>,

    /// Buffer of Voxel Bricks. Each brick contains voxel_brick_dim^3 elements.
    /// Each Brick has a corresponding 64 bit occupancy bitmap in the @voxel_maps buffer.
    #[storage(4, visibility(compute))]
    pub(crate) voxels: Vec<Voxelement>,

    /// Stores each unique color, it is references in @voxels
    /// and in @children_buffer as well( in case of solid bricks )
    #[storage(5, visibility(compute))]
    pub(crate) color_palette: Vec<Vec4>,
}

#[derive(Resource)]
pub(crate) struct SvxRenderPipeline {
    pub update_tree: bool,

    pub(crate) render_queue: RenderQueue,
    pub(crate) update_pipeline: CachedComputePipelineId,

    // Render data buffers
    pub(crate) octree_meta_buffer: Option<Buffer>,
    pub(crate) metadata_buffer: Option<Buffer>,
    pub(crate) readable_metadata_buffer: Option<Buffer>,
    pub(crate) node_children_buffer: Option<Buffer>,
    pub(crate) node_ocbits_buffer: Option<Buffer>,
    pub(crate) voxels_buffer: Option<Buffer>,
    pub(crate) color_palette_buffer: Option<Buffer>,
    pub(crate) tree_data_handler: Option<OctreeGPUDataHandler>,

    pub(crate) viewing_glass_bind_group_layout: BindGroupLayout,
    pub(crate) render_data_bind_group_layout: BindGroupLayout,
    pub(crate) viewing_glass_bind_group: Option<BindGroup>,
    pub(crate) tree_bind_group: Option<BindGroup>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub(crate) struct SvxLabel;

pub(crate) struct SvxRenderNode {
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
