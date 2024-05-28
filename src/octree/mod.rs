pub mod bytecode;
pub mod detail;
pub mod tests;
pub mod types;
pub mod update;

#[cfg(feature = "raytracing")]
pub mod raytracing;

pub use crate::spatial::math::vector::V3c;
pub use types::{Octree, VoxelData};

use crate::object_pool::{key_none_value, ObjectPool};
use crate::octree::{
    detail::{bound_contains, child_octant_for},
    types::{NodeChildren, NodeContent, OctreeError},
};
use crate::spatial::{math::hash_region, Cube};
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
                        self.node_children[current_node_key][child_octant_at_position as u32];
                    if self.nodes.key_is_valid(child_at_position as usize) {
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
                        self.node_children[current_node_key][child_octant_at_position as u32];
                    if self.nodes.key_is_valid(child_at_position as usize) {
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
}
