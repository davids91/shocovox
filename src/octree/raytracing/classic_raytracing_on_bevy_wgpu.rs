use crate::object_pool::key_none_value;
use crate::octree::{
    raytracing::types::{OctreeMetaData, OctreeViewMaterial, SizedNode, Viewport, Voxelement},
    NodeContent,
};

use bevy::{
    math::Vec3,
    pbr::Material,
    render::{color::Color, render_resource::ShaderRef},
};

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
                                let content = data[x][y][z].user_data();
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
                        contains_nodes: *count as u32,
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
