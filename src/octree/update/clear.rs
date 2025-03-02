use crate::{
    object_pool::empty_marker,
    octree::{
        detail::{bound_contains, child_octant_for},
        types::{BrickData, NodeChildren, NodeContent, OctreeError, PaletteIndexValues},
        Octree, VoxelData,
    },
    spatial::{
        lut::OCTANT_OFFSET_REGION_LUT,
        math::{
            flat_projection, hash_region, matrix_index_for, set_occupancy_in_bitmap_64bits,
            vector::V3c, BITMAP_DIMENSION,
        },
        Cube,
    },
};
use std::hash::Hash;

#[cfg(feature = "bytecode")]
use bendy::{decoding::FromBencode, encoding::ToBencode};

impl<
        #[cfg(all(feature = "bytecode", feature = "serialization"))] T: FromBencode
            + ToBencode
            + Serialize
            + DeserializeOwned
            + Default
            + Eq
            + Clone
            + Hash
            + VoxelData,
        #[cfg(all(feature = "bytecode", not(feature = "serialization")))] T: FromBencode + ToBencode + Default + Eq + Clone + Hash + VoxelData,
        #[cfg(all(not(feature = "bytecode"), feature = "serialization"))] T: Serialize + DeserializeOwned + Default + Eq + Clone + Hash + VoxelData,
        #[cfg(all(not(feature = "bytecode"), not(feature = "serialization")))] T: Default + Eq + Clone + Hash + VoxelData,
    > Octree<T>
{
    //####################################################################################
    //    █████████  █████       ██████████   █████████   ███████████
    //   ███░░░░░███░░███       ░░███░░░░░█  ███░░░░░███ ░░███░░░░░███
    //  ███     ░░░  ░███        ░███  █ ░  ░███    ░███  ░███    ░███
    // ░███          ░███        ░██████    ░███████████  ░██████████
    // ░███          ░███        ░███░░█    ░███░░░░░███  ░███░░░░░███
    // ░░███     ███ ░███      █ ░███ ░   █ ░███    ░███  ░███    ░███
    //  ░░█████████  ███████████ ██████████ █████   █████ █████   █████
    //   ░░░░░░░░░  ░░░░░░░░░░░ ░░░░░░░░░░ ░░░░░   ░░░░░ ░░░░░   ░░░░░
    //####################################################################################
    /// clears the voxel at the given position
    pub fn clear(&mut self, position: &V3c<u32>) -> Result<(), OctreeError> {
        self.clear_at_lod(position, 1)
    }

    /// Clears the data at the given position and lod size
    /// * `position` - the position to insert data into, must be contained within the tree
    /// * `clear_size` - The size to update. The value `brick_dimension * (2^x)` is used instead, when size is higher, than brick_dimension
    pub fn clear_at_lod(
        &mut self,
        position: &V3c<u32>,
        clear_size: u32,
    ) -> Result<(), OctreeError> {
        let root_bounds = Cube::root_bounds(self.octree_size as f32);
        if !bound_contains(&root_bounds, &V3c::from(*position)) {
            return Err(OctreeError::InvalidPosition {
                x: position.x,
                y: position.y,
                z: position.z,
            });
        }

        // A CPU stack does not consume significant relevant resources, e.g. a 4096*4096*4096 chunk has depth of 12
        let mut node_stack = vec![(Self::ROOT_NODE_KEY, root_bounds)];
        let mut actual_update_size = 0;
        loop {
            let (current_node_key, current_bounds) = *node_stack.last().unwrap();
            let current_node_key = current_node_key as usize;
            let target_child_octant = child_octant_for(&current_bounds, &V3c::from(*position));
            let target_bounds = Cube {
                min_position: current_bounds.min_position
                    + OCTANT_OFFSET_REGION_LUT[target_child_octant as usize] * current_bounds.size
                        / 2.,
                size: current_bounds.size / 2.,
            };
            let target_child_key = self.node_children[current_node_key].child(target_child_octant);

            if clear_size > 1
                && target_bounds.size <= clear_size as f32
                && *position <= target_bounds.min_position.into()
                && self.nodes.key_is_valid(target_child_key)
            {
                // The whole node to be erased
                // Parent occupied bits are correctly set in post-processing
                if self.nodes.key_is_valid(target_child_key) {
                    self.deallocate_children_of(target_child_key);
                    *self.nodes.get_mut(target_child_key) = NodeContent::Nothing;
                    actual_update_size = target_bounds.size as usize;

                    node_stack.push((
                        self.node_children[current_node_key].child(target_child_octant) as u32,
                        target_bounds,
                    ));
                }
                // If the target child is empty, there's nothing to do and the targeted area is empty already
                break;
            }

            if target_bounds.size > clear_size.max(self.brick_dim) as f32 {
                // iteration needs to go deeper, as current Node size is still larger, than the requested clear size
                if self.nodes.key_is_valid(target_child_key) {
                    //Iteration can go deeper , as target child is valid
                    node_stack.push((
                        self.node_children[current_node_key].child(target_child_octant) as u32,
                        target_bounds,
                    ));
                } else {
                    // no children are available for the target octant
                    if matches!(
                        self.nodes.get(current_node_key),
                        NodeContent::Leaf(_) | NodeContent::UniformLeaf(_)
                    ) {
                        // The current Node is a leaf, representing the area under current_bounds
                        // filled with the data stored in NodeContent::*Leaf(_)
                        let target_match = match self.nodes.get(current_node_key) {
                            NodeContent::Nothing | NodeContent::Internal(_) => {
                                panic!("Non-leaf node expected to be leaf!")
                            }
                            NodeContent::UniformLeaf(brick) => match brick {
                                BrickData::Empty => true,
                                BrickData::Solid(voxel) => NodeContent::pix_points_to_empty(
                                    voxel,
                                    &self.voxel_color_palette,
                                    &self.voxel_data_palette,
                                ),
                                BrickData::Parted(brick) => {
                                    let index_in_matrix =
                                        *position - V3c::from(current_bounds.min_position);
                                    let index_in_matrix = flat_projection(
                                        index_in_matrix.x as usize,
                                        index_in_matrix.y as usize,
                                        index_in_matrix.z as usize,
                                        self.brick_dim as usize,
                                    );
                                    NodeContent::pix_points_to_empty(
                                        &brick[index_in_matrix],
                                        &self.voxel_color_palette,
                                        &self.voxel_data_palette,
                                    )
                                }
                            },
                            NodeContent::Leaf(bricks) => {
                                match &bricks[target_child_octant as usize] {
                                    BrickData::Empty => true,
                                    BrickData::Solid(voxel) => NodeContent::pix_points_to_empty(
                                        voxel,
                                        &self.voxel_color_palette,
                                        &self.voxel_data_palette,
                                    ),
                                    BrickData::Parted(brick) => {
                                        let index_in_matrix =
                                            *position - V3c::from(current_bounds.min_position);
                                        let index_in_matrix = flat_projection(
                                            index_in_matrix.x as usize,
                                            index_in_matrix.y as usize,
                                            index_in_matrix.z as usize,
                                            self.brick_dim as usize,
                                        );
                                        NodeContent::pix_points_to_empty(
                                            &brick[index_in_matrix],
                                            &self.voxel_color_palette,
                                            &self.voxel_data_palette,
                                        )
                                    }
                                }
                            }
                        };
                        if target_match
                            || self
                                .nodes
                                .get(current_node_key)
                                .is_empty(&self.voxel_color_palette, &self.voxel_data_palette)
                        {
                            // the data stored equals the given data, at the requested position
                            // so no need to continue iteration as data already matches
                            break;
                        }

                        // The contained data does not match the given data at the given position,
                        // but the current node is a leaf, so it needs to be divided into separate nodes
                        // with its children having the same data as the current node, to keep integrity.
                        // It needs to be separated because it has an extent above DIM
                        debug_assert!(
                            current_bounds.size > self.brick_dim as f32,
                            "Expected Leaf node to have an extent({:?}) above DIM({:?})!",
                            current_bounds.size,
                            self.brick_dim
                        );
                        self.subdivide_leaf_to_nodes(
                            current_node_key,
                            target_child_octant as usize,
                        );

                        node_stack.push((
                            self.node_children[current_node_key].child(target_child_octant) as u32,
                            target_bounds,
                        ));
                    } else {
                        // current Node is a non-leaf Node, which doesn't have the child at the requested position.
                        // Nothing to do, because child didn't exist in the first place
                        break;
                    }
                }
            } else {
                // when clearing Nodes with size > DIM, Nodes are being cleared
                // current_bounds.size == min_node_size, which is the desired depth
                actual_update_size = self.leaf_update(
                    true,
                    current_node_key,
                    &current_bounds,
                    &target_bounds,
                    target_child_octant as usize,
                    position,
                    clear_size,
                    empty_marker::<PaletteIndexValues>(),
                );

                break;
            }
        }

        // post-processing operations
        // If a whole node was removed in the operation, it has to be cleaned up properly
        let mut removed_node = if let Some((child_key, child_bounds)) = node_stack.pop() {
            if child_bounds.size as usize <= actual_update_size {
                Some((child_key, child_bounds))
            } else {
                None
            }
        } else {
            None
        };
        let mut simplifyable = self.auto_simplify; // Don't even start to simplify if it's disabled
        for (node_key, node_bounds) in node_stack.into_iter().rev() {
            if let Some((child_key, child_bounds)) = removed_node {
                // If the child of this node was set to NodeContent::Nothing during this clear operation
                // it needs to be freed up, and the child index of this node needs to be updated as well
                let child_octant = hash_region(
                    &((child_bounds.min_position - node_bounds.min_position)
                        + V3c::unit(child_bounds.size / 2.)),
                    node_bounds.size / 2.,
                ) as usize;
                self.node_children[node_key as usize].clear(child_octant);
                self.nodes.free(child_key as usize);
                // Occupancy bitmask is re-evaluated fully in the below blocks
                removed_node = None;
            };

            let previous_occupied_bits = self.stored_occupied_bits(node_key as usize);
            let mut new_occupied_bits =
                if let NodeChildren::NoChildren = self.node_children[node_key as usize] {
                    0
                } else {
                    previous_occupied_bits
                };

            if node_bounds.size as usize == actual_update_size {
                new_occupied_bits = 0;
            } else {
                // Calculate the new occupied bits of the node
                let start_in_bitmap =
                    matrix_index_for(&node_bounds, position, BITMAP_DIMENSION as u32);
                let bitmap_update_size = (actual_update_size as f32 * BITMAP_DIMENSION as f32
                    / node_bounds.size)
                    .ceil() as usize;
                for x in start_in_bitmap.x
                    ..(start_in_bitmap.x + bitmap_update_size).min(BITMAP_DIMENSION)
                {
                    for y in start_in_bitmap.y
                        ..(start_in_bitmap.y + bitmap_update_size).min(BITMAP_DIMENSION)
                    {
                        for z in start_in_bitmap.z
                            ..(start_in_bitmap.z + bitmap_update_size).min(BITMAP_DIMENSION)
                        {
                            if self.should_bitmap_be_empty_at_bitmap_index(
                                node_key as usize,
                                &V3c::new(x, y, z),
                            ) {
                                set_occupancy_in_bitmap_64bits(
                                    &V3c::new(x, y, z),
                                    1,
                                    BITMAP_DIMENSION,
                                    false,
                                    &mut new_occupied_bits,
                                );
                            }
                        }
                    }
                }
            }

            *self.nodes.get_mut(node_key as usize) = if 0 != new_occupied_bits
                && matches!(
                    self.node_children[node_key as usize],
                    NodeChildren::Children(_)
                ) {
                NodeContent::Internal(new_occupied_bits)
            } else {
                //Occupied bits depleted to 0x0
                for child_octant in 0..8 {
                    debug_assert!(self.node_empty_at(node_key as usize, child_octant));
                }
                self.deallocate_children_of(node_key as usize);
                removed_node = Some((node_key, node_bounds));
                NodeContent::Nothing
            };
            debug_assert!(
                0 != new_occupied_bits
                    || matches!(self.nodes.get(node_key as usize), NodeContent::Nothing),
                "Occupied bits doesn't match node[{:?}]: {:?} <> {:?}\nnode children: {:?}",
                node_key,
                new_occupied_bits,
                self.nodes.get(node_key as usize),
                self.node_children[node_key as usize]
            );

            if 0 == new_occupied_bits {
                self.node_children[node_key as usize] = NodeChildren::NoChildren;
            } else {
                self.store_occupied_bits(node_key as usize, new_occupied_bits);
            }

            // update MIP maps
            self.update_mip(node_key as usize, &node_bounds, position);

            // Decide to continue or not
            if simplifyable {
                // If any Nodes fail to simplify, no need to continue because their parents can not be simplified further
                simplifyable = self.simplify(node_key as usize);
            }
            if previous_occupied_bits == new_occupied_bits {
                // In case the occupied bits were not modified, there's no need to continue
                break;
            }
        }
        Ok(())
    }
}
