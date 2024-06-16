use crate::object_pool::empty_marker;
use crate::octree::types::{NodeChildren, NodeChildrenArray, NodeContent, Octree, VoxelData};
use crate::octree::{hash_region, Cube, V3c};
use crate::spatial::math::{octant_bitmask, set_occupancy_in_bitmap_64bits};

///####################################################################################
/// Utility functions
///####################################################################################

/// Returns whether the given bound contains the given position.
pub(in crate::octree) fn bound_contains(bounds: &Cube, position: &V3c<f32>) -> bool {
    position.x >= bounds.min_position.x
        && position.x < bounds.min_position.x + bounds.size
        && position.y >= bounds.min_position.y
        && position.y < bounds.min_position.y + bounds.size
        && position.z >= bounds.min_position.z
        && position.z < bounds.min_position.z + bounds.size
}

/// Returns with the octant value(i.e. index) of the child for the given position
pub(in crate::octree) fn child_octant_for(bounds: &Cube, position: &V3c<f32>) -> u8 {
    debug_assert!(bound_contains(bounds, position));
    hash_region(
        &(*position - bounds.min_position).into(),
        bounds.size as f32,
    )
}

///####################################################################################
/// NodeChildren
///####################################################################################
impl<T> NodeChildren<T>
where
    T: Default + Clone + PartialEq,
{
    pub(in crate::octree) fn is_empty(&self) -> bool {
        match &self.content {
            NodeChildrenArray::NoChildren => true,
            NodeChildrenArray::Children(_) => false,
            NodeChildrenArray::OccupancyBitmap(mask) => 0 == *mask,
        }
    }

    pub(in crate::octree) fn new(empty_marker: T) -> Self {
        Self {
            empty_marker,
            content: NodeChildrenArray::default(),
        }
    }

    pub(in crate::octree) fn from(empty_marker: T, children: [T; 8]) -> Self {
        Self {
            empty_marker,
            content: NodeChildrenArray::Children(children),
        }
    }
    pub(in crate::octree) fn bitmasked(empty_marker: T, bitmap: u64) -> Self {
        Self {
            empty_marker,
            content: NodeChildrenArray::OccupancyBitmap(bitmap),
        }
    }

    pub(in crate::octree) fn iter(&self) -> Option<std::slice::Iter<T>> {
        match &self.content {
            NodeChildrenArray::Children(c) => Some(c.iter()),
            _ => None,
        }
    }

    pub(in crate::octree) fn clear(&mut self, child_index: usize) {
        debug_assert!(child_index < 8);
        if let NodeChildrenArray::Children(c) = &mut self.content {
            c[child_index] = self.empty_marker.clone();
            if 8 == c.iter().filter(|e| **e == self.empty_marker).count() {
                self.content = NodeChildrenArray::NoChildren;
            }
        }
    }

    pub(in crate::octree) fn set(&mut self, children: [T; 8]) {
        self.content = NodeChildrenArray::Children(children)
    }

    fn occupied_bits(&self) -> u8 {
        match &self.content {
            NodeChildrenArray::Children(c) => {
                let mut result = 0;
                for (child_octant, child) in c.iter().enumerate().take(8) {
                    if *child != self.empty_marker {
                        result |= octant_bitmask(child_octant as u8);
                    }
                }
                result
            }
            _ => 0,
        }
    }

    #[cfg(feature = "bevy_wgpu")]
    pub(in crate::octree) fn get_full(&self) -> [T; 8] {
        match &self.content {
            NodeChildrenArray::Children(c) => c.clone(),
            _ => [
                self.empty_marker.clone(),
                self.empty_marker.clone(),
                self.empty_marker.clone(),
                self.empty_marker.clone(),
                self.empty_marker.clone(),
                self.empty_marker.clone(),
                self.empty_marker.clone(),
                self.empty_marker.clone(),
            ],
        }
    }
}

