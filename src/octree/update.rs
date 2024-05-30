use crate::object_pool::key_none_value;
use crate::octree::{
    detail::{bound_contains, child_octant_for},
    hash_region,
    types::{NodeChildren, NodeContent, OctreeError},
    Octree, VoxelData,
};
use crate::spatial::{
    math::{octant_bitmask, offset_region, vector::V3c},
    Cube,
};

impl<T: Default + PartialEq + Clone + VoxelData, const DIM: usize> Octree<T, DIM> {
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
        let root_bounds = Cube::root_bounds(self.octree_size);
        if !bound_contains(&root_bounds, position) {
            return Err(OctreeError::InvalidPosition {
                x: position.x,
                y: position.y,
                z: position.z,
            });
        }

        // A vector does not consume significant resources in this case, e.g. a 4096*4096*4096 chunk has depth of 12
        let mut node_stack = vec![(Octree::<T, DIM>::ROOT_NODE_KEY, root_bounds)];
        loop {
            let (current_node_key, current_bounds) = *node_stack.last().unwrap();
            let current_node_key = current_node_key as usize;
            let current_node = self.nodes.get(current_node_key);
            let target_child_octant = child_octant_for(&current_bounds, position);
            let target_child_occupies = octant_bitmask(target_child_octant);

            if current_bounds.size > insert_size.max(DIM as u32) {
                // iteration needs to go deeper, as current Node size is still larger, than the requested
                if self.nodes.key_is_valid(
                    self.node_children[current_node_key][target_child_octant as u32] as usize,
                ) {
                    node_stack.push((
                        self.node_children[current_node_key][target_child_octant as u32],
                        Cube {
                            min_position: current_bounds.min_position
                                + offset_region(target_child_octant) * current_bounds.size / 2,
                            size: current_bounds.size / 2,
                        },
                    ));
                } else {
                    let is_full_match = current_node.is_all(&data);
                    // no children are available for the target octant
                    if current_node.is_leaf() && is_full_match {
                        // The current Node is a leaf, but the data stored equals the data to be set, so no need to go deeper as tha data already matches
                        break;
                    }
                    if current_node.is_leaf() && !is_full_match {
                        // The current Node is a leaf, which essentially represents an area where all the contained space have the same data.
                        // The contained data does not match the given data to set the position to, so all of the Nodes' children need to be created
                        // as separate Nodes with the same data as their parent to keep integrity
                        let new_children =
                            self.make_uniform_children(current_node.leaf_data().clone());

                        // Set node type as internal; Since this node in this function will only have
                        // at most 1 child node( the currently inserted node ), so the occupancy bitmask
                        // is known at this point: as the node was a leaf, it's fully occupied
                        *self.nodes.get_mut(current_node_key) = NodeContent::Internal(0xFF);

                        self.node_children[current_node_key].set(new_children);
                        node_stack.push((
                            self.node_children[current_node_key][target_child_octant as u32],
                            Cube {
                                min_position: current_bounds.min_position
                                    + offset_region(target_child_octant) * current_bounds.size / 2,
                                size: current_bounds.size / 2,
                            },
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
                                // occupancy bitmask needs to employ that
                                *self.nodes.get_mut(current_node_key) =
                                    NodeContent::Internal(occupied_bits | target_child_occupies);
                            }
                            _ => {}
                        }

                        // The occupancy bitmask of the newly inserted child will be updated in the next
                        // loop of the depth iteration
                        let child_key = self.nodes.push(NodeContent::Internal(0)) as u32;
                        self.node_children
                            .resize(self.nodes.len(), NodeChildren::new(key_none_value()));

                        node_stack.push((
                            child_key,
                            Cube {
                                min_position: current_bounds.min_position
                                    + offset_region(target_child_octant) * current_bounds.size / 2,
                                size: current_bounds.size / 2,
                            },
                        ));
                        self.node_children[current_node_key][target_child_octant as u32] =
                            node_stack.last().unwrap().0;
                    }
                }
            } else {
                // current_bounds.size == min_node_size, which is the desired depth
                let mut mat_index = Self::mat_index(&current_bounds, position);
                let mut matrix_update_fn = |d: &mut [[[T; DIM]; DIM]; DIM]| {
                    // In case insert_size does not equal DIM, the matrix needs to be updated
                    if insert_size == 1 {
                        d[mat_index.x][mat_index.y][mat_index.z] = data.clone();
                    } else if insert_size < DIM as u32 {
                        // update size is smaller, than the matrix, but > 1
                        // simulate the Nodes layout and update accordingly
                        mat_index.cut_each_component(&(DIM - insert_size as usize));
                        for x in d.iter_mut().skip(mat_index.x).take(insert_size as usize) {
                            for y in x.iter_mut().skip(mat_index.x).take(insert_size as usize) {
                                for item in
                                    y.iter_mut().skip(mat_index.x).take(insert_size as usize)
                                {
                                    *item = data.clone();
                                }
                            }
                        }
                    }
                };
                match self.nodes.get_mut(current_node_key) {
                    NodeContent::Leaf(d) => {
                        matrix_update_fn(d);
                    }
                    // should the current Node be anything other, than a leaf at this point, it is to be converted into one
                    _ => {
                        if insert_size == DIM as u32 || insert_size >= current_bounds.size {
                            // update size equals matrix size, update the whole matrix
                            *self.nodes.get_mut(current_node_key) = NodeContent::leaf_from(data);
                            self.deallocate_children_of(node_stack.last().unwrap().0);
                            break;
                        } else {
                            *self.nodes.get_mut(current_node_key) =
                                NodeContent::leaf_from(T::default());
                            matrix_update_fn(self.nodes.get_mut(current_node_key).mut_leaf_data());
                        }
                    }
                }
                break;
            }
        }

