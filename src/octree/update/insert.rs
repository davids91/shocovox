use crate::{
    octree::{
        detail::{bound_contains, child_octant_for},
        empty_marker,
        types::{BrickData, NodeChildren, NodeContent, OctreeEntry, OctreeError},
        Octree, VoxelData,
    },
    spatial::{
        lut::{BITMAP_MASK_FOR_OCTANT_LUT, OCTANT_OFFSET_REGION_LUT},
        math::{flat_projection, matrix_index_for, set_occupancy_in_bitmap_64bits, vector::V3c},
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
    //  █████ ██████   █████  █████████  ██████████ ███████████   ███████████
    // ░░███ ░░██████ ░░███  ███░░░░░███░░███░░░░░█░░███░░░░░███ ░█░░░███░░░█
    //  ░███  ░███░███ ░███ ░███    ░░░  ░███  █ ░  ░███    ░███ ░   ░███  ░
    //  ░███  ░███░░███░███ ░░█████████  ░██████    ░██████████      ░███
    //  ░███  ░███ ░░██████  ░░░░░░░░███ ░███░░█    ░███░░░░░███     ░███
    //  ░███  ░███  ░░█████  ███    ░███ ░███ ░   █ ░███    ░███     ░███
    //  █████ █████  ░░█████░░█████████  ██████████ █████   █████    █████
    // ░░░░░ ░░░░░    ░░░░░  ░░░░░░░░░  ░░░░░░░░░░ ░░░░░   ░░░░░    ░░░░░
    //####################################################################################
    /// Inserts the given data into the octree into the given voxel position
    /// If there is already available data it overwrites it, except if all components are empty
    /// If all components are empty, this is a no-op, to erase data, please use @clear
    /// * `position` - the position to insert the data into, must be contained within the tree
    pub fn insert<'a, E: Into<OctreeEntry<'a, T>>>(
        &mut self,
        position: &V3c<u32>,
        data: E,
    ) -> Result<(), OctreeError>
    where
        T: 'a,
    {
        self.insert_internal(true, position, data.into())
    }

    /// Inserts the given data for the octree in the given lod(level of detail) based on insert_size
    /// If there is already available data it overwrites it, except if all components are empty
    /// * `position` - the position to insert the data into, must be contained within the tree
    /// * `insert_size` - The size to update. The value `brick_dimension * (2^x)` is used instead, when size is higher, than brick_dimension
    /// * `data` - The data to insert - cloned if needed
    pub fn insert_at_lod<'a, E: Into<OctreeEntry<'a, T>>>(
        &mut self,
        position: &V3c<u32>,
        insert_size: u32,
        data: E,
    ) -> Result<(), OctreeError>
    where
        T: 'a,
    {
        self.insert_at_lod_internal(true, position, insert_size, data.into())
    }

    /// Updates the given data at the the given voxel position inside the octree
    /// Already available data is untouched, if it is not specified in the entry
    /// If all components are empty, this is a no-op, to erase data, please use @clear
    /// * `position` - the position to insert the data into, must be contained within the tree
    pub fn update<'a, E: Into<OctreeEntry<'a, T>>>(
        &mut self,
        position: &V3c<u32>,
        data: E,
    ) -> Result<(), OctreeError>
    where
        T: 'a,
    {
        self.insert_internal(false, position, data.into())
    }

    pub fn insert_internal(
        &mut self,
        overwrite_if_empty: bool,
        position: &V3c<u32>,
        data: OctreeEntry<T>,
    ) -> Result<(), OctreeError> {
        self.insert_at_lod_internal(overwrite_if_empty, position, 1, data)
    }

    pub fn insert_at_lod_internal(
        &mut self,
        overwrite_if_empty: bool,
        position: &V3c<u32>,
        insert_size: u32,
        data: OctreeEntry<T>,
    ) -> Result<(), OctreeError> {
        let root_bounds = Cube::root_bounds(self.octree_size as f32);
        let position = V3c::<f32>::from(*position);
        if !bound_contains(&root_bounds, &position) {
            return Err(OctreeError::InvalidPosition {
                x: position.x as u32,
                y: position.y as u32,
                z: position.z as u32,
            });
        }

        // Nothing to do when data is empty
        if data.is_none() {
            return Ok(());
        }

        // A CPU stack does not consume significant relevant resources, e.g. a 4096*4096*4096 chunk has depth of 12
        let mut node_stack = vec![(Self::ROOT_NODE_KEY, root_bounds)];
        let mut actual_update_size = 0;
        let target_content = self.add_to_palette(&data);
        loop {
            let (current_node_key, current_bounds) = *node_stack.last().unwrap();
            let current_node_key = current_node_key as usize;
            let target_child_octant = child_octant_for(&current_bounds, &position);
            let target_bounds = Cube {
                min_position: current_bounds.min_position
                    + OCTANT_OFFSET_REGION_LUT[target_child_octant as usize] * current_bounds.size
                        / 2.,
                size: current_bounds.size / 2.,
            };

            let target_child_key = self.node_children[current_node_key].child(target_child_octant);

            if insert_size > 1
                && target_bounds.size <= insert_size as f32
                && position <= target_bounds.min_position
            {
                // Whole child node to be overwritten with data
                // Occupied bits are correctly set in post-processing
                if let NodeContent::Leaf(_) | NodeContent::UniformLeaf(_) =
                    self.nodes.get(current_node_key)
                {
                    self.subdivide_leaf_to_nodes(current_node_key, target_child_octant as usize);
                }

                if self.nodes.key_is_valid(target_child_key) {
                    self.deallocate_children_of(target_child_key);
                    *self.nodes.get_mut(target_child_key) =
                        NodeContent::UniformLeaf(BrickData::Solid(target_content));
                    self.node_children[target_child_key as usize] =
                        NodeChildren::OccupancyBitmap(u64::MAX);
                } else {
                    // Push in a new uniform leaf child
                    let new_child_index = self
                        .nodes
                        .push(NodeContent::UniformLeaf(BrickData::Solid(target_content)))
                        as u32;
                    self.node_children.resize(
                        self.node_children.len().max(new_child_index as usize + 1),
                        NodeChildren::default(),
                    );
                    self.node_mips
                        .resize(self.node_mips.len().max(self.nodes.len()), BrickData::Empty);
                    *self.node_children[current_node_key]
                        .child_mut(target_child_octant as usize)
                        .unwrap() = new_child_index;
                    self.node_children[new_child_index as usize] =
                        NodeChildren::OccupancyBitmap(u64::MAX);
                }
                actual_update_size = target_bounds.size as usize;
                break;
            }

            // iteration needs to go deeper, as current node is not a leaf,
            // and target size is still larger, than brick dimension.
            // The whole node can't be overwritten because that case was handled before this
            if target_bounds.size > self.brick_dim as f32
                && (!matches!(
                    self.nodes.get(current_node_key),
                    NodeContent::Leaf(_) | NodeContent::UniformLeaf(_)
                ) || self.node_children[current_node_key].child(target_child_octant)
                    == empty_marker::<u32>() as usize)
            {
                // the child at the queried position exists and valid, recurse into it
                if self.nodes.key_is_valid(target_child_key) {
                    node_stack.push((
                        self.node_children[current_node_key].child(target_child_octant) as u32,
                        target_bounds,
                    ));
                } else {
                    // no children are available for the target octant while
                    // current node size is still larger, than the requested size
                    if matches!(
                        self.nodes.get(current_node_key),
                        NodeContent::Leaf(_) | NodeContent::UniformLeaf(_)
                    ) {
                        // The current Node is a leaf, representing the area under current_bounds
                        // filled with the data stored in NodeContent::*Leaf(_)
                        let target_match = match self.nodes.get(current_node_key) {
                            NodeContent::Internal(_) | NodeContent::Nothing => false,
                            NodeContent::UniformLeaf(brick) => match brick {
                                BrickData::Empty => false,
                                BrickData::Solid(voxel) => *voxel == target_content,
                                BrickData::Parted(brick) => {
                                    let index_in_matrix = matrix_index_for(
                                        &current_bounds,
                                        &(position.into()),
                                        self.brick_dim,
                                    );
                                    let index_in_matrix = flat_projection(
                                        index_in_matrix.x,
                                        index_in_matrix.y,
                                        index_in_matrix.z,
                                        self.brick_dim as usize,
                                    );
                                    brick[index_in_matrix] == target_content
                                }
                            },
                            NodeContent::Leaf(bricks) => {
                                match &bricks[target_child_octant as usize] {
                                    BrickData::Empty => false,
                                    BrickData::Solid(voxel) => *voxel == target_content,
                                    BrickData::Parted(brick) => {
                                        let index_in_matrix = matrix_index_for(
                                            &target_bounds,
                                            &(position.into()),
                                            self.brick_dim,
                                        );
                                        let index_in_matrix = flat_projection(
                                            index_in_matrix.x,
                                            index_in_matrix.y,
                                            index_in_matrix.z,
                                            self.brick_dim as usize,
                                        );
                                        brick[index_in_matrix] == target_content
                                    }
                                }
                            }
                        };

                        if target_match || self.nodes.get(current_node_key).is_all(&target_content)
                        {
                            // the data stored equals the given data, at the requested position
                            // so no need to continue iteration as data already matches
                            break;
                        }

                        // The contained data does not match the given data at the given position,
                        // but the current node is a leaf, so it needs to be divided into separate nodes
                        // with its children having the same data as the current node to keep integrity
                        self.subdivide_leaf_to_nodes(
                            current_node_key,
                            target_child_octant as usize,
                        );

                        node_stack.push((
                            self.node_children[current_node_key].child(target_child_octant) as u32,
                            target_bounds,
                        ));
                    } else {
                        // current Node is a non-leaf Node, which doesn't have the child at the requested position,
                        // so it is inserted and the Node becomes non-empty
                        match self.nodes.get(current_node_key) {
                            NodeContent::Nothing => {
                                // A special case during the first insertion, where the root Node was empty beforehand
                                *self.nodes.get_mut(current_node_key) = NodeContent::Internal(0);
                            }
                            NodeContent::Internal(_occupied_bits) => {} // Nothing to do
                            NodeContent::Leaf(_) | NodeContent::UniformLeaf(_) => {
                                panic!("Leaf Node expected to be non-leaf!");
                            }
                        }

                        // Insert a new child Node
                        let new_child_node = self.nodes.push(NodeContent::Nothing) as u32;

                        // Update node_children to reflect the inserted node
                        self.node_children.resize(
                            self.node_children.len().max(self.nodes.len()),
                            NodeChildren::default(),
                        );
                        self.node_mips
                            .resize(self.node_mips.len().max(self.nodes.len()), BrickData::Empty);
                        *self.node_children[current_node_key]
                            .child_mut(target_child_octant as usize)
                            .unwrap() = new_child_node;

                        // The occupancy bitmap of the node will be updated
                        // in the next iteration or in the post-processing logic
                        node_stack.push((new_child_node, target_bounds));
                    }
                }
            } else {
                actual_update_size = self.leaf_update(
                    overwrite_if_empty,
                    current_node_key,
                    &current_bounds,
                    &target_bounds,
                    target_child_octant as usize,
                    &(position.into()),
                    insert_size,
                    target_content,
                );

                break;
            }
        }

        // post-processing operations
        let mut simplifyable = self.auto_simplify; // Don't even start to simplify if it's disabled
        for (node_key, node_bounds) in node_stack.into_iter().rev() {
            if !self.nodes.key_is_valid(node_key as usize) {
                continue;
            }

            // In case any node is NodeContent::Nothing, it is to be converted to an internal node
            if let NodeContent::Nothing = self.nodes.get(node_key as usize) {
                *self.nodes.get_mut(node_key as usize) = NodeContent::Internal(0);
            }

            // Update Node occupied bits
            let mut new_occupied_bits = self.stored_occupied_bits(node_key as usize);
            if node_bounds.size as usize == actual_update_size {
                new_occupied_bits = u64::MAX;
            } else {
                set_occupancy_in_bitmap_64bits(
                    &((position - node_bounds.min_position).into()),
                    actual_update_size,
                    node_bounds.size as usize,
                    true,
                    &mut new_occupied_bits,
                );
            }
            #[cfg(debug_assertions)]
            {
                if let NodeContent::Leaf(bricks) = self.nodes.get(node_key as usize) {
                    for octant in 0..8 {
                        if let BrickData::Solid(_) | BrickData::Empty = bricks[octant] {
                            // with solid and empty bricks, the relevant occupied bits should either be empty or full
                            if let NodeChildren::OccupancyBitmap(occupied_bits) =
                                self.node_children[node_key as usize]
                            {
                                debug_assert!(
                                        0 == occupied_bits & BITMAP_MASK_FOR_OCTANT_LUT[octant]
                                            || BITMAP_MASK_FOR_OCTANT_LUT[octant]
                                                == occupied_bits
                                                    & BITMAP_MASK_FOR_OCTANT_LUT[octant],
                                        "Brickdata at octant[{:?}] doesn't match occupied bits: {:?} <> ({:#10X} & {:#10X} ==> {:#10X})",
                                        octant,
                                        bricks[octant],
                                        occupied_bits,
                                        BITMAP_MASK_FOR_OCTANT_LUT[octant],
                                        (occupied_bits & BITMAP_MASK_FOR_OCTANT_LUT[octant])
                                    );
                            }
                        }
                    }
                }
            }
            self.store_occupied_bits(node_key as usize, new_occupied_bits);

            // update MIP maps
            self.update_mip(node_key as usize, &node_bounds, &(position.into()));

            // Decide to continue or not
            if matches!(
                self.nodes.get(node_key as usize),
                NodeContent::Leaf(_) | NodeContent::UniformLeaf(_)
            ) {
                // In case of leaf nodes, just try to simplify and continue
                simplifyable = self.simplify(node_key as usize);
                continue;
            }

            if simplifyable {
                simplifyable = self.simplify(node_key as usize); // If any Nodes fail to simplify, no need to continue because their parents can not be simplified because of it
            }
        }
        Ok(())
    }
}
