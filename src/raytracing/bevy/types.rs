use crate::octree::{types::PaletteIndexValues, BoxTree, V3cf32, VoxelData};
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
    hash::Hash,
    ops::Range,
    sync::{Arc, Mutex},
};

#[derive(Clone, ShaderType)]
pub struct OctreeMetaData {
    /// Color of the ambient light in the render
    pub ambient_light_color: V3cf32,

    /// Position of the ambient light in the render
    pub ambient_light_position: V3cf32,

    /// Size of the octree to display
    pub(crate) boxtree_size: u32,

    /// Contains the properties of the Octree
    ///  _===================================================================_
    /// | Byte 0-1 | Voxel Brick Dimension                                    |
    /// |=====================================================================|
    /// | Byte 2   | Features                                                 |
    /// |---------------------------------------------------------------------|
    /// |  bit 0   | 1 if MIP maps are enabled                                |
    /// |  bit 1   | unused                                                   |
    /// |  bit 2   | unused                                                   |
    /// |  bit 3   | unused                                                   |
    /// |  bit 4   | unused                                                   |
    /// |  bit 5   | unused                                                   |
    /// |  bit 6   | unused                                                   |
    /// |  bit 7   | unused                                                   |
    /// |=====================================================================|
    /// | Byte 3   | unused                                                   |
    /// `=====================================================================`
    pub(crate) tree_properties: u32,
}

#[derive(Debug, Clone, Copy, ShaderType)]
pub struct Viewport {
    /// The origin of the viewport, think of it as the position the eye
    pub origin: V3cf32,

    /// The direction the raycasts are based upon, think of it as wherever the eye looks
    pub direction: V3cf32,

    /// The volume the viewport reaches to
    /// * `x` - looking glass width
    /// * `y` - looking glass height
    /// * `z` - the max depth of the viewport
    pub frustum: V3cf32,

    /// Field of View: how scattered will the rays in the viewport are
    pub fov: f32,
}

pub struct RenderBevyPlugin<T = u32>
where
    T: Default + Clone + Eq + VoxelData + Send + Sync + 'static,
{
    pub(crate) dummy: std::marker::PhantomData<T>,
}

#[derive(Resource, Clone, TypePath, ExtractResource)]
#[type_path = "shocovox::gpu::OctreeGPUHost"]
pub struct OctreeGPUHost<T = u32>
where
    T: Default + Clone + Eq + VoxelData + Send + Sync + Hash + 'static,
{
    pub tree: BoxTree<T>,
}

#[derive(Default, Resource, Clone, TypePath, ExtractResource)]
#[type_path = "shocovox::gpu::SvxViewSet"]
pub struct SvxViewSet {
    pub views: Vec<Arc<Mutex<OctreeGPUView>>>,
    pub(crate) resources: Vec<Option<OctreeRenderDataResources>>,
}

/// The Camera responsible for storing frustum and view related data
#[derive(Clone)]
pub struct OctreeSpyGlass {
    pub(crate) output_texture: Handle<Image>,
    pub(crate) viewport_changed: bool,
    pub(crate) viewport: Viewport,
    pub(crate) node_requests: Vec<u32>,
}

/// A View of an Octree
#[derive(Resource, Clone)]
pub struct OctreeGPUView {
    /// The camera for casting the rays
    pub spyglass: OctreeSpyGlass,

    /// Set to true if the view needs to be reloaded
    pub(crate) reload: bool,

    /// True if the initial data already sent to GPU
    pub init_data_sent: bool,

    /// Sets to true if related data on the GPU matches with CPU
    pub data_ready: bool,

    /// The data handler responsible for uploading data to the GPU
    pub(crate) data_handler: OctreeGPUDataHandler,

    /// The currently used resolution the raycasting dimensions are based for the base ray
    pub(crate) resolution: [u32; 2],

    /// The currently used output texture
    pub(crate) output_texture: Handle<Image>,

    /// The new resolution to be set ASAP if any
    pub(crate) new_resolution: Option<[u32; 2]>,

    /// The new output texture to be used, if any
    pub(crate) new_output_texture: Option<Handle<Image>>,
}

