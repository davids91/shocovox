use crate::octree::{Cube, V3c};
use crate::spatial::raytracing::CubeRayIntersection;

#[cfg(feature = "bevy_wgpu")]
use bevy::{
    asset::Asset,
    ecs::system::Resource,
    math::{Vec2, Vec3},
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    render::{color::Color, render_resource::ShaderType},
};

pub(crate) struct NodeStackItem {
    pub(crate) bounds_intersection: CubeRayIntersection,
    pub(crate) bounds: Cube,
    pub(crate) node: u32,
    pub(crate) target_octant: u8,
    pub(crate) child_center: V3c<f32>,
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
    pub(crate) contains_nodes: u32, // it is a leaf if it contains 1 node and has no children
    pub(crate) children: [u32; 8],  // Either an index or a "none value"
    pub(crate) voxels_start_at: u32, // index of where the voxel values contained in the node start inside the voxels buffer,
                                     // or a "none_value". Should the field contain an index, the next voxel_matrix_dim^3 elements
                                     // inside the voxels buffer count as part of the nodes voxel
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
#[derive(Asset, Resource, Clone, AsBindGroup, TypePath)]
#[type_path = "shocovox::gpu::OctreeViewMaterial"]
pub struct OctreeViewMaterial {
    #[uniform(0)]
    pub viewport: Viewport,

    #[uniform(1)]
    pub(crate) meta: OctreeMetaData,

    #[storage(2)]
    pub(crate) nodes: Vec<SizedNode>,

    #[storage(3)]
    pub(crate) voxels: Vec<Voxelement>,
}
