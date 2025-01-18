pub mod types;
pub mod update;

mod convert;
mod detail;
mod node;

#[cfg(test)]
mod tests;

#[cfg(feature = "raytracing")]
pub mod raytracing;

pub use crate::spatial::math::vector::{V3c, V3cf32};
pub use types::{Albedo, Octree, VoxelData};

use crate::object_pool::{empty_marker, ObjectPool};
use crate::octree::{
    detail::{bound_contains, child_octant_for},
    types::{BrickData, NodeChildren, NodeContent, OctreeError},
};
use crate::spatial::{
    math::{flat_projection, matrix_index_for},
    Cube,
};
use bendy::{decoding::FromBencode, encoding::ToBencode};

#[cfg(debug_assertions)]
use crate::spatial::math::position_in_bitmap_64bits;

impl<T> Octree<T>
where
    T: Default + Eq + Clone + Copy + VoxelData,
{
    /// converts the data structure to a byte representation
    pub fn to_bytes(&self) -> Vec<u8> {
        self.to_bencode().ok().unwrap()
    }

    /// parses the data structure from a byte string
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self::from_bencode(&bytes).ok().unwrap()
    }

    /// saves the data structure to the given file path
    pub fn save(&self, path: &str) -> Result<(), std::io::Error> {
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
    /// Generic parameter self.octree_dim must be one of `(2^x)` and smaller, than the size of the octree
    /// * `size` - must be `self.octree_dim * (2^x)`, e.g: self.octree_dim == 2 --> size can be 2,4,8,16,32...
    pub fn new(size: u32, brick_dimension: u32) -> Result<Self, OctreeError> {
        if 0 == size || (brick_dimension as f32).log(2.0).fract() != 0.0 {
            return Err(OctreeError::InvalidBrickDimension(brick_dimension));
        }
        if brick_dimension > size
            || 0 == size
            || (size as f32 / brick_dimension as f32).log(2.0).fract() != 0.0
        {
            return Err(OctreeError::InvalidSize(size));
        }
        if brick_dimension >= size {
            return Err(OctreeError::InvalidStructure(
                "Octree size must be larger, than the brick dimension".into(),
            ));
        }
        let node_count_estimation = (size / brick_dimension).pow(3);
        let mut nodes =
            ObjectPool::<NodeContent<T>>::with_capacity(node_count_estimation.min(1024) as usize);
        let mut node_children = Vec::with_capacity(node_count_estimation.min(1024) as usize * 8);
        node_children.push(NodeChildren::new(empty_marker()));
        let root_node_key = nodes.push(NodeContent::Nothing); // The first element is the root Node
        assert!(root_node_key == 0);
        Ok(Self {
            auto_simplify: true,
            octree_size: size,
            brick_dim: brick_dimension,
            nodes,
            node_children,
        })
    }

    /// Provides immutable reference to the data, if there is any at the given position
    pub fn get(&self, position: &V3c<u32>) -> Option<&T> {
        let mut current_bounds = Cube::root_bounds(self.octree_size as f32);
        let mut current_node_key = Self::ROOT_NODE_KEY as usize;
        let position = V3c::from(*position);
        if !bound_contains(&current_bounds, &position) {
            return None;
        }

        loop {
            match self.nodes.get(current_node_key) {
                NodeContent::Nothing => return None,
                NodeContent::Leaf(bricks) => {
                    // In case self.octree_dim == octree size, the root node can not be a leaf...
                    debug_assert!(self.brick_dim < self.octree_size);
                    debug_assert!(
                        0 < self.nodes.get(current_node_key).count_non_empties(),
                        "At least some children should be Some(x) in a Leaf!"
                    );
                    // Hash the position to the target child
                    let child_octant_at_position = child_octant_for(&current_bounds, &position);

                    // If the child exists, query it for the voxel
                    match &bricks[child_octant_at_position as usize] {
                        BrickData::Empty => {
                            return None;
                        }
                        BrickData::Parted(brick) => {
                            current_bounds =
                                Cube::child_bounds_for(&current_bounds, child_octant_at_position);
                            let mat_index = matrix_index_for(
                                &current_bounds,
                                &V3c::from(position),
                                self.brick_dim,
                            );
                            let mat_index = flat_projection(
                                mat_index.x as usize,
                                mat_index.y as usize,
                                mat_index.z as usize,
                                self.brick_dim as usize,
                            );
                            if !brick[mat_index].is_empty() {
                                return Some(&brick[mat_index]);
                            }
                            return None;
                        }
                        BrickData::Solid(voxel) => {
                            return Some(voxel);
                        }
                    }
                }
                NodeContent::UniformLeaf(brick) => match brick {
                    BrickData::Empty => {
                        return None;
                    }
                    BrickData::Parted(brick) => {
                        let mat_index =
                            matrix_index_for(&current_bounds, &V3c::from(position), self.brick_dim);
                        let mat_index = flat_projection(
                            mat_index.x as usize,
                            mat_index.y as usize,
                            mat_index.z as usize,
                            self.brick_dim as usize,
                        );
                        if brick[mat_index].is_empty() {
                            return None;
                        }
                        return Some(&brick[mat_index]);
                    }
                    BrickData::Solid(voxel) => {
                        if voxel.is_empty() {
                            return None;
                        }
                        return Some(voxel);
                    }
                },
                NodeContent::Internal(occupied_bits) => {
                    // Hash the position to the target child
                    let child_octant_at_position = child_octant_for(&current_bounds, &position);
                    let child_at_position =
                        self.node_children[current_node_key][child_octant_at_position as u32];

                    // There is a valid child at the given position inside the node, recurse into it
                    if self.nodes.key_is_valid(child_at_position as usize) {
                        #[cfg(debug_assertions)]
                        {
                            // calculate the corresponding position in the nodes occupied bits
                            let pos_in_node =
                                matrix_index_for(&current_bounds, &(position.into()), 4);

                            let should_bit_be_empty = self.should_bitmap_be_empty_at_position(
                                current_node_key,
                                &current_bounds,
                                &position,
                            );

                            let pos_in_bitmap = position_in_bitmap_64bits(&pos_in_node, 4);
                            let is_bit_empty = 0 == (occupied_bits & (0x01 << pos_in_bitmap));
                            // the corresponding bit should be set
                            debug_assert!(
                                 (should_bit_be_empty && is_bit_empty)||(!should_bit_be_empty && !is_bit_empty),
                                  "Node[{:?}] under {:?} \n has a child in octant[{:?}](global position: {:?}), which is incompatible with the occupancy bitmap: {:#10X}",
                                  current_node_key,
                                  current_bounds,
                                  child_octant_at_position,
                                  position, occupied_bits
                            );
                        }
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

    /// Provides a mutable reference to the voxel inside the given node
    /// Requires the bounds of the Node, and the position inside the node its providing reference from
    fn get_mut_ref(
        &mut self,
        bounds: &Cube,
        position: &V3c<f32>,
        node_key: usize,
    ) -> Option<&mut T> {
        debug_assert!(bound_contains(bounds, position));
        match self.nodes.get_mut(node_key) {
            NodeContent::Leaf(bricks) => {
                // In case self.octree_dim == octree size, the root node can not be a leaf...
                debug_assert!(self.brick_dim < self.octree_size);

                // Hash the position to the target child
                let child_octant_at_position = child_octant_for(bounds, position);

                // If the child exists, query it for the voxel
                match &mut bricks[child_octant_at_position as usize] {
                    BrickData::Empty => None,
                    BrickData::Parted(ref mut brick) => {
                        let bounds = Cube::child_bounds_for(bounds, child_octant_at_position);
                        let mat_index =
                            matrix_index_for(&bounds, &V3c::from(*position), self.brick_dim);
                        let mat_index = flat_projection(
                            mat_index.x as usize,
                            mat_index.y as usize,
                            mat_index.z as usize,
                            self.brick_dim as usize,
                        );
                        if !brick[mat_index].is_empty() {
                            return Some(&mut brick[mat_index]);
                        }
                        None
                    }
                    BrickData::Solid(ref mut voxel) => Some(voxel),
                }
            }
            NodeContent::UniformLeaf(brick) => match brick {
                BrickData::Empty => None,
                BrickData::Parted(brick) => {
                    let mat_index = matrix_index_for(bounds, &V3c::from(*position), self.brick_dim);
                    let mat_index = flat_projection(
                        mat_index.x as usize,
                        mat_index.y as usize,
                        mat_index.z as usize,
                        self.brick_dim as usize,
                    );
                    if brick[mat_index].is_empty() {
                        return None;
                    }
                    Some(&mut brick[mat_index])
                }
                BrickData::Solid(voxel) => {
                    if voxel.is_empty() {
                        return None;
                    }
                    Some(voxel)
                }
            },
            &mut NodeContent::Nothing | &mut NodeContent::Internal(_) => None,
        }
    }

    /// Provides mutable reference to the data, if there is any at the given position
    pub fn get_mut(&mut self, position: &V3c<u32>) -> Option<&mut T> {
        let mut current_bounds = Cube::root_bounds(self.octree_size as f32);
        let mut current_node_key = Self::ROOT_NODE_KEY as usize;
        let position = V3c::from(*position);
        if !bound_contains(&current_bounds, &position) {
            return None;
        }

        loop {
            match self.nodes.get(current_node_key) {
                NodeContent::Nothing => {
                    return None;
                }
                NodeContent::Internal(occupied_bits) => {
                    // Hash the position to the target child
                    let child_octant_at_position = child_octant_for(&current_bounds, &position);
                    let child_at_position =
                        self.node_children[current_node_key][child_octant_at_position as u32];

                    // If the target child is valid, recurse into it
                    if self.nodes.key_is_valid(child_at_position as usize) {
                        #[cfg(debug_assertions)]
                        {
                            // calculate the corresponding position in the nodes occupied bits
                            let pos_in_node =
                                matrix_index_for(&current_bounds, &(position.into()), 4);

                            // the corresponding bit should be set
                            debug_assert!(
                                0 != (occupied_bits
                                    & 0x01
                                        << position_in_bitmap_64bits(
                                            &pos_in_node,
                                            4
                                        )),
                                "Node[{current_node_key}] under {:?} has a child in octant[{child_octant_at_position}](global position: {:?}), which is not shown in the occupancy bitmap: {:#10X}",
                                current_bounds,
                                position, occupied_bits
                            );
                        }
                        current_node_key = child_at_position as usize;
                        current_bounds =
                            Cube::child_bounds_for(&current_bounds, child_octant_at_position);
                    } else {
                        return None;
                    }
                }
                NodeContent::Leaf(_) | NodeContent::UniformLeaf(_) => {
                    debug_assert!(
                        0 < self.nodes.get(current_node_key).count_non_empties(),
                        "At least some children should be Some(x) in a Leaf!"
                    );
                    return self.get_mut_ref(&current_bounds, &position, current_node_key);
                }
            }
        }
    }

    /// Tells the radius of the area covered by the octree
    pub fn get_size(&self) -> u32 {
        self.octree_size
    }
}