#[derive(Debug, Clone)]
pub(crate) struct VictimPointer {
    pub(crate) max_meta_len: usize,
    pub(crate) loop_count: usize,
    pub(crate) stored_items: usize,
    pub(crate) meta_index: usize,
    pub(crate) child: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum BrickOwnedBy {
    NotOwned,
    NodeAsChild(u32, u8),
    NodeAsMIP(u32),
}

#[derive(Resource, Clone)]
pub struct OctreeGPUDataHandler {
    pub(crate) render_data: OctreeRenderData,
    pub(crate) victim_node: VictimPointer,
    pub(crate) victim_brick: usize,
    pub(crate) node_key_vs_meta_index: BiHashMap<usize, usize>,
    pub(crate) brick_ownership: BiHashMap<usize, BrickOwnedBy>,
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
    pub(crate) boxtree_meta_buffer: Buffer,
    pub(crate) used_bits_buffer: Buffer,
    pub(crate) node_metadata_buffer: Buffer,
    pub(crate) node_children_buffer: Buffer,
    pub(crate) node_mips_buffer: Buffer,

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
    pub(crate) readable_used_bits_buffer: Buffer,
    // }--
}

/// An update to a single brick inside the GPU cache in a view
#[derive(Default)]
pub(crate) struct BrickUpdate<'a> {
    pub(crate) brick_index: usize,
    pub(crate) data: Option<&'a [PaletteIndexValues]>,
}

/// An update generated by a request to insert a node, brick or MIP
#[derive(Default)]
pub(crate) struct CacheUpdatePackage<'a> {
    /// The bricks updated during the request
    pub(crate) brick_updates: Vec<BrickUpdate<'a>>,

    /// The list of modified nodes during the operation
    pub(crate) modified_nodes: Vec<usize>,

    /// Used bits updated for both bricks and nodes inside the render data cache
    pub(crate) modified_usage_range: Range<usize>,
}

#[derive(Clone, TypePath)]
#[type_path = "shocovox::gpu::ShocoVoxRenderData"]
pub struct OctreeRenderData {
    /// CPU only field, contains stored MIP feature enabled state
    pub(crate) mips_enabled: bool,

    /// Contains the properties of the Octree
    pub(crate) octree_meta: OctreeMetaData,

    /// Usage information for nodes and bricks
    ///  _===============================================================_
    /// |    bit 0 | 1 if node is used by raytracing algo*               |
    /// |----------------------------------------------------------------|
    /// | bit 1-30 | 30x 1 bit: 1 if brick used by the raytracing algo   |
    /// `================================================================`
    /// * - Same bit used for node_children, node_ocbits, node_mips and node_structure
    ///   - Root node doesn't use this bit, as it will never be overwritten by cache
    pub(crate) used_bits: Vec<u32>,

    /// Node Property descriptors
    ///  _===============================================================_
    /// | Byte 0   | 8x 1 bit: 1 in case node is a leaf                  |
    /// |----------------------------------------------------------------|
    /// | Byte 1   | 8x 1 bit: 1 in case node is uniform                 |
    /// |----------------------------------------------------------------|
    /// | Byte 2   | 8x 1 bit: 1 if node has MIP                         |
    /// |----------------------------------------------------------------|
    /// | Byte 3   | unused - potentially: normal vector index           |
    /// `================================================================`
    pub(crate) node_metadata: Vec<u32>,

    /// Composite field: Children information
    /// In case of Internal Nodes
    /// -----------------------------------------
    /// Index values for Nodes, 64 value per @SizedNode entry.
    /// Each value points to one of 64 children of the node,
    /// either pointing to a node in metadata, or marked empty
    /// when there are no children in the given sectant
    ///
    /// In case of Leaf Nodes:
    /// -----------------------------------------
    /// Contains 64 bricks pointing to the child of the node for the relevant sectant
    /// according to @node_metadata ( Uniform/Non-uniform ) a node may have 1
    /// or 64 children, in that case only the first index is used.
    /// Structure is as follows:
    ///  _===============================================================_
    /// | bit 0-30 | index of where the voxel brick starts               |
    /// |          | inside the @voxels_buffer(when parted)              |
    /// |          | or inside the @color_palette(when solid)            |
    /// |----------------------------------------------------------------|
    /// |   bit 31 | 0 if brick is parted, 1 if solid                    |
    /// `================================================================`
    pub(crate) node_children: Vec<u32>,

    /// Index values for node MIPs stored inside the bricks, each node has one MIP index, or marked empty
    /// Structure is the same as one child in @node_children
    pub(crate) node_mips: Vec<u32>,

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
    pub(crate) spyglass_bind_group_layout: BindGroupLayout,
    pub(crate) render_data_bind_group_layout: BindGroupLayout,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub(crate) struct SvxLabel;

pub(crate) struct SvxRenderNode {
    pub(crate) ready: bool,
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
