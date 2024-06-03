use crate::object_pool::empty_marker;
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
impl<T, const DIM: usize> Octree<T, DIM>
where
    T: Default + Clone + VoxelData,
{
    fn meta_set_is_leaf(sized_node_meta: &mut u32, is_leaf: bool) {
        *sized_node_meta =
            (*sized_node_meta & 0x00FFFFFF) | if is_leaf { 0x01000000 } else { 0x00000000 };
    }

    fn meta_set_lvl2_occupancy_bitmask(sized_node_meta: &mut u32, bitmask: u8) {
        *sized_node_meta = (*sized_node_meta & 0xFFFFFF00) | bitmask as u32;
    }

    fn create_meta(&self, node_key: usize) -> u32 {
        let node = self.nodes.get(node_key);
        let mut meta = 0;
        match node {
            NodeContent::Leaf(_) => {
                Self::meta_set_is_leaf(&mut meta, true);
                Self::meta_set_lvl2_occupancy_bitmask(&mut meta, 0xFF);
            }
            NodeContent::Internal(occupied_bits) => {
                Self::meta_set_is_leaf(&mut meta, false);
                Self::meta_set_lvl2_occupancy_bitmask(&mut meta, *occupied_bits);
            }
            _ => {
                Self::meta_set_is_leaf(&mut meta, false);
                Self::meta_set_lvl2_occupancy_bitmask(&mut meta, 0x00);
            }
        };
        meta
    }

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
            let mut sized_node = SizedNode {
                sized_node_meta: self.create_meta(i),
                children: self.node_children[i].get_full(),
                voxels_start_at: empty_marker(),
            };
            if let NodeContent::Leaf(data) = self.nodes.get(i) {
                sized_node.voxels_start_at = voxels.len() as u32;
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
            nodes.push(sized_node);
        }
        OctreeViewMaterial {
            viewport: *viewport,
            meta,
            nodes,
            voxels,
        }
    }
}