use std::{
    matches,
    ops::{Index, IndexMut},
};
impl<T> Index<u32> for NodeChildren<T>
where
    T: Default + Copy + Clone,
{
    type Output = T;
    fn index(&self, index: u32) -> &T {
        match &self.content {
            NodeChildrenArray::Children(c) => &c[index as usize],
            _ => &self.empty_marker,
        }
    }
}

impl<T> IndexMut<u32> for NodeChildren<T>
where
    T: Default + Copy + Clone,
{
    fn index_mut(&mut self, index: u32) -> &mut T {
        if let NodeChildrenArray::NoChildren = &mut self.content {
            self.content = NodeChildrenArray::Children([self.empty_marker; 8]);
        }
        match &mut self.content {
            NodeChildrenArray::Children(c) => &mut c[index as usize],
            _ => unreachable!(),
        }
    }
}

///####################################################################################
/// NodeContent
///####################################################################################
impl<T, const DIM: usize> NodeContent<T, DIM>
where
    T: VoxelData + PartialEq + Clone + Default,
{
    pub fn is_leaf(&self) -> bool {
        matches!(self, NodeContent::Leaf(_))
    }

    pub fn is_empty(&self) -> bool {
        match self {
            NodeContent::Leaf(d) => {
                for x in d.iter() {
                    for y in x.iter() {
                        for item in y.iter() {
                            if !item.is_empty() {
                                return false;
                            }
                        }
                    }
                }
                true
            }
            NodeContent::Nothing => true,
            NodeContent::Internal(_) => false,
        }
    }

    pub fn is_all(&self, data: &T) -> bool {
        match self {
            NodeContent::Leaf(d) => {
                for x in d.iter() {
                    for y in x.iter() {
                        for item in y.iter() {
                            if *item != *data {
                                return false;
                            }
                        }
                    }
                }
                true
            }
            _ => false,
        }
    }

    pub fn leaf_data(&self) -> &[[[T; DIM]; DIM]; DIM] {
        match self {
            NodeContent::Leaf(t) => t,
            _ => panic!("leaf_data was called for NodeContent<T> where there is no content!"),
        }
    }

    pub fn mut_leaf_data(&mut self) -> &mut [[[T; DIM]; DIM]; DIM] {
        match self {
            NodeContent::Leaf(t) => t,
            _ => panic!("leaf_data was called for NodeContent<T> where there is no content!"),
        }
    }

    pub fn as_leaf_ref(&self) -> Option<&[[[T; DIM]; DIM]; DIM]> {
        match self {
            NodeContent::Leaf(t) => Some(t),
            _ => None,
        }
    }

    pub fn as_mut_leaf_ref(&mut self) -> Option<&mut [[[T; DIM]; DIM]; DIM]> {
        match self {
            NodeContent::Leaf(t) => Some(t),
            _ => None,
        }
    }

    pub fn leaf_from(data: T) -> Self {
        NodeContent::Leaf(array_init::array_init(|_| {
            array_init::array_init(|_| array_init::array_init(|_| data.clone()))
        }))
    }
}

///####################################################################################
/// Octree
///####################################################################################
impl<T, const DIM: usize> Octree<T, DIM>
where
    T: Default + Clone + VoxelData,
{
    /// The root node is always the first item
    pub(crate) const ROOT_NODE_KEY: u32 = 0;
}

impl<T: Default + Clone + VoxelData, const DIM: usize> Octree<T, DIM> {
    pub(in crate::octree) fn mat_index(bounds: &Cube, position: &V3c<u32>) -> V3c<usize> {
        // --> In case the smallest possible node the contained matrix of voxels
        // starts at bounds min_position and ends in min_position + (DIM,DIM,DIM)
        // --> In case of bigger Nodes the below ratio equation is relevant
        // mat[xyz]/DIM = (position - min_position) / bounds.size
        let mat_index = (V3c::<usize>::from(*position - bounds.min_position.into()) * DIM)
            / bounds.size as usize;
        // The difference between the actual position and min bounds
        // must not be greater, than DIM at each dimension
        debug_assert!(mat_index.x < DIM);
        debug_assert!(mat_index.y < DIM);
        debug_assert!(mat_index.z < DIM);
        mat_index
    }

