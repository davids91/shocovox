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
            AsBindGroup, BindGroup, BindGroupLayout, Buffer, CachedComputePipelineId, ShaderType,
        },
        renderer::RenderQueue,
    },
};

#[derive(Clone, ShaderType)]
pub(crate) struct Voxelement {
    pub(crate) albedo_index: u32, // in color palette
    pub(crate) content: u32,
}

#[derive(Clone, ShaderType)]
pub(crate) struct SizedNode {
    /// Cache index of where the voxel values contained in the node start inside the voxels buffer,
    /// or a "none_value". Should the field contain an index, the next voxel_brick_dim^3 elements
    /// inside the @voxels array count as part of the voxels associated with the node
    pub(crate) voxels_start_at: u32,
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
    pub do_the_thing: bool, //STRICTLY FOR DEBUG REASONS

    // new layout
    // #[uniform(0, visibility(compute))]
    // pub(crate) meta: OctreeMetaData,

    // #[storage(1, visibility(compute))]
    // pub(crate) root_node: SizedNode,

    // #[storage(2, visibility(compute))]
    // pub(crate) nodes: Vec<SizedNode>,

    // #[storage(3, visibility(compute))]
    // pub(crate) children_buffer: Vec<u32>,

    // #[storage(4, visibilty(compute))]
    // pub(crate) voxels: Vec<Voxelement>,

    // #[storage(5, visibility(compute))]
    // pub(crate) color_palette: Vec<Vec4>,
    /// Bits storing information for multiple fields
    /// Each array is the same size, but might be different in terms of bytes per index.
    /// e.g. Nodes have 1 SizedNode under each index, children_buffer have 8, Voxelements have DIM*DIM*DIM
    /// For each index in the array, 2 Bytes of metadata is accounted for.
    /// Structure is the following:
    /// _---------------------------------------------------------------------_
    /// | Byte 0 | metadata                                                   |
    /// |---------------------------------------------------------------------|
    /// |  bit 0 | 1 in case node is used by the raytracing algorithm*        |
    /// |  bit 1 | 1 in case voxel brick is used by the raytracing algorithm  |
    /// |  bit 2 | 1 in case node is a leaf                                   |
    /// |  ...   | unused, potentially: 1 if node has children                |
    /// |  ...   | unused, potentially: 1 if node has user data               |
    /// |  ...   | unused, potentially: 1 if node has voxels                  |
    /// |  ...   | unused, potentially: voxel brick size: 1, full or sparse   |
    /// |---------------------------------------------------------------------|
    /// | Byte 1 | nodes lvl2 occupancy bitmap                                |
    /// `---------------------------------------------------------------------`
    ///
    /// * the same bit is used for children_buffer
    #[storage(5, visibility(compute))]
    pub(crate) data_meta_bytes: Vec<u32>,

    // old layout.. TO BE REMOVED
    #[uniform(0, visibility(compute))]
    pub(crate) octree_meta: OctreeMetaData,

    #[storage(1, visibility(compute))]
    pub(crate) nodes: Vec<SizedNode>,

    /// [u32; 8] for each node, with the structure:
    /// - In case of internal nodes:
    ///   - 8 Index value of node children
    /// - In case of leaf nodes:
    ///   - Byte 1-4: Occupancy bitmap LSB
    ///   - Byte 5-8: Occupancy bitmap MSB
    ///   - Byte 9-12: TBD
    ///   - Byte 13-16: TBD
    ///   - Byte 17-20: TBD
    ///   - Byte 21-24: TBD
    ///   - Byte 25-28: TBD
    ///   - Byte 29-32: TBD
    #[storage(2, visibility(compute))]
    pub node_children: Vec<u32>,

    #[storage(3, visibility(compute))]
    pub(crate) voxels: Vec<Voxelement>,

    #[storage(4, visibility(compute))]
    pub(crate) color_palette: Vec<Vec4>,
}

#[derive(Resource)]
pub(crate) struct ShocoVoxRenderPipeline {
    pub update_tree: bool,

    // The candidates for deletion inside nodes array on page deletion
    pub(crate) victim_pointer: u32,

    pub(crate) render_queue: RenderQueue,
    pub(crate) update_pipeline: CachedComputePipelineId,

    // Render data buffers
    pub(crate) octree_meta_buffer: Option<Buffer>,
    pub(crate) nodes_buffer: Option<Buffer>,
    pub(crate) node_children_buffer: Option<Buffer>,
    pub(crate) voxels_buffer: Option<Buffer>,
    pub(crate) color_palette_buffer: Option<Buffer>,
    pub(crate) data_meta_bytes_buffer: Option<Buffer>,
    pub(crate) readable_data_meta_bytes_buffer: Option<Buffer>,

    pub(crate) viewing_glass_bind_group_layout: BindGroupLayout,
    pub(crate) render_data_bind_group_layout: BindGroupLayout,
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
    use bevy::render::render_resource::encase::ShaderType;

    #[test]
    fn test_wgpu_compatibility() {
        Viewport::assert_uniform_compat();
        OctreeMetaData::assert_uniform_compat();
        Voxelement::assert_uniform_compat();
        SizedNode::assert_uniform_compat();
    }
}
