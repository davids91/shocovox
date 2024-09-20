use crate::object_pool::empty_marker;
use crate::octree::types::NodeChildrenArray;
use crate::octree::{
    detail::{bound_contains, child_octant_for},
    types::{NodeChildren, NodeContent, OctreeError},
    Octree, VoxelData,
};
use crate::spatial::{
    math::{
        hash_region, octant_bitmask, offset_region, set_occupancy_in_bitmap_64bits, vector::V3c,
    },
    Cube,
};

impl<T, const DIM: usize> Octree<T, DIM>
where
    T: Default + PartialEq + Clone + Copy + PartialEq + VoxelData,
{
    /// Inserts the given data into the octree into the intended voxel position
    pub fn insert(&mut self, position: &V3c<u32>, data: T) -> Result<(), OctreeError> {
        self.insert_at_lod(position, 1, data)
    }

    /// Updates the given node to be a Leaf, and inserts the provided data for it
    fn leaf_update(
        &mut self,
        node_key: usize,
        node_bounds: &Cube,
        target_bounds: &Cube,
        target_child_octant: usize,
        position: &V3c<u32>,
        size: usize,
        data: Option<T>,
    ) {
        debug_assert!(target_bounds.size <= DIM as f32);
        // Update the leaf node, if it is possible as is, and if it's even needed to update
        // and decide if the node content needs to be divided into bricks, and the update function to be called again
        if match self.nodes.get_mut(node_key) {
            NodeContent::Leaf(mats) => {
                //If there is no brick in the target position of the leaf, create one
                if let None = mats[target_child_octant as usize] {
                    mats[target_child_octant as usize] =
                        Some(Box::new([[[T::default(); DIM]; DIM]; DIM]));
                }

                // Update the voxel inside the target brick at the target position
                let mat_index = Self::mat_index(&target_bounds, &V3c::from(*position));
                Self::update_brick(
                    mats[target_child_octant as usize].as_mut().unwrap(),
                    mat_index,
                    size,
                    &mut self.node_children[node_key].content,
                    data,
                );
                false
            }
            NodeContent::UniformLeaf(mat) => {
                // The target position index is to be calculated from the node bounds,
                // instead of the target bounds because the position should cover the whole leaf
                // not just one brick in it
                let mat_index = Self::mat_index(&node_bounds, &V3c::from(*position));

                // In case the data doesn't match the current contents of the node, it needs to be subdivided
                (data.is_none() && !mat[mat_index.x][mat_index.y][mat_index.z].is_empty())
                    || (data.is_some()
                        && data.unwrap() != mat[mat_index.x][mat_index.y][mat_index.z])
            }
            NodeContent::HomogeneousLeaf(d) => {
                // In case the data doesn't match the current contents of the node, it needs to be subdivided
                (data.is_none() && !d.is_empty()) || (data.is_some() && data.unwrap() != *d)
            }
            NodeContent::Nothing | NodeContent::Internal(_) => {
                if size == DIM || size as f32 >= node_bounds.size {
                    // update size equals node size, update the whole node
                    self.deallocate_children_of(node_key as u32);
                    if let Some(data) = data {
                        // New full leaf node
                        *self.nodes.get_mut(node_key) = NodeContent::HomogeneousLeaf(data);
                        self.node_children[node_key].content =
                            NodeChildrenArray::OccupancyBitmap(u64::MAX);
                    } else {
                        // New empty leaf node, it will be erased during the post-process operations
                        *self.nodes.get_mut(node_key) = NodeContent::Nothing;
                        self.node_children[node_key].content =
                            NodeChildrenArray::OccupancyBitmap(0);
                    }
                    false
                } else {
                    // Current node might be an internal node, but because the function
                    // should only be called when target bounds <= DIM
                    // That means no internal nodes should contain data at this point
                    *self.nodes.get_mut(node_key) =
                        NodeContent::Leaf([None, None, None, None, None, None, None, None]);
                    self.node_children[node_key].content =
                        NodeChildrenArray::OccupancyBitmaps([0; 8]);
                    true
                }
            }
        } {
            // the data at the position inside the brick doesn't match the given data,
            // so the leaf needs to be divided into a NodeContent::Leaf(mats)
            self.nodes
                .get_mut(node_key)
                .subdivide_leaf(&mut self.node_children[node_key].content);
            self.leaf_update(
                node_key,
                node_bounds,
                target_bounds,
                target_child_octant,
                position,
                size,
                data,
            );
        }
    }

    fn update_brick(
        d: &mut [[[T; DIM]; DIM]; DIM],
        mut mat_index: V3c<usize>,
        size: usize,
        node_children_array: &mut NodeChildrenArray<u32>,
        data: Option<T>,
    ) {
        debug_assert!(size <= DIM as usize);
        mat_index.cut_each_component(&(DIM - size as usize));
        if !matches!(node_children_array, NodeChildrenArray::OccupancyBitmap(_)) {
            *node_children_array = NodeChildrenArray::OccupancyBitmap(0);
        }
        for x in mat_index.x..(mat_index.x + size) {
            for y in mat_index.y..(mat_index.y + size) {
                for z in mat_index.z..(mat_index.z + size) {
                    if let Some(data) = data {
                        d[x][y][z] = data.clone();
                    } else {
                        d[x][y][z].clear();
                    }
                    if let NodeChildrenArray::OccupancyBitmap(bitmap) = node_children_array {
                        set_occupancy_in_bitmap_64bits(x, y, z, DIM, data.is_some(), bitmap);
                    }
                }
            }
        }
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

        // A CPU stack does not consume significant relevant resources, e.g. a 4096*4096*4096 chunk has depth of 12
        let mut node_stack = vec![(Octree::<T, DIM>::ROOT_NODE_KEY, root_bounds)];
        loop {
            let (current_node_key, current_bounds) = *node_stack.last().unwrap();
            let current_node_key = current_node_key as usize;
            let current_node = self.nodes.get(current_node_key);
            let target_child_octant = child_octant_for(&current_bounds, &position);
            let target_child_occupies = octant_bitmask(target_child_octant);
            let target_child_key = self.node_children[current_node_key][target_child_octant as u32];
            let target_bounds = Cube {
                min_position: current_bounds.min_position
                    + offset_region(target_child_octant) * current_bounds.size / 2.,
                size: current_bounds.size / 2.,
            };

            // iteration needs to go deeper, as current target size is still larger, than the requested
            if target_bounds.size > insert_size.max(DIM as u32) as f32 {
                // the child at the queried position exists and valid, recurse into it
                if self.nodes.key_is_valid(target_child_key as usize) {
                    node_stack.push((target_child_key, target_bounds));
                } else {
                    // no children are available for the target octant while
                    // current node size is still larger, than the requested size
                    if current_node.is_leaf() {
                        // The current Node is a leaf, representing the area under current_bounds
                        // filled with the data stored in NodeContent::*Leaf(_)
                        let target_match = match current_node {
                            NodeContent::Nothing | NodeContent::Internal(_) => {
                                panic!("Non-leaf node expected to be leaf!")
                            }
                            NodeContent::HomogeneousLeaf(d) => *d == data,
                            NodeContent::UniformLeaf(mat) => {
                                let index_in_matrix = position - current_bounds.min_position;
                                mat[index_in_matrix.x as usize][index_in_matrix.y as usize]
                                    [index_in_matrix.z as usize]
                                    == data
                            }
                            NodeContent::Leaf(mats) => {
                                let index_in_matrix = position - target_bounds.min_position;
                                mats[target_child_octant as usize].is_some()
                                    && (mats[target_child_octant as usize].as_ref().unwrap()
                                        [index_in_matrix.x as usize]
                                        [index_in_matrix.y as usize]
                                        [index_in_matrix.z as usize]
                                        == data)
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
                        self.nodes
                            .get_mut(current_node_key)
                            .subdivide_leaf(&mut self.node_children[current_node_key].content);

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
                                *self.nodes.get_mut(current_node_key) =
                                    NodeContent::Internal(target_child_occupies);
                            }
                            NodeContent::Internal(occupied_bits) => {
                                // the node has pre-existing children and a new child node is inserted
                                // occupancy bitmap needs to contain this information
                                *self.nodes.get_mut(current_node_key) =
                                    NodeContent::Internal(occupied_bits | target_child_occupies);
                            }
                            NodeContent::Leaf(_)
                            | NodeContent::UniformLeaf(_)
                            | NodeContent::HomogeneousLeaf(_) => {
                                panic!("Leaf Node expected to be non-leaf!");
                            }
                        }

                        // Update node_children to reflect the inserted node
                        self.node_children
                            .resize(self.nodes.len(), NodeChildren::new(empty_marker()));
                        self.node_children[current_node_key][target_child_octant as u32] =
                            node_stack.last().unwrap().0;

                        // The occupancy bitmap of the node will be updated
                        // in the next iteration or in the post-processing logic
                        let child_key = self.nodes.push(NodeContent::Internal(0)) as u32;

                        node_stack.push((child_key, target_bounds));
                    }
                }
            } else {
                // target_bounds.size <= min_node_size, which is the desired depth!
                self.leaf_update(
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
        for (node_key, _node_bounds) in node_stack.into_iter().rev() {
            let current_node = self.nodes.get(node_key as usize);
            if !self.nodes.key_is_valid(node_key as usize)
                || matches!(
                    current_node,
                    NodeContent::Leaf(_)
                        | NodeContent::UniformLeaf(_)
                        | NodeContent::HomogeneousLeaf(_)
                )
            {
                continue;
            }
            let previous_occupied_bits = self.occupied_8bit(node_key);
            let occupied_bits = self.occupied_8bit(node_key);
            if let NodeContent::Nothing = current_node {
                // This is incorrect information which needs to be corrected
                // As the current Node is either a parent or a data leaf, it can not be Empty or nothing
                *self.nodes.get_mut(node_key as usize) =
                    NodeContent::Internal(self.occupied_8bit(node_key));
            }
            if simplifyable {
                simplifyable = self.simplify(node_key); // If any Nodes fail to simplify, no need to continue because their parents can not be simplified because of it
            }
            if !simplifyable && occupied_bits == previous_occupied_bits {
                break;
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
        loop {
            let (current_node_key, current_bounds) = *node_stack.last().unwrap();
            let current_node_key = current_node_key as usize;
            let current_node = self.nodes.get(current_node_key);
            let target_child_octant = child_octant_for(&current_bounds, &position);
            let target_bounds = Cube {
                min_position: current_bounds.min_position
                    + offset_region(target_child_octant) * current_bounds.size / 2.,
                size: current_bounds.size / 2.,
            };

            if current_bounds.size > clear_size.max(DIM as u32) as f32 {
                // iteration needs to go deeper, as current Node size is still larger, than the requested clear size
                if self.nodes.key_is_valid(
                    self.node_children[current_node_key][target_child_octant as u32] as usize,
                ) {
                    //Iteration can go deeper , as target child is valid
                    node_stack.push((
                        self.node_children[current_node_key][target_child_octant as u32],
                        target_bounds,
                    ));
                } else {
                    // no children are available for the target octant
                    if current_node.is_leaf() {
                        // The current Node is a leaf, representing the area under current_bounds
                        // filled with the data stored in NodeContent::*Leaf(_)
                        let target_match = match current_node {
                            NodeContent::Nothing | NodeContent::Internal(_) => {
                                panic!("Non-leaf node expected to be leaf!")
                            }
                            NodeContent::HomogeneousLeaf(d) => {
                                debug_assert!(
                                    !d.is_empty(),
                                    "HomogeneousLeaf should not be empty!"
                                );
                                d.is_empty()
                            }
                            NodeContent::UniformLeaf(mat) => {
                                let index_in_matrix = position - current_bounds.min_position;
                                mat[index_in_matrix.x as usize][index_in_matrix.y as usize]
                                    [index_in_matrix.z as usize]
                                    .is_empty()
                            }
                            NodeContent::Leaf(mats) => {
                                let index_in_matrix = position - target_bounds.min_position;
                                mats[target_child_octant as usize].is_some()
                                    && (mats[target_child_octant as usize].as_ref().unwrap()
                                        [index_in_matrix.x as usize]
                                        [index_in_matrix.y as usize]
                                        [index_in_matrix.z as usize]
                                        .is_empty())
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
                        self.nodes
                            .get_mut(current_node_key)
                            .subdivide_leaf(&mut self.node_children[current_node_key].content);

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
                self.leaf_update(
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
        let mut empty_child: Option<(Cube, usize)> = None;
        if let Some((node_key, node_bounds)) = node_stack.pop() {
            debug_assert!(
                !self.nodes.key_is_valid(node_key as usize)
                    || (self.nodes.get(node_key as usize).is_empty()
                        == self.node_children[node_key as usize].is_empty())
            );
            if self.nodes.key_is_valid(node_key as usize)
                && self.node_children[node_key as usize].is_empty()
            {
                // If the updated node is empty, mark it as such
                *self.nodes.get_mut(node_key as usize) = NodeContent::Nothing;
                empty_child = Some((node_bounds, node_key as usize));
            }
        }
        let mut simplifyable = self.auto_simplify; // Don't even start to simplify if it's disabled
        for (node_key, node_bounds) in node_stack.into_iter().rev() {
            let previous_occupied_bits = self.occupied_8bit(node_key);
            let occupied_bits = self.occupied_8bit(node_key);
            *self.nodes.get_mut(node_key as usize) = if 0 != occupied_bits {
                NodeContent::Internal(occupied_bits)
            } else {
                NodeContent::Nothing
            };
            if let Some((child_bounds, child_key)) = empty_child {
                // If the child of this node was set to NodeContent::Nothing during this clear operation
                // it needs to be freed up, and the child index of this node needs to be updated as well
                let child_octant = hash_region(
                    &((child_bounds.min_position - node_bounds.min_position)
                        + V3c::unit(child_bounds.size / 2.)),
                    node_bounds.size / 2.,
                ) as usize;
                self.node_children[node_key as usize].clear(child_octant);
                self.nodes.free(child_key);
            };
            if let NodeContent::Nothing = self.nodes.get(node_key as usize) {
                debug_assert!(self.node_children[node_key as usize].is_empty());
                empty_child = Some((node_bounds, node_key as usize));
            }

            if simplifyable {
                simplifyable = self.simplify(node_key); // If any Nodes fail to simplify, no need to continue because their parents can not be simplified because of it
            }
            if !simplifyable && previous_occupied_bits == occupied_bits {
                break;
            }
        }
        Ok(())
    }

    /// Updates the given node recursively to collapse nodes with uniform children into a leaf
    /// Returns with true if the given node was modified in any way, except when the node content is
    /// NodeContent::Nothing, because that should be eliminated in any way possible
    pub(crate) fn simplify(&mut self, node_key: u32) -> bool {
        if self.nodes.key_is_valid(node_key as usize) {
            match self.nodes.get(node_key as usize) {
                NodeContent::Nothing => true,
                NodeContent::HomogeneousLeaf(d) => {
                    if d.is_empty() {
                        *self.nodes.get_mut(node_key as usize) = NodeContent::Nothing;
                        true
                    } else {
                        false
                    }
                }
                NodeContent::UniformLeaf(data) => {
                    if self.nodes.get(node_key as usize).is_all(&data[0][0][0]) {
                        if data[0][0][0].is_empty() {
                            *self.nodes.get_mut(node_key as usize) = NodeContent::Nothing;
                            self.node_children[node_key as usize].content =
                                NodeChildrenArray::NoChildren;
                        } else {
                            *self.nodes.get_mut(node_key as usize) =
                                NodeContent::HomogeneousLeaf(data[0][0][0]);
                            self.node_children[node_key as usize].content =
                                NodeChildrenArray::OccupancyBitmap(u64::MAX);
                        }
                        true
                    } else {
                        false
                    }
                }
                NodeContent::Leaf(mats) => {
                    debug_assert!(matches!(
                        self.node_children[node_key as usize].content,
                        NodeChildrenArray::OccupancyBitmaps(_)
                    ));
                    let leaf_data = mats[0].clone();
                    for octant in 1..8 {
                        if mats[octant].is_none()
                            || leaf_data.is_none()
                            || leaf_data.as_ref().unwrap() != mats[octant].as_ref().unwrap()
                        {
                            return false;
                        }
                    }

                    // Every matrix is the same! Make leaf uniform
                    *self.nodes.get_mut(node_key as usize) =
                        NodeContent::UniformLeaf(leaf_data.unwrap());
                    self.node_children[node_key as usize].content =
                        NodeChildrenArray::OccupancyBitmap(
                            if let NodeChildrenArray::OccupancyBitmaps(bitmaps) =
                                self.node_children[node_key as usize].content
                            {
                                bitmaps[0]
                            } else {
                                panic!("Leaf NodeContent should have OccupancyBitmaps assigned to them in self.node_children! ");
                            },
                        );
                    self.simplify(node_key); // Try to collapse it to homogeneous node, but
                                             // irrespective of the results, fn result is true,
                                             // because the node was updated already
                    true
                }
                NodeContent::Internal(_) => {
                    debug_assert!(matches!(
                        self.node_children[node_key as usize].content,
                        NodeChildrenArray::Children(_),
                    ));
                    let child_keys = if let NodeChildrenArray::Children(children) =
                        self.node_children[node_key as usize].content
                    {
                        children
                    } else {
                        return false;
                    };

                    // Try to simplify each child of the node
                    self.simplify(child_keys[0]);
                    for octant in 1..8 {
                        self.simplify(child_keys[octant]);
                        if self.nodes.get(child_keys[0] as usize)
                            != self.nodes.get(child_keys[octant] as usize)
                        {
                            return false;
                        }
                    }

                    // All children are the same!
                    // make the current node a leaf, erase the children
                    debug_assert!(matches!(
                        self.nodes.get(child_keys[0] as usize),
                        NodeContent::Leaf(_)
                            | NodeContent::UniformLeaf(_)
                            | NodeContent::HomogeneousLeaf(_)
                    ));
                    self.nodes.swap(node_key as usize, child_keys[0] as usize);

                    // Update nodechildren to have the corresponding occupancy_bitmap
                    self.node_children[node_key as usize] =
                        self.node_children[child_keys[0] as usize];

                    // Deallocate the children, because we don't need them anymore
                    self.deallocate_children_of(node_key);

                    // At this point there's no need to call simplify on the new leaf node
                    // because it's been attempted already on the data it copied from
                    true
                }
            }
        } else {
            self.nodes.try_defragment(node_key as usize);
            false
        }
    }
}
