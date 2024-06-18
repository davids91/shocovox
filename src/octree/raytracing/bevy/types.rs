use bevy::{
    asset::Handle,
    ecs::component::Component,
    ecs::system::Resource,
    math::{Vec2, Vec3},
    reflect::TypePath,
    render::{
        color::Color,
        extract_resource::ExtractResource,
        render_graph::RenderLabel,
        render_resource::{
            AsBindGroup, BindGroup, BindGroupLayout, CachedComputePipelineId, ShaderType,
        },
        texture::Image,
    },
};

#[derive(Clone, ShaderType)]
pub(crate) struct Voxelement {
    pub(crate) albedo: Color,
    pub(crate) content: u32,
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

    /// - In case of internal nodes:
    ///   - Index values of node children
    /// - In case of leaf nodes:
    ///   - Byte 1-4: Occupancy bitmap MSB
    ///   - Byte 5-8: Occupancy bitmap LSB
    ///   - Byte 9-12: TBD
    ///   - Byte 13-16: TBD
    ///   - Byte 17-20: TBD
    ///   - Byte 21-24: TBD
    ///   - Byte 25-28: TBD
    ///   - Byte 29-32: TBD
    pub(crate) children: [u32; 8],

    /// index of where the voxel values contained in the node start inside the voxels buffer,
    /// or a "none_value". Should the field contain an index, the next voxel_brick_dim^3 elements
    /// inside the @voxels array count as part of the voxels associated with the node
    pub(crate) voxels_start_at: u32,
}

#[derive(Clone, ShaderType)]
pub struct OctreeMetaData {
    pub(crate) octree_size: u32,
    pub(crate) voxel_brick_dim: u32,
    pub ambient_light_color: Color,
    pub ambient_light_position: Vec3,
}

#[derive(Clone, Copy, ShaderType)]
pub struct Viewport {
    pub origin: Vec3,
    pub direction: Vec3,
    pub size: Vec2,
    pub fov: f32,
}

pub struct ShocoVoxRenderPlugin {
    pub resolution: [u32; 2],
}

#[derive(Resource, Clone, AsBindGroup, TypePath, ExtractResource)]
#[type_path = "shocovox::gpu::ShocoVoxViewingGlass"]
pub struct ShocoVoxViewingGlass {
    #[storage_texture(1, image_format = Rgba8Unorm, access = ReadWrite)]
    pub output_texture: Handle<Image>,

    #[uniform(2, visibility(compute))]
    pub viewport: Viewport,

    #[uniform(3, visibility(compute))]
    pub(crate) meta: OctreeMetaData,

    #[storage(4, visibility(compute))]
    pub(crate) nodes: Vec<SizedNode>,

    #[storage(5, visibility(compute))]
    pub(crate) voxels: Vec<Voxelement>,
}

#[derive(Resource)]
pub(crate) struct ShocoVoxRenderPipeline {
    pub(crate) viewing_glass_bind_group_layout: BindGroupLayout,
    pub(crate) update_pipeline: CachedComputePipelineId,
    pub(crate) bind_group: Option<BindGroup>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub(crate) struct ShocoVoxLabel;

pub(crate) struct ShocoVoxRenderNode {
    pub(crate) ready: bool,
    pub(crate) resolution: [u32; 2],
}
