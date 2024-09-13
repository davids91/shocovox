use crate::object_pool::empty_marker;
use crate::octree::{
    raytracing::bevy::types::{OctreeMetaData, ShocoVoxRenderData, SizedNode, Voxelement},
    types::{NodeChildrenArray, NodeContent},
    Octree, V3c, VoxelData,
};
use bevy::math::Vec4;
use std::collections::HashMap;

impl<T, const DIM: usize> Octree<T, DIM>
where
    T: Default + Clone + VoxelData,
{
    fn meta_set_is_leaf(sized_node_meta: &mut u32, is_leaf: bool) {
        *sized_node_meta =
            (*sized_node_meta & 0x00FFFFFF) | if is_leaf { 0x01000000 } else { 0x00000000 };
    }

    fn meta_set_node_occupancy_bitmap(sized_node_meta: &mut u32, bitmap: u8) {
        *sized_node_meta = (*sized_node_meta & 0xFFFFFF00) | bitmap as u32;
    }

    fn create_meta(&self, node_key: usize) -> u32 {
        let node = self.nodes.get(node_key);
        let mut meta = 0;
        match node {
            NodeContent::Leaf(_) => {
                Self::meta_set_is_leaf(&mut meta, true);
                Self::meta_set_node_occupancy_bitmap(
                    &mut meta,
                    self.occupied_8bit(node_key as u32),
                );
            }
            NodeContent::Internal(occupied_bits) => {
                Self::meta_set_is_leaf(&mut meta, false);
                Self::meta_set_node_occupancy_bitmap(&mut meta, *occupied_bits);
            }
            _ => {
                Self::meta_set_is_leaf(&mut meta, false);
                Self::meta_set_node_occupancy_bitmap(&mut meta, 0x00);
            }
        };
        meta
    }

    pub fn create_bevy_view(&self) -> ShocoVoxRenderData {
        let octree_meta = OctreeMetaData {
            octree_size: self.octree_size,
            voxel_brick_dim: DIM as u32,
            ambient_light_color: V3c::new(1., 1., 1.),
            ambient_light_position: V3c::new(
                self.octree_size as f32,
                self.octree_size as f32,
                self.octree_size as f32,
            ),
        };

        let mut nodes = Vec::new();
        let mut node_children = Vec::new();
        let mut voxels = Vec::new();
        let mut color_palette = Vec::new();
        let mut data_meta_bytes = Vec::new();

        // Build up Nodes
        let mut map_to_node_index_in_nodes_buffer = HashMap::new();
        for i in 0..self.nodes.len() {
            if self.nodes.key_is_valid(i) {
                map_to_node_index_in_nodes_buffer.insert(i as usize, nodes.len());
                nodes.push(SizedNode {
                    // sized_node_meta: self.create_meta(i),
                    children_start_at: empty_marker(),
                    voxels_start_at: empty_marker(),
                });
            }
        }

        // Build up voxel content
        let mut map_to_color_index_in_palette = HashMap::new();
        for i in 0..self.nodes.len() {
            if !self.nodes.key_is_valid(i) {
                continue;
            }
            nodes[map_to_node_index_in_nodes_buffer[&i]].children_start_at =
                node_children.len() as u32;
            if let NodeContent::Leaf(data) = self.nodes.get(i) {
                debug_assert!(matches!(
                    self.node_children[i].content,
                    NodeChildrenArray::OccupancyBitmap(_)
                ));
                let occupied_bits = match self.node_children[i].content {
                    NodeChildrenArray::OccupancyBitmap(bitmap) => bitmap,
                    _ => panic!("Found Leaf Node without occupancy bitmap!"),
                };
                node_children.extend_from_slice(&[
                    (occupied_bits & 0x00000000FFFFFFFF) as u32,
                    ((occupied_bits & 0xFFFFFFFF00000000) >> 32) as u32,
                ]);
                nodes[map_to_node_index_in_nodes_buffer[&i]].voxels_start_at = voxels.len() as u32;
                for z in 0..DIM {
                    for y in 0..DIM {
                        for x in 0..DIM {
                            let albedo = data[x][y][z].albedo();
                            if !map_to_color_index_in_palette.contains_key(&albedo) {
                                map_to_color_index_in_palette.insert(albedo, color_palette.len());
                                color_palette.push(Vec4::new(
                                    albedo.r as f32 / 255.,
                                    albedo.g as f32 / 255.,
                                    albedo.b as f32 / 255.,
                                    albedo.a as f32 / 255.,
                                ));
                            }
                            let albedo_index = map_to_color_index_in_palette[&albedo];

                            voxels.push(Voxelement {
                                albedo_index: albedo_index as u32,
                                content: data[x][y][z].user_data(),
                            })
                        }
                    }
                }
            } else {
                //Internal nodes
                for c in 0..8 {
                    let child_index = &self.node_children[i][c];
                    if *child_index != self.node_children[i].empty_marker {
                        debug_assert!(map_to_node_index_in_nodes_buffer
                            .contains_key(&(*child_index as usize)));
                        node_children.push(
                            map_to_node_index_in_nodes_buffer[&(*child_index as usize)] as u32,
                        );
                    } else {
                        node_children.push(*child_index);
                    }
                }
            }
        }

        ShocoVoxRenderData {
            do_the_thing: false,
            data_meta_bytes,
            // root_node: SizedNode {
            //     sized_node_meta: self.create_meta(Self::ROOT_NODE_KEY as usize),
            //     children_start_at: empty_marker(),
            //     voxels_start_at: empty_marker(),
            // },
            octree_meta,
            nodes,
            node_children,
            voxels,
            color_palette,
        }
    }

    pub(crate) fn insert_elements_into_cache(
        &self,
        render_data: &mut ShocoVoxRenderData,
        requested_nodes: Vec<u32>,
    ) {
        //TODO: find the first unused element, and overwrite it with the item
    }
}
