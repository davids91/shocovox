pub mod bytecode;
pub mod detail;
pub mod tests;
pub mod types;

#[cfg(feature = "raytracing")]
pub mod raytracing;

pub use types::{Octree, VoxelData};

use crate::object_pool::key_none_value;
use crate::object_pool::ObjectPool;
use crate::octree::detail::{bound_contains, child_octant_for};
use crate::octree::types::{NodeChildren, NodeContent, OctreeError};
use crate::spatial::math::{hash_region, offset_region, V3c};
use crate::spatial::Cube;
use bendy::{decoding::FromBencode, encoding::ToBencode};

impl<T: Default + PartialEq + Clone + VoxelData, const DIM: usize> Octree<T, DIM> {
    /// converts the data structure to a byte representation
    pub fn to_bytes(&self) -> Vec<u8> {
        self.to_bencode().ok().unwrap()
    }

    /// parses the data structure from a byte string
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self::from_bencode(&bytes).ok().unwrap()
    }

    /// saves the data structure to the given file path
    pub fn save(&mut self, path: &str) -> Result<(), std::io::Error> {
        use std::fs::File;
        use std::io::Write;
        let mut file = File::create(path)?;
        file.write_all(&self.to_bytes())?;
        Ok(())
    }

    /// loads the data structure from the given file path
    pub fn load(path: &str) -> Result<Self, std::io::Error> {
        use std::fs::File;
        use std::io::Read;
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(Self::from_bytes(bytes))
    }

    /// creates an octree with overall size nodes_dimension * DIM
    /// * `size` - must be `DIM * (2^x)`, e.g: DIM == 3 --> size can be 3,6,12,24,48 ...
    pub fn new(size: u32) -> Result<Self, OctreeError> {
        if Self::is_size_inadequate(size) {
            return Err(OctreeError::InvalidNodeSize(size));
        }
        let mut nodes = ObjectPool::<NodeContent<T, DIM>>::with_capacity(size.pow(3) as usize);
        let mut node_children = Vec::with_capacity(size.pow(3) as usize);
        node_children.push(NodeChildren::new(key_none_value()));
        let root_node_key = nodes.push(NodeContent::Nothing); // The first element is the root Node
        assert!(root_node_key == 0);
        Ok(Self {
            auto_simplify: true,
            octree_size: size,
            nodes,
            node_children,
        })
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
        let mut added_nodes_count = 0;
        loop {
            let (current_node_key, current_bounds) = *node_stack.last().unwrap();
            let current_node_key = current_node_key as usize;
            let target_child_octant = child_octant_for(&current_bounds, position);

            if current_bounds.size > insert_size.max(DIM as u32) {
                // iteration needs to go deeper, as current Node size is still larger, than the requested
                if crate::object_pool::key_might_be_valid(
                    self.node_children[current_node_key][target_child_octant],
                ) {
                    node_stack.push((
                        self.node_children[current_node_key][target_child_octant],
                        Cube {
                            min_position: current_bounds.min_position
                                + offset_region(target_child_octant) * current_bounds.size / 2,
                            size: current_bounds.size / 2,
                        },
                    ));
                } else {
                    let is_full_match = self.nodes.get(current_node_key).is_all(&data);
                    // no children are available for the target octant
                    if self.nodes.get(current_node_key).is_leaf() && is_full_match {
                        // The current Node is a leaf, but the data stored equals the data to be set, so no need to go deeper as tha data already matches
                        break;
                    }
                    if self.nodes.get(current_node_key).is_leaf() && !is_full_match {
                        // The current Node is a leaf, which essentially represents an area where all the contained space have the same data.
                        // The contained data does not match the given data to set the position to, so all of the Nodes' children need to be created
                        // as separate Nodes with the same data as their parent to keep integrity
                        let content = self.nodes.get(current_node_key).clone();
                        let new_children = self.make_uniform_children(content.leaf_data().clone());

                        // Set node type as internal, after the insertion the count will be updated for the whole structure
                        // Since this node in this function will only have at most 1 child node( the currently inserted node),
                        // Internal count will be updated correctly, as the other children of this node will contain no voxels
                        *self.nodes.get_mut(current_node_key) = NodeContent::Internal(0);
                        self.node_children[current_node_key].set(new_children);
                        node_stack.push((
                            self.node_children[current_node_key][target_child_octant],
                            Cube {
                                min_position: current_bounds.min_position
                                    + offset_region(target_child_octant) * current_bounds.size / 2,
                                size: current_bounds.size / 2,
                            },
                        ));
                    } else {
                        // current Node is a non-leaf Node, which doesn't have the child at the requested position, so it is inserted
                        // The Node becomes non-empty, but its count remains 0; After the insertion the count will be updated for the whole structure
                        match *self.nodes.get(current_node_key) {
                            NodeContent::Nothing => {
                                // A special case during the first insertion, where the root Node was empty beforehand
                                *self.nodes.get_mut(current_node_key) = NodeContent::Internal(0);
                            }
                            _ => {}
                        };
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
                        self.node_children[current_node_key][target_child_octant] =
                            node_stack.last().unwrap().0;
                    }
                }
            } else {
                let mut mat_index = Self::mat_index(&current_bounds, position);
                let mut matrix_update_fn = |d: &mut [[[T; DIM]; DIM]; DIM]| {
                    // In case insert_size does not equal DIM, the matrix needs to be updated
                    if insert_size == 1 {
                        d[mat_index.x][mat_index.y][mat_index.z] = data.clone();
                    } else if insert_size < DIM as u32 {
                        // update size is smaller, than the matrix, but > 1
                        // simulate the Nodes layout and update accordingly
                        mat_index.cut_each_component(&(DIM - insert_size as usize));
                        for x in mat_index.x..(mat_index.x + insert_size as usize) {
                            for y in mat_index.y..(mat_index.y + insert_size as usize) {
                                for z in mat_index.z..(mat_index.z + insert_size as usize) {
                                    d[x][y][z] = data.clone();
                                }
                            }
                        }
                    }
                };
                match self.nodes.get_mut(current_node_key) {
                    NodeContent::Leaf(d) => {
                        added_nodes_count = insert_size.pow(3);
                        matrix_update_fn(d);
                    }
                    // should the current Node be anything other, than a leaf at this point, it is to be converted into one
                    _ => {
                        added_nodes_count = current_bounds.size.pow(3);
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
        for (node_key, node_bounds) in node_stack.into_iter().rev() {
            if simplifyable {
                simplifyable = self.simplify(node_key); // If any Nodes fail to simplify, no need to continue because their parents can not be simplified because of it
            }
            match self.nodes.get(node_key as usize) {
                NodeContent::Nothing => {
                    // This is incorrect information which needs to be corrected
                    // As the current Node is either a parent or a data leaf, it can not be Empty or nothing
                    // To correct this, the children will determine the count
                    *self.nodes.get_mut(node_key as usize) =
                        NodeContent::Internal(self.count_cached_children(node_key));
                }
                // Update the number of voxels the node contains; It can hold at maximum size*size*size voxels
                NodeContent::Internal(contains_count) => {
                    *self.nodes.get_mut(node_key as usize) = NodeContent::Internal(
                        (contains_count + added_nodes_count).min(node_bounds.size.pow(3)),
                    );
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Provides immutable reference to the data, if there is any at the given position
    pub fn get(&self, position: &V3c<u32>) -> Option<&T> {
        let mut current_bounds = Cube::root_bounds(self.octree_size);
        let mut current_node_key = Octree::<T, DIM>::ROOT_NODE_KEY as usize;
        if !bound_contains(&current_bounds, position) {
            return None;
        }

        loop {
            match self.nodes.get(current_node_key) {
                NodeContent::Nothing => {
                    return None;
                }
                NodeContent::Leaf(mat) => {
                    let mat_index = Self::mat_index(&current_bounds, position);
                    if !mat[mat_index.x][mat_index.y][mat_index.z].is_empty() {
                        return Some(&mat[mat_index.x][mat_index.y][mat_index.z]);
                    }
                    return None;
                }
                _ => {
                    let child_octant_at_position = child_octant_for(&current_bounds, position);
                    let child_at_position =
                        self.node_children[current_node_key][child_octant_at_position];
                    if crate::object_pool::key_might_be_valid(child_at_position) {
                        current_node_key = child_at_position as usize;
                        current_bounds =
                            Cube::child_bounds_for(&current_bounds, child_octant_at_position);
                    } else {
                        return None;
                    }
                }
            }
        }
    }

    /// Provides mutable reference to the data, if there is any at the given position
    pub fn get_mut(&mut self, position: &V3c<u32>) -> Option<&mut T> {
        let mut current_bounds = Cube::root_bounds(self.octree_size);
        let mut current_node_key = Octree::<T, DIM>::ROOT_NODE_KEY as usize;
        if !bound_contains(&current_bounds, position) {
            return None;
        }

        loop {
            match self.nodes.get(current_node_key) {
                NodeContent::Nothing => {
                    return None;
                }
                NodeContent::Leaf(mat) => {
                    let mat_index = Self::mat_index(&current_bounds, position);
                    if !mat[mat_index.x][mat_index.y][mat_index.z].is_empty() {
                        return Some(
                            &mut self
                                .nodes
                                .get_mut(current_node_key)
                                .as_mut_leaf_ref()
                                .unwrap()[mat_index.x][mat_index.y][mat_index.z],
                        );
                    }
                    return None;
                }
                _ => {
                    let child_octant_at_position = child_octant_for(&current_bounds, position);
                    let child_at_position =
                        self.node_children[current_node_key][child_octant_at_position];
                    if crate::object_pool::key_might_be_valid(child_at_position) {
                        current_node_key = child_at_position as usize;
                        current_bounds =
                            Cube::child_bounds_for(&current_bounds, child_octant_at_position);
                    } else {
                        return None;
                    }
                }
            }
        }
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
        let mut target_child_octant = 9; //This init value should not be used. In case there is only one node, there is parent of it;
        let mut removed_nodes_count = 0;
        loop {
            let (current_node_key, current_bounds) = *node_stack.last().unwrap();
            let current_node_key = current_node_key as usize;
            if current_bounds.size > clear_size.max(DIM as u32) {
                // iteration needs to go deeper, as current Node size is still larger, than the requested clear size
                target_child_octant = child_octant_for(&current_bounds, position);
                if crate::object_pool::key_might_be_valid(
                    self.node_children[current_node_key][target_child_octant],
                ) {
                    //Iteration can go deeper , as target child is valid
                    node_stack.push((
                        self.node_children[current_node_key][target_child_octant],
                        Cube {
                            min_position: current_bounds.min_position
                                + offset_region(target_child_octant) * current_bounds.size / 2,
                            size: current_bounds.size / 2,
                        },
                    ));
                } else {
                    // no children are available for the target octant
                    if self.nodes.get(current_node_key).is_leaf() {
                        // The current Node is a leaf, which essentially represents an area where all the contained space have the same data.
                        // The contained data does not match the given data to set the position to, so all of the Nodes' children need to be created
                        // as separate Nodes with the same data as their parent to keep integrity, the node targeted for clean will update node count correctly
                        assert!(self.nodes.get(current_node_key).is_leaf());
                        let current_data = self.nodes.get(current_node_key).leaf_data().clone();
                        let new_children = self.make_uniform_children(current_data);
                        *self.nodes.get_mut(current_node_key) =
                            NodeContent::Internal(current_bounds.size.pow(3));
                        self.node_children[current_node_key].set(new_children);
                        node_stack.push((
                            self.node_children[current_node_key][target_child_octant],
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
                    removed_nodes_count = 1;
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
                    removed_nodes_count = clear_size.pow(3);
                } else {
                    // The size to clear equals, or is greater than DIM, the whole node is to be erased
                    // unset the current node and its children
                    self.deallocate_children_of(current_node_key as u32);

                    // Set the parents child to None
                    if node_stack.len() >= 2 && target_child_octant < 9 {
                        self.nodes.free(current_node_key);
                        let parent_key = node_stack[node_stack.len() - 2].0 as usize;
                        self.node_children[parent_key][target_child_octant] = key_none_value();
                    } else {
                        // If the node doesn't have parents, then it's a root node and should not be deleted
                        *self.nodes.get_mut(current_node_key) = NodeContent::Nothing;
                    }
                    removed_nodes_count = current_bounds.size.pow(3);
                }
                break;
            }
        }

        // post-processing operations
        node_stack.pop(); // Except for the last removed element
        for (node_key, _node_bounds) in node_stack.into_iter().rev() {
            match self.nodes.get(node_key as usize) {
                NodeContent::Nothing => {
                    *self.nodes.get_mut(node_key as usize) =
                        if !self.node_children[node_key as usize].is_empty() {
                            // This is incorrect information which needs to be corrected
                            // As the current Node is either a parent or a data leaf, it can not be Empty or nothing
                            // To correct this, the children will determine the count
                            NodeContent::Internal(self.count_cached_children(node_key))
                        } else {
                            NodeContent::Nothing
                        };
                }
                NodeContent::Internal(contains_count) => {
                    *self.nodes.get_mut(node_key as usize) =
                        if removed_nodes_count < *contains_count {
                            NodeContent::Internal(contains_count - removed_nodes_count)
                        } else {
                            NodeContent::Nothing
                        };
                }
                _ => {}
            }
        }
        Ok(())
    }
}