    pub(in crate::octree) fn bruteforce_occupancy_bitmask(brick: &[[[T; DIM]; DIM]; DIM]) -> u64 {
        let mut bitmask = 0u64;
        for x in 0..DIM {
            for y in 0..DIM {
                for z in 0..DIM {
                    set_occupancy_in_bitmap_64bits(
                        x,
                        y,
                        z,
                        DIM,
                        !brick[x][y][z].is_empty(),
                        &mut bitmask,
                    );
                }
            }
        }
        bitmask
    }

    pub(in crate::octree) fn make_uniform_children(
        &mut self,
        content: [[[T; DIM]; DIM]; DIM],
    ) -> [u32; 8] {
        // Create new children leaf nodes based on the provided content
        let occupancy_bitmap = Self::bruteforce_occupancy_bitmask(&content);
        let children = [
            self.nodes.push(NodeContent::Leaf(content.clone())) as u32,
            self.nodes.push(NodeContent::Leaf(content.clone())) as u32,
            self.nodes.push(NodeContent::Leaf(content.clone())) as u32,
            self.nodes.push(NodeContent::Leaf(content.clone())) as u32,
            self.nodes.push(NodeContent::Leaf(content.clone())) as u32,
            self.nodes.push(NodeContent::Leaf(content.clone())) as u32,
            self.nodes.push(NodeContent::Leaf(content.clone())) as u32,
            self.nodes.push(NodeContent::Leaf(content)) as u32,
        ];

        // node_children array needs to be resized to fit the new children
        self.node_children
            .resize(self.nodes.len(), NodeChildren::new(empty_marker()));

        // each new children is a leaf, so node_children needs to be adapted to that
        for c in children {
            self.node_children[c as usize] =
                NodeChildren::bitmasked(empty_marker(), occupancy_bitmap);
        }
        children
    }

    pub(in crate::octree) fn deallocate_children_of(&mut self, node: u32) {
        let mut to_deallocate = Vec::new();
        if let Some(children) = self.node_children[node as usize].iter() {
            for child in children {
                if self.nodes.key_is_valid(*child as usize) {
                    to_deallocate.push(*child);
                }
            }
            for child in to_deallocate {
                self.deallocate_children_of(child); // Recursion should be fine as depth is not expceted to be more, than 32
                self.nodes.free(child as usize);
            }
        }
        self.node_children[node as usize].content = NodeChildrenArray::NoChildren;
    }

    /// Calculates the occupied bits of a Node; For empty nodes(Nodecontent::Nothing) as well;
    /// As they might be empty by fault and to correct them the occupied bits is required.
    /// Leaf node occupancy bitmap should not be calculated by this function
    pub(in crate::octree) fn occupied_8bit(&self, node: u32) -> u8 {
        match self.nodes.get(node as usize) {
            NodeContent::Leaf(_) => {
                let leaf_occupied_bits = match self.node_children[node as usize].content {
                    NodeChildrenArray::OccupancyBitmap(occupied_bits) => occupied_bits,
                    _ => {
                        debug_assert!(false);
                        0
                    }
                };
                (((leaf_occupied_bits & 0x0000000000330033) > 0) as u8) << 0
                    | (((leaf_occupied_bits & 0x0000000000cc00cc) > 0) as u8) << 1
                    | (((leaf_occupied_bits & 0x0033003300000000) > 0) as u8) << 2
                    | (((leaf_occupied_bits & 0x00cc00cc00000000) > 0) as u8) << 3
                    | (((leaf_occupied_bits & 0x0000000033003300) > 0) as u8) << 4
                    | (((leaf_occupied_bits & 0x00000000cc00cc00) > 0) as u8) << 5
                    | (((leaf_occupied_bits & 0x3300330000000000) > 0) as u8) << 6
                    | (((leaf_occupied_bits & 0xcc00cc0000000000) > 0) as u8) << 7
            }
            _ => self.node_children[node as usize].occupied_bits(),
        }
    }
}
