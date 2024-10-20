use crate::object_pool::empty_marker;
use crate::octree::types::BrickData;
use crate::octree::{
    raytracing::bevy::types::{OctreeMetaData, ShocoVoxRenderData, Voxelement},
    types::{NodeChildrenArray, NodeContent},
    Albedo, Octree, V3c, VoxelData,
};
use bevy::math::Vec4;
use std::collections::HashMap;

impl<T, const DIM: usize> Octree<T, DIM>
where
    T: Default + Clone + Copy + PartialEq + VoxelData,
{
    /// Updates the meta element value to store that the corresponding node is a leaf node
    fn meta_set_is_leaf(sized_node_meta: &mut u32, is_leaf: bool) {
        *sized_node_meta =
            (*sized_node_meta & 0xFFFFFFFB) | if is_leaf { 0x00000004 } else { 0x00000000 };
    }

    /// Updates the meta element value to store that the corresponding node is a uniform leaf node
    fn meta_set_is_uniform(sized_node_meta: &mut u32, is_uniform: bool) {
        *sized_node_meta =
            (*sized_node_meta & 0xFFFFFFF7) | if is_uniform { 0x00000008 } else { 0x00000000 };
    }

    /// Updates the meta element value to store that the corresponding node is a leaf node
    fn meta_set_node_occupancy_bitmap(sized_node_meta: &mut u32, bitmap: u8) {
        *sized_node_meta = (*sized_node_meta & 0xFFFF00FF) | ((bitmap as u32) << 8);
    }

    /// Updates the meta element value to store the brick structure of the given leaf node.
    /// Does not erase anything in @sized_node_meta, it's expected to be cleared before
    /// the first use of this function
    /// for the given brick octant
    /// * `sized_node_meta` - the bytes to update
    /// * `brick` - the brick to describe into the bytes
    /// * `brick_octant` - the octant to update in the bytes
    fn meta_add_leaf_brick_structure(
        sized_node_meta: &mut u32,
        brick: &BrickData<T, DIM>,
        brick_octant: usize,
    ) {
        match brick {
            BrickData::Empty => {} // Child structure properties already set to NIL
            BrickData::Solid(_voxel) => {
                // set child Occupied bits, child Structure bits already set to 0
                *sized_node_meta |= 0x01 << (8 + brick_octant);
            }
            BrickData::Parted(_brick) => {
                // set child Occupied bits
                *sized_node_meta |= 0x01 << (8 + brick_octant);

                // set child Structure bits
                *sized_node_meta |= 0x01 << (16 + brick_octant);
            }
        };
    }

    /// Updates the given meta element value to store the leaf structure of the given node
    /// the given NodeContent reference is expected to be a leaf node
    fn meta_set_leaf_structure(sized_node_meta: &mut u32, leaf: &NodeContent<T, DIM>) {
        match leaf {
            NodeContent::UniformLeaf(brick) => {
                Self::meta_set_is_leaf(sized_node_meta, true);
                Self::meta_set_is_uniform(sized_node_meta, true);
                Self::meta_add_leaf_brick_structure(sized_node_meta, brick, 0);
            }
            NodeContent::Leaf(bricks) => {
                Self::meta_set_is_leaf(sized_node_meta, true);
                Self::meta_set_is_uniform(sized_node_meta, false);
                for octant in 0..8 {
                    Self::meta_add_leaf_brick_structure(sized_node_meta, &bricks[octant], octant);
                }
            }
            NodeContent::Internal(_) | NodeContent::Nothing => {
                panic!("Expected node content to be of a leaf");
            }
        }
    }

    /// Creates the descriptor bytes for the given node
    fn create_node_properties(node: &NodeContent<T, DIM>) -> u32 {
        let mut meta = 0;
        match node {
            NodeContent::UniformLeaf(_) => {
                Self::meta_set_is_leaf(&mut meta, true);
                Self::meta_set_leaf_structure(&mut meta, node);
            }
            NodeContent::Leaf(_) => {
                Self::meta_set_is_leaf(&mut meta, true);
                Self::meta_set_leaf_structure(&mut meta, node);
            }
            NodeContent::Internal(occupied_bits) => {
                Self::meta_set_is_leaf(&mut meta, false);
                Self::meta_set_node_occupancy_bitmap(&mut meta, *occupied_bits);
            }
            NodeContent::Nothing => {
                Self::meta_set_is_leaf(&mut meta, false);
                Self::meta_set_node_occupancy_bitmap(&mut meta, 0x00);
            }
        };
        meta
    }

    /// Loads a brick into the provided voxels vector and color palette
    /// * `brick` - The brick to upload
    /// * `voxels` - The destination buffer
    /// * `color_palette` - The used color palette
    /// * `map_to_color_index_in_palette` - Indexing helper for the color palette
    /// * `returns` - the identifier to set in @SizedNode and true if a new brick was aded to the voxels vector
    fn add_brick_to_vec(
        brick: &BrickData<T, DIM>,
        voxels: &mut Vec<Voxelement>,
        color_palette: &mut Vec<Vec4>,
        map_to_color_index_in_palette: &mut HashMap<Albedo, usize>,
    ) -> (u32, bool) {
        match brick {
            BrickData::Empty => (empty_marker(), false),
            BrickData::Solid(voxel) => {
                let albedo = voxel.albedo();
                if let std::collections::hash_map::Entry::Vacant(e) =
                    map_to_color_index_in_palette.entry(albedo)
                {
                    e.insert(color_palette.len());
                    color_palette.push(Vec4::new(
                        albedo.r as f32 / 255.,
                        albedo.g as f32 / 255.,
                        albedo.b as f32 / 255.,
                        albedo.a as f32 / 255.,
                    ));
                }
                (map_to_color_index_in_palette[&albedo] as u32, false)
            }
            BrickData::Parted(brick) => {
                voxels.reserve(DIM * DIM * DIM);
                let brick_index = voxels.len() / (DIM * DIM * DIM);
                debug_assert_eq!(
                    voxels.len() % (DIM * DIM * DIM),
                    0,
                    "Expected Voxel buffer length({:?}) to be divisble by {:?}",
                    voxels.len(),
                    (DIM * DIM * DIM)
                );
                for z in 0..DIM {
                    for y in 0..DIM {
                        for x in 0..DIM {
                            let albedo = brick[x][y][z].albedo();
                            if let std::collections::hash_map::Entry::Vacant(e) =
                                map_to_color_index_in_palette.entry(albedo)
                            {
                                e.insert(color_palette.len());
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
                                content: brick[x][y][z].user_data(),
                            });
                        }
                    }
                }
                (brick_index as u32, true)
            }
        }
    }

    /// Creates GPU compatible data renderable on the GPU from an octree
    pub fn create_bevy_view(&self) -> ShocoVoxRenderData {
        let meta = OctreeMetaData {
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
        let mut children_buffer = Vec::new();
        let mut voxels = Vec::new();
        let mut voxel_maps = Vec::new();
        let mut color_palette = Vec::new();

        // Build up Nodes
        let mut map_to_node_index_in_nodes_buffer = HashMap::new();
        for i in 0..self.nodes.len() {
            if self.nodes.key_is_valid(i) {
                map_to_node_index_in_nodes_buffer.insert(i, nodes.len());
                nodes.push(Self::create_node_properties(self.nodes.get(i)));
            }
        }

        // Build up voxel content
        children_buffer.reserve(self.nodes.len() * 8);
        let mut map_to_color_index_in_palette = HashMap::new();
        for i in 0..self.nodes.len() {
            if !self.nodes.key_is_valid(i) {
                continue;
            }
            match self.nodes.get(i) {
                NodeContent::UniformLeaf(brick) => {
                    voxel_maps.reserve(2);
                    debug_assert!(
                        matches!(
                            self.node_children[i].content,
                            NodeChildrenArray::OccupancyBitmap(_)
                        ),
                        "Expected Uniform leaf to have OccupancyBitmap(_) instead of {:?}",
                        self.node_children[i].content
                    );

                    let (brick_index, brick_added) = Self::add_brick_to_vec(
                        brick,
                        &mut voxels,
                        &mut color_palette,
                        &mut map_to_color_index_in_palette,
                    );

                    children_buffer.extend_from_slice(&[brick_index, 0, 0, 0, 0, 0, 0, 0]);
                    if brick_added {
                        if let NodeChildrenArray::OccupancyBitmap(occupied_bits) =
                            self.node_children[i].content
                        {
                            voxel_maps.extend_from_slice(&[
                                (occupied_bits & 0x00000000FFFFFFFF) as u32,
                                ((occupied_bits & 0xFFFFFFFF00000000) >> 32) as u32,
                            ]);
                        } else {
                            panic!("Leaf node is expected to have Occupied bitmap array!");
                        }
                    } else {
                        // If no brick was added, the occupied bits should either be empty or full
                        if let NodeChildrenArray::OccupancyBitmap(occupied_bits) =
                            self.node_children[i].content
                        {
                            debug_assert!(occupied_bits == 0 || occupied_bits == u64::MAX);
                        }
                    }
                }
                NodeContent::Leaf(bricks) => {
                    voxel_maps.reserve(16);
                    debug_assert!(
                        matches!(
                            self.node_children[i].content,
                            NodeChildrenArray::OccupancyBitmaps(_)
                        ),
                        "Expected Leaf to have OccupancyBitmaps(_) instead of {:?}",
                        self.node_children[i].content
                    );

                    let mut children = vec![0; 8];
                    for octant in 0..8 {
                        let (brick_index, brick_added) = Self::add_brick_to_vec(
                            &bricks[octant],
                            &mut voxels,
                            &mut color_palette,
                            &mut map_to_color_index_in_palette,
                        );

                        children[octant] = brick_index;
                        if brick_added {
                            if let NodeChildrenArray::OccupancyBitmaps(occupied_bits) =
                                self.node_children[i].content
                            {
                                voxel_maps.extend_from_slice(&[
                                    (occupied_bits[octant] & 0x00000000FFFFFFFF) as u32,
                                    ((occupied_bits[octant] & 0xFFFFFFFF00000000) >> 32) as u32,
                                ]);
                                debug_assert_eq!(
                                    (occupied_bits[octant] & 0x00000000FFFFFFFF) as u32,
                                    voxel_maps[brick_index as usize * 2],
                                    "Expected brick occupied bits lsb to match voxel maps data!",
                                );
                                debug_assert_eq!(
                                    ((occupied_bits[octant] & 0xFFFFFFFF00000000) >> 32) as u32,
                                    voxel_maps[brick_index as usize * 2 + 1],
                                    "Expected brick occupied bits lsb to match voxel maps data!",
                                );
                            } else {
                                panic!("Leaf node is expected to have Occupied bitmap array!");
                            }
                        } else {
                            // If no brick was added, the occupied bits should either be empty or full
                            if let NodeChildrenArray::OccupancyBitmaps(occupied_bits) =
                                self.node_children[i].content
                            {
                                debug_assert!(
                                    occupied_bits[octant] == 0 || occupied_bits[octant] == u64::MAX
                                );
                            }
                        }
                    }
                    children_buffer.extend_from_slice(&children);
                }
                NodeContent::Internal(_) => {
                    for c in 0..8 {
                        let child_index = &self.node_children[i][c];
                        if *child_index != empty_marker() {
                            debug_assert!(map_to_node_index_in_nodes_buffer
                                .contains_key(&(*child_index as usize)));
                            children_buffer.push(
                                map_to_node_index_in_nodes_buffer[&(*child_index as usize)] as u32,
                            );
                        } else {
                            children_buffer.push(*child_index);
                        }
                    }
                }
                NodeContent::Nothing => {} // Nothing to do with an empty node
            }
        }

        debug_assert_eq!(
            voxel_maps.len() / 2,
            voxels.len() / (DIM * DIM * DIM),
            "Voxel occupancy bitmaps length({:?}) should match length of voxel buffer({:?})!",
            voxel_maps.len(),
            voxels.len()
        );

        debug_assert_eq!(
            nodes.len(),
            children_buffer.len() / (8),
            "Node count({:?}) should match length of children buffer({:?})!",
            nodes.len(),
            children_buffer.len()
        );

        ShocoVoxRenderData {
            meta,
            nodes,
            children_buffer,
            voxels,
            voxel_maps,
            color_palette,
        }
    }
}
