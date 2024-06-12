use crate::octree::{Cube, V3c};
use crate::spatial::raytracing::CubeRayIntersection;

#[cfg(feature = "bevy_wgpu")]
use bevy::{
    asset::Handle,
    ecs::system::Resource,
    math::{Vec2, Vec3},
    reflect::TypePath,
    render::{
        color::Color,
        extract_resource::ExtractResource,
        render_resource::{AsBindGroup, ShaderType},
        texture::Image,
    },
};

pub(crate) struct NodeStackItem {
    pub(crate) bounds_intersection: CubeRayIntersection,
    pub(crate) bounds: Cube,
    pub(crate) node: u32,
    pub(crate) occupied_bits: u8,
    pub(crate) target_octant: u8,
    pub(crate) child_center: V3c<f32>,
}

#[cfg(feature = "bevy_wgpu")]
pub struct ShocoVoxRenderPlugin {
    pub resolution: [u32; 2],
}

#[cfg(feature = "bevy_wgpu")]
#[derive(Clone, ShaderType)]
pub(crate) struct Voxelement {
    pub(crate) albedo: Color,
    pub(crate) content: u32,
}

#[cfg(feature = "bevy_wgpu")]
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
    /// or a "none_value". Should the field contain an index, the next voxel_matrix_dim^3 elements
    /// inside the @voxels array count as part of the voxels associated with the node
    pub(crate) voxels_start_at: u32,
}

#[cfg(feature = "bevy_wgpu")]
#[derive(Clone, ShaderType)]
pub struct OctreeMetaData {
    pub(crate) octree_size: u32,
    pub(crate) voxel_matrix_dim: u32,
    pub ambient_light_color: Color,
    pub ambient_light_position: Vec3,
}

#[cfg(feature = "bevy_wgpu")]
#[derive(Clone, Copy, ShaderType)]
pub struct Viewport {
    pub origin: Vec3,
    pub direction: Vec3,
    pub size: Vec2,
    pub fov: f32,
}

#[cfg(feature = "bevy_wgpu")]
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
