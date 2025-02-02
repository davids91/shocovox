use crate::octree::{types::PaletteIndexValues, Octree, V3cf32, VoxelData};
use bevy::{
    asset::Handle,
    ecs::system::Resource,
    math::Vec4,
    prelude::Image,
    reflect::TypePath,
    render::{
        extract_resource::ExtractResource,
        render_graph::RenderLabel,
        render_resource::{
            BindGroup, BindGroupLayout, Buffer, CachedComputePipelineId, ShaderType,
        },
        renderer::RenderQueue,
    },
};
use bimap::BiHashMap;
use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex},
};

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

pub struct RenderBevyPlugin<T = u32>
where
    T: Default + Clone + Eq + VoxelData + Send + Sync + 'static,
{
    pub(crate) dummy: std::marker::PhantomData<T>,
    pub(crate) resolution: [u32; 2],
}

#[derive(Resource, Clone, TypePath, ExtractResource)]
#[type_path = "shocovox::gpu::OctreeGPUHost"]
pub struct OctreeGPUHost<T = u32>
where
    T: Default + Clone + Eq + VoxelData + Send + Sync + Hash + 'static,
{
    pub tree: Octree<T>,
}

#[derive(Default, Resource, Clone, TypePath, ExtractResource)]
#[type_path = "shocovox::gpu::SvxViewSet"]
pub struct SvxViewSet {
    pub views: Vec<Arc<Mutex<OctreeGPUView>>>,
}

#[derive(Resource, Clone)]
pub struct OctreeGPUView {
    pub spyglass: OctreeSpyGlass,
    pub(crate) data_handler: OctreeGPUDataHandler,
}

#[derive(Debug, Clone)]
pub(crate) struct VictimPointer {
    pub(crate) max_meta_len: usize,
    pub(crate) loop_count: usize,
    pub(crate) stored_items: usize,
    pub(crate) meta_index: usize,
    pub(crate) child: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BrickOwnedBy {
    NotOwned,
    Node(u32, u8),
}

#[derive(Resource, Clone)]
pub struct OctreeGPUDataHandler {
    pub(crate) render_data: OctreeRenderData,
    pub(crate) victim_node: VictimPointer,
    pub(crate) victim_brick: usize,
    pub(crate) node_key_vs_meta_index: BiHashMap<usize, usize>,
    pub(crate) brick_ownership: Vec<BrickOwnedBy>,
    pub(crate) map_to_brick_maybe_owned_by_node: HashMap<(usize, u8), usize>,
    pub(crate) uploaded_color_palette_size: usize,
}

#[derive(Clone)]
pub(crate) struct OctreeRenderDataResources {
    // Spyglass group
    // --{
    pub(crate) spyglass_bind_group: BindGroup,
    pub(crate) viewport_buffer: Buffer,
    pub(crate) node_requests_buffer: Buffer,
    // }--

    // Octree render data group
    // --{
    pub(crate) tree_bind_group: BindGroup,
    pub(crate) metadata_buffer: Buffer,
    pub(crate) node_children_buffer: Buffer,

    /// Buffer of Node occupancy bitmaps. Each node has a 64 bit bitmap,
    /// which is stored in 2 * u32 values. only available in GPU, to eliminate needles redundancy
    pub(crate) node_ocbits_buffer: Buffer,

    /// Buffer of Voxel Bricks. Each brick contains voxel_brick_dim^3 elements.
    /// Each Brick has a corresponding 64 bit occupancy bitmap in the @voxel_maps buffer.
    /// Only available in GPU, to eliminate needles redundancy
    pub(crate) voxels_buffer: Buffer,
    pub(crate) color_palette_buffer: Buffer,

    // Staging buffers for data reads
    pub(crate) readable_node_requests_buffer: Buffer,
    pub(crate) readable_metadata_buffer: Buffer,
    // }--
}

#[derive(Clone)]
pub struct OctreeSpyGlass {
    pub output_texture: Handle<Image>,
    pub viewport: Viewport,
    pub(crate) node_requests: Vec<u32>,
}

pub(crate) struct BrickUpdate<'a> {
    pub(crate) brick_index: usize,
    pub(crate) data: Option<&'a [PaletteIndexValues]>,
}

#[derive(Clone, TypePath)]
#[type_path = "shocovox::gpu::ShocoVoxRenderData"]
pub struct OctreeRenderData {
    /// Contains the properties of the Octree
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
    /// | each bit is 1 if brick is used (it means do not delete please)      |
    /// `=====================================================================`
    /// *(1) Only first bit is used in case uniform leaf nodes
    /// *(2) The same bit is used for node_children and node_occupied_bits
    /// *(3) One index in the array covers 8 bricks, which is the theoretical maximum
    ///      number of bricks for one node. In practice however the number of bricks
    ///      are only 4-5 times more, than the number of nodes, because of the internal nodes;
    ///      And only a fraction of them are visible in a render.
    /// *(4) Root node does not have this bit used, because it will never be overwritten
    ///      due to the victim pointer logic
    pub(crate) metadata: Vec<u32>,

    /// Index values for Nodes, 8 value per @SizedNode entry. Each value points to:
    /// In case of Internal Nodes
    /// -----------------------------------------
    ///
    /// In case of Leaf Nodes:
    /// -----------------------------------------
    /// index of where the voxel brick start inside the @voxels buffer.
    /// Leaf node might contain 1 or 8 bricks according to @sized_node_meta, while
    pub(crate) node_children: Vec<u32>,

    /// Buffer of Node occupancy bitmaps. Each node has a 64 bit bitmap,
    /// which is stored in 2 * u32 values
    pub(crate) node_ocbits: Vec<u32>,

    /// Stores each unique color, it is references in @voxels
    /// and in @children_buffer as well( in case of solid bricks )
    pub(crate) color_palette: Vec<Vec4>,
}

#[derive(Resource)]
pub(crate) struct SvxRenderPipeline {
    pub update_tree: bool,

    pub(crate) render_queue: RenderQueue,
    pub(crate) update_pipeline: CachedComputePipelineId,

    // Data layout and data
    pub(crate) spyglass_bind_group_layout: BindGroupLayout,
    pub(crate) render_data_bind_group_layout: BindGroupLayout,
    pub(crate) resources: Option<OctreeRenderDataResources>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub(crate) struct SvxLabel;

pub(crate) struct SvxRenderNode {
    pub(crate) ready: bool,
    pub(crate) resolution: [u32; 2],
}

#[cfg(test)]
mod types_wgpu_byte_compatibility_tests {
    use super::{OctreeMetaData, Viewport};
    use bevy::render::render_resource::encase::ShaderType;

    #[test]
    fn test_wgpu_compatibility() {
        Viewport::assert_uniform_compat();
        OctreeMetaData::assert_uniform_compat();
    }
}
