use crate::object_pool::key_none_value;
use crate::octree::NodeContent;

use bevy::asset::Asset;
use bevy::ecs::system::Resource;
use bevy::math::{Vec2, Vec3};
use bevy::pbr::Material;
use bevy::render::{
    color::Color,
    render_resource::{ShaderRef, ShaderType},
};
use bevy::{reflect::TypePath, render::render_resource::AsBindGroup};

#[derive(Clone, ShaderType)]
struct Voxelement {
    albedo: Color,
    content: u32,
}

#[derive(Clone, ShaderType)]
struct SizedNode {
    contains_nodes: u32, // it is a leaf if it contains 1 node and has no children
    children: [u32; 8],  // Either an index or a "none value"
    voxels_start_at: u32, // index of where the voxel values contained in the node start inside the voxels buffer,
                          // or a "none_value". Should the field contain an index, the next voxel_matrix_dim^3 elements
                          // inside the voxels buffer count as part of the nodes voxel
}

#[derive(Clone, ShaderType)]
struct OctreeMetaData {
    octree_size: u32,
    voxel_matrix_dim: u32,
    ambient_light_color: Color,
    ambient_light_position: Vec3,
}

#[derive(Clone, Copy, ShaderType)]
pub struct Viewport {
    pub origin: Vec3,
    pub direction: Vec3,
    pub size: Vec2,
    pub fov: f32,
}

#[derive(Asset, Resource, Clone, AsBindGroup, TypePath)]
#[type_path = "shocovox::gpu::OctreeViewMaterial"]
pub struct OctreeViewMaterial {
    #[uniform(0)]
    pub viewport: Viewport,

    #[uniform(1)]
    meta: OctreeMetaData,

    #[storage(2)]
    nodes: Vec<SizedNode>,

    #[storage(3)]
    voxels: Vec<Voxelement>,
}

impl Material for OctreeViewMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/viewport_render.wgsl".into()
    }
}

use crate::octree::{Octree, VoxelData};
impl<T: Default + Clone + VoxelData, const DIM: usize> Octree<T, DIM> {
    pub fn create_bevy_material_view(&self, viewport: &Viewport) -> OctreeViewMaterial {
        let meta = OctreeMetaData {
            octree_size: self.octree_size,
            voxel_matrix_dim: DIM as u32,
            ambient_light_color: Color::rgba(1., 1., 1., 1.),
            ambient_light_position: Vec3::new(
                self.octree_size as f32,
                self.octree_size as f32,
                self.octree_size as f32,
            ),
        };
        let mut nodes = Vec::new();
        let mut voxels = Vec::new();
        for i in 0..self.nodes.len() {
            match self.nodes.get(i) {
                NodeContent::Leaf(data) => {
                    nodes.push(SizedNode {
                        contains_nodes: 1,
                        children: self.node_children[i].get_full(),
                        voxels_start_at: voxels.len() as u32,
                    });
                    for x in 0..DIM {
                        for y in 0..DIM {
                            for z in 0..DIM {
                                let albedo = data[x][y][z].albedo();
                                let content = data[x][y][z].user_data().unwrap_or(0);
                                voxels.push(Voxelement {
                                    albedo: Color::rgba(
                                        albedo[0] as f32 / 255.,
                                        albedo[1] as f32 / 255.,
                                        albedo[2] as f32 / 255.,
                                        albedo[3] as f32 / 255.,
                                    ),
                                    content,
                                })
                            }
                        }
                    }
                }
                NodeContent::Internal(count) => {
                    nodes.push(SizedNode {
                        contains_nodes: *count,
                        children: self.node_children[i].get_full(),
                        voxels_start_at: key_none_value(),
                    });
                }
                NodeContent::Nothing => {
                    nodes.push(SizedNode {
                        contains_nodes: 0,
                        children: self.node_children[i].get_full(),
                        voxels_start_at: key_none_value(),
                    });
                }
            }
        }
        OctreeViewMaterial {
            viewport: *viewport,
            meta,
            nodes,
            voxels,
        }
    }
}