        // post-processing operations
        let mut simplifyable = self.auto_simplify; // Don't even start to simplify if it's disabled
        for (node_key, _node_bounds) in node_stack.into_iter().rev() {
            let current_node = self.nodes.get(node_key as usize);
            if !self.nodes.key_is_valid(node_key as usize)
                || matches!(current_node, NodeContent::Leaf(_))
            {
                continue;
            }
            let previous_occupied_bits = self.occupied_bits(node_key);
            let occupied_bits = self.occupied_bits(node_key);
            if let NodeContent::Nothing = current_node {
                // This is incorrect information which needs to be corrected
                // As the current Node is either a parent or a data leaf, it can not be Empty or nothing
                *self.nodes.get_mut(node_key as usize) =
                    NodeContent::Internal(self.occupied_bits(node_key));
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
        let root_bounds = Cube::root_bounds(self.octree_size);
        if !bound_contains(&root_bounds, position) {
            return Err(OctreeError::InvalidPosition {
                x: position.x,
                y: position.y,
                z: position.z,
            });
        }

        // A vector does not consume significant resources in this case, e.g. a 4096*4096*4096 chunk has depth of 12
        let mut node_stack = vec![(Octree::<T, DIM>::ROOT_NODE_KEY, root_bounds)];
        let mut parent_target = child_octant_for(&root_bounds, position); //This init value is never used

        loop {
            let (current_node_key, current_bounds) = *node_stack.last().unwrap();
            let current_node_key = current_node_key as usize;
            let current_node = self.nodes.get(current_node_key);
            let target_child_octant;

            if current_bounds.size > clear_size.max(DIM as u32) {
                // iteration needs to go deeper, as current Node size is still larger, than the requested clear size
                target_child_octant = child_octant_for(&current_bounds, position);
                if self.nodes.key_is_valid(
                    self.node_children[current_node_key][target_child_octant as u32] as usize,
                ) {
                    //Iteration can go deeper , as target child is valid
                    node_stack.push((
                        self.node_children[current_node_key][target_child_octant as u32],
                        Cube {
                            min_position: current_bounds.min_position
                                + offset_region(target_child_octant) * current_bounds.size / 2,
                            size: current_bounds.size / 2,
                        },
                    ));
                } else {
                    // no children are available for the target octant
                    if current_node.is_leaf() {
                        // The current Node is a leaf, which essentially represents an area where all the contained space have the same data.
                        // The contained data does not match the given data to set the position to, so all of the Nodes' children need to be created
                        // as separate Nodes with the same data as their parent to keep integrity, the cleared node will update occupancy bitmask correctly
                        let current_data = current_node.leaf_data().clone();
                        let new_children = self.make_uniform_children(current_data);
                        *self.nodes.get_mut(current_node_key) = NodeContent::Internal(0xFF);
                        self.node_children[current_node_key].set(new_children);
                        node_stack.push((
                            self.node_children[current_node_key][target_child_octant as u32],
                            Cube {
                                min_position: current_bounds.min_position
                                    + offset_region(target_child_octant) * current_bounds.size / 2,
                                size: current_bounds.size / 2,
                            },
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
                let mut mat_index = Self::mat_index(&current_bounds, position);
                if clear_size == 1 {
                    self.nodes
                        .get_mut(current_node_key)
                        .as_mut_leaf_ref()
                        .unwrap()[mat_index.x][mat_index.y][mat_index.z]
                        .clear();
                } else if clear_size < DIM as u32 {
                    // update size is smaller, than the matrix, but > 1
                    mat_index.cut_each_component(&(DIM - clear_size as usize));
                    for x in mat_index.x..(mat_index.x + clear_size as usize) {
                        for y in mat_index.y..(mat_index.y + clear_size as usize) {
                            for z in mat_index.z..(mat_index.z + clear_size as usize) {
                                self.nodes
                                    .get_mut(current_node_key)
                                    .as_mut_leaf_ref()
                                    .unwrap()[x][y][z]
                                    .clear();
                            }
                        }
                    }
                } else {
                    // The size to clear >= DIM, the whole node is to be erased
                    // unset the current node and its children
                    self.deallocate_children_of(current_node_key as u32);

                    // Set the parents child to None
                    if node_stack.len() >= 2 {
                        self.nodes.free(current_node_key);
                        let parent_key = node_stack[node_stack.len() - 2].0 as usize;
                        self.node_children[parent_key][parent_target as u32] = key_none_value();
                    } else {
                        // If the node doesn't have parents, then it's a root node and should not be deleted
                        *self.nodes.get_mut(current_node_key) = NodeContent::Nothing;
                    }
                }
                break;
            }
            parent_target = target_child_octant;
        }

        // post-processing operations
        let mut empty_child: Option<(Cube, usize)> = None;
        if let Some((node_key, node_bounds)) = node_stack.pop() {
            if self.nodes.key_is_valid(node_key as usize)
                && self.nodes.get(node_key as usize).is_empty()
            {
                // If the updated node is empty, mark it as such
                *self.nodes.get_mut(node_key as usize) = NodeContent::Nothing;
                empty_child = Some((node_bounds, node_key as usize));
            }
        }
        let mut simplifyable = self.auto_simplify; // Don't even start to simplify if it's disabled
        for (node_key, node_bounds) in node_stack.into_iter().rev() {
            let previous_occupied_bits = self.occupied_bits(node_key);
            let occupied_bits = self.occupied_bits(node_key);
            *self.nodes.get_mut(node_key as usize) = if 0 != occupied_bits {
                NodeContent::Internal(occupied_bits)
            } else {
                NodeContent::Nothing
            };
            if let Some((child_bounds, child_key)) = empty_child {
                // If the child of this node was set to NodeContent::Nothing during this clear operation
                // it needs to be freed up, and the child index of this node needs to be updated as well
                let child_octant = hash_region(
                    &(V3c::from(child_bounds.min_position - node_bounds.min_position)
                        + V3c::unit(child_bounds.size as f32 / 2.)),
                    node_bounds.size as f32,
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
}
