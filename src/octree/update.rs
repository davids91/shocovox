use crate::object_pool::empty_marker;
use crate::octree::types::{BrickData, NodeChildrenArray};
use crate::octree::{
    detail::{bound_contains, child_octant_for},
    types::{NodeChildren, NodeContent, OctreeError},
    Octree, VoxelData,
};
use crate::spatial::{
    math::{
        hash_region, matrix_index_for, offset_region, set_occupancy_in_bitmap_64bits, vector::V3c,
        BITMAP_DIMENSION,
    },
    Cube,
};

impl<T, const DIM: usize> Octree<T, DIM>
where
    T: Default + PartialEq + Clone + Copy + PartialEq + VoxelData,
{
    /// Updates the given node to be a Leaf, and inserts the provided data for it.
    /// It will update a whole node, or maximum one brick. Brick update range is starting from the position,
    /// goes up to the extent of the brick. Does not set occupancy bitmap of the given node.
    /// Returns with the size of the actual update
    fn leaf_update(
        &mut self,
        node_key: usize,
        node_bounds: &Cube,
        target_bounds: &Cube,
        target_child_octant: usize,
        position: &V3c<u32>,
        size: usize,
        data: Option<T>,
    ) -> usize {
        // Update the leaf node, if it is possible as is, and if it's even needed to update
        // and decide if the node content needs to be divided into bricks, and the update function to be called again
        if size > 1 && size as f32 >= node_bounds.size {
            // The whole node to be covered in the given data
            if let Some(data) = data {
                *self.nodes.get_mut(node_key) = NodeContent::UniformLeaf(BrickData::Solid(data));
            } else {
                *self.nodes.get_mut(node_key) = NodeContent::Nothing;
            }
            return node_bounds.size as usize;
        }

        match self.nodes.get_mut(node_key) {
            NodeContent::Leaf(bricks) => {
                // In case DIM == octree size, the root node can not be a leaf...
                debug_assert!(DIM < self.octree_size as usize);
                match &mut bricks[target_child_octant] {
                    //If there is no brick in the target position of the leaf, create one
                    BrickData::Empty => {
                        // Create a new empty brick at the given octant
                        let mut new_brick = Box::new([[[T::default(); DIM]; DIM]; DIM]);
                        // update the new empty brick at the given position
                        let update_size =
                            Self::update_brick(&mut new_brick, target_bounds, position, size, data);
                        bricks[target_child_octant] = BrickData::Parted(new_brick);
                        update_size
                    }
                    BrickData::Solid(voxel) => {
                        // In case the data doesn't match the current contents of the node, it needs to be subdivided
                        let update_size;
                        if (data.is_none() && !voxel.is_empty())
                            || (data.is_some() && data.unwrap() != *voxel)
                        {
                            // create new brick and update it at the given position
                            let mut new_brick = Box::new([[[*voxel; DIM]; DIM]; DIM]);
                            update_size = Self::update_brick(
                                &mut new_brick,
                                target_bounds,
                                position,
                                size,
                                data,
                            );
                            bricks[target_child_octant] = BrickData::Parted(new_brick);
                        } else {
                            // Since the Voxel already equals the data to be set, no need to update anything
                            update_size = 0;
                        }
                        update_size
                    }
                    BrickData::Parted(ref mut brick) => {
                        // Simply update the brick at the given position
                        Self::update_brick(brick, target_bounds, position, size, data)
                    }
                }
            }
            NodeContent::UniformLeaf(ref mut mat) => {
                match mat {
                    BrickData::Empty => {
                        debug_assert_eq!(
                            self.node_children[node_key].content,
                            NodeChildrenArray::OccupancyBitmap(0),
                            "Expected Node OccupancyBitmap(0) for empty leaf node instead of {:?}",
                            self.node_children[node_key].content
                        );
                        if data.is_some() {
                            let mut new_leaf_content = [
                                BrickData::Empty,
                                BrickData::Empty,
                                BrickData::Empty,
                                BrickData::Empty,
                                BrickData::Empty,
                                BrickData::Empty,
                                BrickData::Empty,
                                BrickData::Empty,
                            ];

                            // Add a brick to the target octant and update with the given data
                            let mut new_brick = Box::new([[[T::default(); DIM]; DIM]; DIM]);
                            Self::update_brick(&mut new_brick, target_bounds, position, size, data);
                            new_leaf_content[target_child_octant] = BrickData::Parted(new_brick);
                            *self.nodes.get_mut(node_key) = NodeContent::Leaf(new_leaf_content);
                        }
                    }
                    BrickData::Solid(voxel) => {
                        debug_assert!(
                            !voxel.is_empty()
                                && (self.node_children[node_key].content
                                    == NodeChildrenArray::OccupancyBitmap(u64::MAX))
                                || voxel.is_empty()
                                    && (self.node_children[node_key].content
                                        == NodeChildrenArray::OccupancyBitmap(0)),
                            "Expected Node occupancy bitmap({:?}) to align for Solid Voxel Brick in Uniform Leaf, which is {}",
                            self.node_children[node_key].content,
                            if voxel.is_empty() {
                                "empty"
                            } else {
                                "not empty"
                            }
                        );

                        // In case the data request doesn't match node content, it needs to be subdivided
                        if data.is_none() && voxel.is_empty() {
                            // Data request is to clear, it aligns with the voxel content,
                            // it's enough to update the node content in this case
                            *self.nodes.get_mut(node_key) = NodeContent::Nothing;
                            return 0;
                        }

                        if data.is_some_and(|d| d != *voxel)
                            || (data.is_none() && !voxel.is_empty())
                        {
                            // Data request doesn't align with the voxel data
                            // create a voxel brick and try to update with the given data
                            *mat = BrickData::Parted(Box::new([[[*voxel; DIM]; DIM]; DIM]));

                            return self.leaf_update(
                                node_key,
                                node_bounds,
                                target_bounds,
                                target_child_octant,
                                position,
                                size,
                                data,
                            );
                        }

                        // data request aligns with node content
                        return 0;
                    }
                    BrickData::Parted(brick) => {
                        // Check if the voxel at the target position matches with the data update request
                        // The target position index is to be calculated from the node bounds,
                        // instead of the target bounds because the position should cover the whole leaf
                        // not just one brick in it
                        let mat_index = matrix_index_for(node_bounds, position, DIM);
                        let target_voxel = brick[mat_index.x][mat_index.y][mat_index.z];
                        if 1 < DIM // BrickData can only stay parted if DIM is above 1
                            && (data.is_none() && target_voxel.is_empty()
                                || data.is_some_and(|d| d == target_voxel))
                        {
                            // Target voxel matches with the data request, there's nothing to do!
                            return 0;
                        }

                        // the data at the position inside the brick doesn't match the given data,
                        // so the leaf needs to be divided into a NodeContent::Leaf(bricks)
                        let mut leaf_data: [BrickData<T, DIM>; 8] = [
                            BrickData::Empty,
                            BrickData::Empty,
                            BrickData::Empty,
                            BrickData::Empty,
                            BrickData::Empty,
                            BrickData::Empty,
                            BrickData::Empty,
                            BrickData::Empty,
                        ];

                        // Each brick is mapped to take up one subsection of the current data
                        let mut update_size = 0;
                        for octant in 0..8usize {
                            let brick_offset =
                                V3c::<usize>::from(offset_region(octant as u8)) * (2.min(DIM - 1));
                            let mut new_brick = Box::new(
                                [[[brick[brick_offset.x][brick_offset.y][brick_offset.z]; DIM];
                                    DIM]; DIM],
                            );
                            for x in 0..DIM {
                                for y in 0..DIM {
                                    for z in 0..DIM {
                                        if x < 2 && y < 2 && z < 2 {
                                            continue;
                                        }
                                        new_brick[x][y][z] = brick[brick_offset.x + x / 2]
                                            [brick_offset.y + y / 2][brick_offset.z + z / 2];
                                    }
                                }
                            }

                            // Also update the brick if it is the target
                            if octant == target_child_octant {
                                update_size = Self::update_brick(
                                    &mut new_brick,
                                    target_bounds,
                                    position,
                                    size,
                                    data,
                                );
                            }

                            leaf_data[octant] = BrickData::Parted(new_brick)
                        }

                        *self.nodes.get_mut(node_key) = NodeContent::Leaf(leaf_data);
                        debug_assert_ne!(
                            0, update_size,
                            "Expected Leaf node to be updated in operation"
                        );
                        return update_size;
                    }
                }
                self.leaf_update(
                    node_key,
                    node_bounds,
                    target_bounds,
                    target_child_octant,
                    position,
                    size,
                    data,
                )
            }
            NodeContent::Nothing => {
                *self.nodes.get_mut(node_key) = NodeContent::Leaf([
                    BrickData::Empty,
                    BrickData::Empty,
                    BrickData::Empty,
                    BrickData::Empty,
                    BrickData::Empty,
                    BrickData::Empty,
                    BrickData::Empty,
                    BrickData::Empty,
                ]);
                self.leaf_update(
                    node_key,
                    node_bounds,
                    target_bounds,
                    target_child_octant,
                    position,
                    size,
                    data,
                )
            }
            NodeContent::Internal(_occupied_bits) => {
                panic!("Leaf update should not be dealing with internal nodes!")
            }
        }
    }

    /// Updates the content of the given brick and its occupancy bitmap. Each components of mat_index must be smaller, than the size of the brick.
    /// mat_index + size however need not be in bounds, the function will cut each component to fit inside the brick.
    /// * `brick` - mutable reference of the brick to update
    /// * `mat_index` - the first position to update with the given data
    /// * `size` - the number of elements in x,y,z to update with the given data
    /// * `data` - the data  to update the brick with. Erases data in case `None`
    /// * Returns with the size of the update
    fn update_brick(
        brick: &mut [[[T; DIM]; DIM]; DIM],
        brick_bounds: &Cube,
        position: &V3c<u32>,
        size: usize,
        data: Option<T>,
    ) -> usize {
        let size = size.min(DIM);
        let mat_index =
            matrix_index_for(brick_bounds, position, DIM).cut_each_component(&(DIM - size));

        for x in mat_index.x..(mat_index.x + size) {
            for y in mat_index.y..(mat_index.y + size) {
                for z in mat_index.z..(mat_index.z + size) {
                    if let Some(data) = data {
                        brick[x][y][z] = data;
                    } else {
                        brick[x][y][z].clear();
                    }
                }
            }
        }
        size
    }

    /// Inserts the given data into the octree into the intended voxel position
    pub fn insert(&mut self, position: &V3c<u32>, data: T) -> Result<(), OctreeError> {
        self.insert_at_lod(position, 1, data)
    }

    /// Sets the given data for the octree in the given lod(level of detail) based on insert_size
    /// * `position` - the position to insert data into, must be contained within the tree
    /// * `insert_size` - The size of the part to update, counts as one of `DIM * (2^x)` when higher, than DIM
    /// * `data` - The data to insert - cloned if needed
    pub fn insert_at_lod(
        &mut self,
        position: &V3c<u32>,
        insert_size: u32,
        data: T,
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
        if data.is_empty() {
            return Ok(());
        }

        // A CPU stack does not consume significant relevant resources, e.g. a 4096*4096*4096 chunk has depth of 12
        let mut node_stack = vec![(Octree::<T, DIM>::ROOT_NODE_KEY, root_bounds)];
        let mut actual_update_size = 0;
        loop {
            let (current_node_key, current_bounds) = *node_stack.last().unwrap();
            let current_node_key = current_node_key as usize;
            let target_child_octant = child_octant_for(&current_bounds, &position);
            let target_bounds = Cube {
                min_position: current_bounds.min_position
                    + offset_region(target_child_octant) * current_bounds.size / 2.,
                size: current_bounds.size / 2.,
            };

            // iteration needs to go deeper, as current target size is still larger, than the requested
            let current_node = self.nodes.get(current_node_key);
            let target_child_key =
                self.node_children[current_node_key][target_child_octant as u32] as usize;
            if target_bounds.size > insert_size.max(DIM as u32) as f32
                || self.is_node_internal(current_node_key)
            // Complex internal nodes further reduce possible update size
            {
                // the child at the queried position exists and valid, recurse into it
                if self.nodes.key_is_valid(target_child_key) {
                    node_stack.push((
                        self.node_children[current_node_key][target_child_octant as u32],
                        target_bounds,
                    ));
                } else {
                    // no children are available for the target octant while
                    // current node size is still larger, than the requested size
                    if matches!(
                        current_node,
                        NodeContent::Leaf(_) | NodeContent::UniformLeaf(_)
                    ) {
                        // The current Node is a leaf, representing the area under current_bounds
                        // filled with the data stored in NodeContent::*Leaf(_)
                        let target_match = match current_node {
                            NodeContent::Nothing | NodeContent::Internal(_) => {
                                panic!("Non-leaf node expected to be leaf!")
                            }
                            NodeContent::UniformLeaf(brick) => match brick {
                                BrickData::Empty => false,
                                BrickData::Solid(voxel) => *voxel == data,
                                BrickData::Parted(brick) => {
                                    let index_in_matrix =
                                        matrix_index_for(&current_bounds, &(position.into()), DIM);
                                    brick[index_in_matrix.x][index_in_matrix.y][index_in_matrix.z]
                                        == data
                                }
                            },
                            NodeContent::Leaf(bricks) => {
                                match &bricks[target_child_octant as usize] {
                                    BrickData::Empty => false,
                                    BrickData::Solid(voxel) => *voxel == data,
                                    BrickData::Parted(brick) => {
                                        let index_in_matrix = position - target_bounds.min_position;
                                        brick[index_in_matrix.x as usize]
                                            [index_in_matrix.y as usize]
                                            [index_in_matrix.z as usize]
                                            == data
                                    }
                                }
                            }
                        };

                        if target_match || current_node.is_all(&data) {
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
                            self.node_children[current_node_key][target_child_octant as u32],
                            target_bounds,
                        ));
                    } else {
                        // current Node is a non-leaf Node, which doesn't have the child at the requested position,
                        // so it is inserted and the Node becomes non-empty
                        match current_node {
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
                            NodeChildren::new(empty_marker()),
                        );
                        self.node_children[current_node_key][target_child_octant as u32] =
                            new_child_node;

                        // The occupancy bitmap of the node will be updated
                        // in the next iteration or in the post-processing logic
                        node_stack.push((new_child_node, target_bounds));
                    }
                }
            } else {
                // target_bounds.size <= min_node_size, which is the desired depth!
                actual_update_size = self.leaf_update(
                    current_node_key,
                    &current_bounds,
                    &target_bounds,
                    target_child_octant as usize,
                    &(position.into()),
                    insert_size as usize,
                    Some(data),
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

            // Update Node occupied bits in case of internal nodes
            if let NodeContent::Internal(ref mut occupied_bits) =
                self.nodes.get_mut(node_key as usize)
            {
                let corrected_update_size = ((node_bounds.size * actual_update_size as f32)
                    / BITMAP_DIMENSION as f32)
                    .ceil() as usize;
                set_occupancy_in_bitmap_64bits(
                    &matrix_index_for(&node_bounds, &(position.into()), BITMAP_DIMENSION),
                    corrected_update_size,
                    BITMAP_DIMENSION,
                    true,
                    occupied_bits,
                );
            } else {
                // Update the occupied bits of the leaf node
                let mut new_occupied_bits = self.stored_occupied_bits(node_key as usize);
                if node_bounds.size as usize == actual_update_size {
                    new_occupied_bits = u64::MAX;
                } else {
                    let corrected_update_size = ((node_bounds.size * actual_update_size as f32)
                        / (DIM as f32 * 2.))
                        .ceil() as usize;
                    set_occupancy_in_bitmap_64bits(
                        &matrix_index_for(&node_bounds, &(position.into()), DIM * 2),
                        corrected_update_size,
                        DIM * 2,
                        true,
                        &mut new_occupied_bits,
                    );
                }
                self.store_occupied_bits(node_key as usize, new_occupied_bits);
            }
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

    /// clears the voxel at the given position
    pub fn clear(&mut self, position: &V3c<u32>) -> Result<(), OctreeError> {
        self.clear_at_lod(position, 1)
    }

    /// Clears the data at the given position and lod size
    /// * `position` - the position to insert data into, must be contained within the tree
    /// * `clear_size` - The size of the part to clear, counts as one of `DIM * (2^x)` when higher, than DIM
    pub fn clear_at_lod(
        &mut self,
        position: &V3c<u32>,
        clear_size: u32,
    ) -> Result<(), OctreeError> {
        let position = V3c::<f32>::from(*position);
        let root_bounds = Cube::root_bounds(self.octree_size as f32);
        if !bound_contains(&root_bounds, &position) {
            return Err(OctreeError::InvalidPosition {
                x: position.x as u32,
                y: position.y as u32,
                z: position.z as u32,
            });
        }

        // A CPU stack does not consume significant relevant resources, e.g. a 4096*4096*4096 chunk has depth of 12
        let mut node_stack = vec![(Octree::<T, DIM>::ROOT_NODE_KEY, root_bounds)];
        let mut actual_update_size = 0;
        loop {
            let (current_node_key, current_bounds) = *node_stack.last().unwrap();
            let current_node_key = current_node_key as usize;
            let target_child_octant = child_octant_for(&current_bounds, &position);
            let target_bounds = Cube {
                min_position: current_bounds.min_position
                    + offset_region(target_child_octant) * current_bounds.size / 2.,
                size: current_bounds.size / 2.,
            };

            let current_node = self.nodes.get(current_node_key);
            let target_child_key =
                self.node_children[current_node_key][target_child_octant as u32] as usize;
            if target_bounds.size > clear_size.max(DIM as u32) as f32
                || self.is_node_internal(current_node_key)
            // Complex internal nodes further reduce possible update size
            {
                // iteration needs to go deeper, as current Node size is still larger, than the requested clear size
                if self.nodes.key_is_valid(target_child_key) {
                    //Iteration can go deeper , as target child is valid
                    node_stack.push((
                        self.node_children[current_node_key][target_child_octant as u32],
                        target_bounds,
                    ));
                } else {
                    // no children are available for the target octant
                    if matches!(
                        current_node,
                        NodeContent::Leaf(_) | NodeContent::UniformLeaf(_)
                    ) {
                        // The current Node is a leaf, representing the area under current_bounds
                        // filled with the data stored in NodeContent::*Leaf(_)
                        let target_match = match current_node {
                            NodeContent::Nothing | NodeContent::Internal(_) => {
                                panic!("Non-leaf node expected to be leaf!")
                            }
                            NodeContent::UniformLeaf(brick) => match brick {
                                BrickData::Empty => true,
                                BrickData::Solid(voxel) => voxel.is_empty(),
                                BrickData::Parted(brick) => {
                                    let index_in_matrix = position - current_bounds.min_position;
                                    brick[index_in_matrix.x as usize][index_in_matrix.y as usize]
                                        [index_in_matrix.z as usize]
                                        .is_empty()
                                }
                            },
                            NodeContent::Leaf(bricks) => {
                                match &bricks[target_child_octant as usize] {
                                    BrickData::Empty => true,
                                    BrickData::Solid(voxel) => voxel.is_empty(),
                                    BrickData::Parted(brick) => {
                                        let index_in_matrix = position - target_bounds.min_position;
                                        brick[index_in_matrix.x as usize]
                                            [index_in_matrix.y as usize]
                                            [index_in_matrix.z as usize]
                                            .is_empty()
                                    }
                                }
                            }
                        };
                        if target_match || current_node.is_empty() {
                            // the data stored equals the given data, at the requested position
                            // so no need to continue iteration as data already matches
                            break;
                        }

                        // The contained data does not match the given data at the given position,
                        // but the current node is a leaf, so it needs to be divided into separate nodes
                        // with its children having the same data as the current node to keep integrity
                        // It needs to be separated because it has an extent above DIM
                        debug_assert!(
                            current_bounds.size > DIM as f32,
                            "Expected Leaf node to have an extent({:?}) above DIM({DIM})!",
                            current_bounds.size
                        );
                        self.subdivide_leaf_to_nodes(
                            current_node_key,
                            target_child_octant as usize,
                        );

                        node_stack.push((
                            self.node_children[current_node_key][target_child_octant as u32],
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
                    current_node_key,
                    &current_bounds,
                    &target_bounds,
                    target_child_octant as usize,
                    &(position.into()),
                    clear_size as usize,
                    None,
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
            let previous_occupied_bits = self.stored_occupied_bits(node_key as usize);
            let mut new_occupied_bits = previous_occupied_bits;
            *self.nodes.get_mut(node_key as usize) = if 0 != new_occupied_bits {
                NodeContent::Internal(new_occupied_bits)
            } else {
                NodeContent::Nothing
            };
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
                removed_node = None;
            };

            if let NodeContent::Nothing = self.nodes.get(node_key as usize) {
                debug_assert!(self.node_children[node_key as usize].is_empty());
                removed_node = Some((node_key, node_bounds));
            }

            if node_bounds.size as usize == actual_update_size {
                new_occupied_bits = 0;
            } else {
                // Calculate the new occupied bits of the node
                let start_in_matrix =
                    matrix_index_for(&node_bounds, &position.into(), (DIM * 2).max(4));
                let bitmap_update_size = ((node_bounds.size * actual_update_size as f32)
                    / (DIM as f32 * 2.).max(4.))
                .ceil() as usize;
                for x in start_in_matrix.x
                    ..(start_in_matrix.x + bitmap_update_size).min(BITMAP_DIMENSION)
                {
                    for y in start_in_matrix.y
                        ..(start_in_matrix.y + bitmap_update_size).min(BITMAP_DIMENSION)
                    {
                        for z in start_in_matrix.z
                            ..(start_in_matrix.z + bitmap_update_size).min(BITMAP_DIMENSION)
                        {
                            if self.should_bitmap_be_empty_at_index(
                                node_key as usize,
                                &V3c::new(x, y, z),
                            ) {
                                set_occupancy_in_bitmap_64bits(
                                    &V3c::new(x, y, z),
                                    1,
                                    (DIM * 2).max(4),
                                    false,
                                    &mut new_occupied_bits,
                                );
                            }
                        }
                    }
                }
            }
            debug_assert!(
                0 != new_occupied_bits
                    || matches!(self.nodes.get(node_key as usize), NodeContent::Nothing)
            );
            self.store_occupied_bits(node_key as usize, new_occupied_bits);

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

    /// Updates the given node recursively to collapse nodes with uniform children into a leaf
    /// Returns with true if the given node was simplified
    pub(crate) fn simplify(&mut self, node_key: usize) -> bool {
        if self.nodes.key_is_valid(node_key) {
            match self.nodes.get_mut(node_key) {
                NodeContent::Nothing => true,
                NodeContent::UniformLeaf(brick) => {
                    debug_assert!(matches!(
                        self.node_children[node_key].content,
                        NodeChildrenArray::OccupancyBitmap(_)
                    ));
                    match brick {
                        BrickData::Empty => true,
                        BrickData::Solid(voxel) => {
                            if voxel.is_empty() {
                                debug_assert_eq!(
                                0,
                                if let NodeChildrenArray::OccupancyBitmap(occupied_bits) =
                                    self.node_children[node_key].content
                                {
                                    occupied_bits
                                } else {
                                    0xD34D
                                },
                                "Solid empty voxel should have its occupied bits set to 0, instead of {:#10X}",
                                if let NodeChildrenArray::OccupancyBitmap(occupied_bits) =
                                    self.node_children[node_key].content
                                {
                                    occupied_bits
                                } else {
                                    0xD34D
                                }
                            );
                                *self.nodes.get_mut(node_key) = NodeContent::Nothing;
                                self.node_children[node_key].content =
                                    NodeChildrenArray::NoChildren;
                                true
                            } else {
                                debug_assert_eq!(
                                u64::MAX,
                                if let NodeChildrenArray::OccupancyBitmap(occupied_bits) =
                                    self.node_children[node_key].content
                                {
                                    occupied_bits
                                } else {
                                    0xD34D
                                },
                                "Solid full voxel should have its occupied bits set to u64::MAX, instead of {:#10X}",
                                if let NodeChildrenArray::OccupancyBitmap(occupied_bits) =
                                    self.node_children[node_key].content
                                {
                                    occupied_bits
                                } else {
                                    0xD34D
                                }
                            );
                                false
                            }
                        }
                        BrickData::Parted(_brick) => {
                            if brick.simplify() {
                                debug_assert!(
                                    self.node_children[node_key].content
                                        == NodeChildrenArray::OccupancyBitmap(u64::MAX)
                                        || self.node_children[node_key].content
                                            == NodeChildrenArray::OccupancyBitmap(0)
                                );
                                true
                            } else {
                                false
                            }
                        }
                    }
                }
                NodeContent::Leaf(bricks) => {
                    debug_assert!(
                        matches!(
                            self.node_children[node_key].content,
                            NodeChildrenArray::OccupancyBitmap(_),
                        ),
                        "Expected node child to be OccupancyBitmap(_) instead of {:?}",
                        self.node_children[node_key].content
                    );
                    bricks[0].simplify();
                    for octant in 1..8 {
                        bricks[octant].simplify();
                        if bricks[0] != bricks[octant] {
                            return false;
                        }
                    }

                    // Every matrix is the same! Make leaf uniform
                    *self.nodes.get_mut(node_key) = NodeContent::UniformLeaf(bricks[0].clone());
                    self.node_children[node_key].content = NodeChildrenArray::OccupancyBitmap(
                        if let NodeChildrenArray::OccupancyBitmap(bitmaps) =
                            self.node_children[node_key].content
                        {
                            bitmaps
                        } else {
                            panic!("Leaf NodeContent should have OccupancyBitmap child assigned to it!");
                        },
                    );
                    self.simplify(node_key); // Try to collapse it to homogeneous node, but
                                             // irrespective of the results of it, return value is true,
                                             // because the node was updated already
                    true
                }
                NodeContent::Internal(_) => {
                    debug_assert!(matches!(
                        self.node_children[node_key].content,
                        NodeChildrenArray::Children(_),
                    ));
                    let child_keys = if let NodeChildrenArray::Children(children) =
                        self.node_children[node_key].content
                    {
                        children
                    } else {
                        return false;
                    };

                    // Try to simplify each child of the node
                    self.simplify(child_keys[0] as usize);

                    if !self.nodes.key_is_valid(child_keys[0] as usize) {
                        // At least try to simplify the siblings
                        for child_key in child_keys.iter().skip(1) {
                            self.simplify(*child_key as usize);
                        }
                        return false;
                    }

                    for octant in 1..8 {
                        self.simplify(child_keys[octant] as usize);
                        if !self.nodes.key_is_valid(child_keys[octant] as usize)
                            || (self.nodes.get(child_keys[0] as usize)
                                != self.nodes.get(child_keys[octant] as usize))
                        {
                            return false;
                        }
                    }

                    // All children are the same!
                    // make the current node a leaf, erase the children
                    debug_assert!(matches!(
                        self.nodes.get(child_keys[0] as usize),
                        NodeContent::Leaf(_) | NodeContent::UniformLeaf(_)
                    ));
                    self.nodes.swap(node_key, child_keys[0] as usize);
                    // Deallocate children, and set correct occupancy bitmap
                    let new_node_children = self.node_children[child_keys[0] as usize];
                    self.deallocate_children_of(node_key as u32);
                    self.node_children[node_key] = new_node_children;

                    // At this point there's no need to call simplify on the new leaf node
                    // because it's been attempted already on the data it copied from
                    true
                }
            }
        } else {
            // can't simplify node based on invalid key
            false
        }
    }
}
