use crate::octree::{
    types::{Albedo, NodeChildren, NodeChildrenArray, NodeContent, Octree, VoxelData},
    {Cube, V3c},
};
use crate::spatial::math::{
    hash_region, octant_bitmask, offset_region, set_occupancy_in_bitmap_64bits,
};

///####################################################################################
/// Utility functions
///####################################################################################

/// Returns whether the given bound contains the given position.
pub(crate) fn bound_contains(bounds: &Cube, position: &V3c<f32>) -> bool {
    position.x >= bounds.min_position.x
        && position.x < bounds.min_position.x + bounds.size
        && position.y >= bounds.min_position.y
        && position.y < bounds.min_position.y + bounds.size
        && position.z >= bounds.min_position.z
        && position.z < bounds.min_position.z + bounds.size
}

/// Returns with the octant value(i.e. index) of the child for the given position
pub(crate) fn child_octant_for(bounds: &Cube, position: &V3c<f32>) -> u8 {
    debug_assert!(bound_contains(bounds, position));
    hash_region(&(*position - bounds.min_position), bounds.size / 2.)
}

///####################################################################################
/// Type implements
///####################################################################################
impl VoxelData for Albedo {
    fn new(color: Albedo, _user_data: u32) -> Self {
        color
    }

    fn albedo(&self) -> Albedo {
        *self
    }

    fn user_data(&self) -> u32 {
        0u32
    }

    fn clear(&mut self) {
        self.r = 0;
        self.g = 0;
        self.b = 0;
        self.a = 0;
    }
}

impl From<u32> for Albedo {
    fn from(value: u32) -> Self {
        let a = (value & 0x000000FF) as u8;
        let b = ((value & 0x0000FF00) >> 8) as u8;
        let g = ((value & 0x00FF0000) >> 16) as u8;
        let r = ((value & 0xFF000000) >> 24) as u8;

        Albedo::default()
            .with_red(r)
            .with_green(g)
            .with_blue(b)
            .with_alpha(a)
    }
}

///####################################################################################
/// NodeChildren
///####################################################################################
impl<T> NodeChildren<T>
where
    T: Default + Clone + Eq,
{
    pub(crate) fn is_empty(&self) -> bool {
        match &self.content {
            NodeChildrenArray::NoChildren => true,
            NodeChildrenArray::Children(_) => false,
            NodeChildrenArray::OccupancyBitmap(mask) => 0 == *mask,
            NodeChildrenArray::OccupancyBitmaps(masks) => 0 == masks.iter().sum::<u64>(),
        }
    }

    pub(crate) fn new(empty_marker: T) -> Self {
        Self {
            empty_marker,
            content: NodeChildrenArray::default(),
        }
    }

    pub(crate) fn iter(&self) -> Option<std::slice::Iter<T>> {
        match &self.content {
            NodeChildrenArray::Children(c) => Some(c.iter()),
            _ => None,
        }
    }

    pub(crate) fn clear(&mut self, child_index: usize) {
        debug_assert!(child_index < 8);
        if let NodeChildrenArray::Children(c) = &mut self.content {
            c[child_index] = self.empty_marker.clone();
            if 8 == c.iter().filter(|e| **e == self.empty_marker).count() {
                self.content = NodeChildrenArray::NoChildren;
            }
        }
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
    T: VoxelData + PartialEq + Clone + Copy + Default,
{
    pub(crate) fn subdivide_leaf(&mut self, children: &mut NodeChildrenArray<u32>) {
        match self {
            NodeContent::Nothing | NodeContent::Internal(_) => {
                panic!("Non-leaf node expected to be Leaf")
            }
            NodeContent::Leaf(_) => {
                // The leaf is already as divided as it can be
                debug_assert!(matches!(children, NodeChildrenArray::OccupancyBitmaps(_)));
            }
            NodeContent::UniformLeaf(mat) => {
                // The leaf will be divided into 8 bricks
                debug_assert!(matches!(children, NodeChildrenArray::OccupancyBitmap(_)));
                let mut leaf_bricks: [Option<Box<[[[T; DIM]; DIM]; DIM]>>; 8] =
                    [None, None, None, None, None, None, None, None];
                let mut children_bitmaps = [0u64; 8];

                // Each brick is mapped to take up one subsection of the current data
                for brick_octant in 0..8usize {
                    let brick_offset = V3c::<usize>::from(offset_region(brick_octant as u8)) * 2;
                    leaf_bricks[brick_octant] = Some(Box::new(
                        [[[mat[brick_offset.x][brick_offset.y][brick_offset.z]; DIM]; DIM]; DIM],
                    ));
                    if let Some(ref mut brick) = leaf_bricks[brick_octant] {
                        for x in 0..DIM {
                            for y in 0..DIM {
                                for z in 0..DIM {
                                    set_occupancy_in_bitmap_64bits(
                                        x,
                                        y,
                                        z,
                                        DIM,
                                        !brick[x][y][z].is_empty(),
                                        &mut children_bitmaps[brick_octant],
                                    );
                                    if x < 2 && y < 2 && z < 2 {
                                        continue;
                                    }
                                    brick[x][y][z] = mat[brick_offset.x + x / 2]
                                        [brick_offset.y + y / 2][brick_offset.z + z / 2];
                                }
                            }
                        }
                    }
                }
                *children = NodeChildrenArray::OccupancyBitmaps(children_bitmaps);
                *self = NodeContent::Leaf(leaf_bricks);
            }
            NodeContent::HomogeneousLeaf(data) => {
                debug_assert!(matches!(children, NodeChildrenArray::OccupancyBitmap(_)));
                *self = NodeContent::Leaf([
                    Some(Box::new([[[*data; DIM]; DIM]; DIM])),
                    Some(Box::new([[[*data; DIM]; DIM]; DIM])),
                    Some(Box::new([[[*data; DIM]; DIM]; DIM])),
                    Some(Box::new([[[*data; DIM]; DIM]; DIM])),
                    Some(Box::new([[[*data; DIM]; DIM]; DIM])),
                    Some(Box::new([[[*data; DIM]; DIM]; DIM])),
                    Some(Box::new([[[*data; DIM]; DIM]; DIM])),
                    Some(Box::new([[[*data; DIM]; DIM]; DIM])),
                ]);
                *children = NodeChildrenArray::OccupancyBitmaps([0xFFFFFFFFFFFFFFFF; 8]);
            }
        }
    }

    pub(crate) fn is_leaf(&self) -> bool {
        matches!(self, NodeContent::Leaf(_))
    }

    pub(crate) fn is_empty(&self) -> bool {
        match self {
            NodeContent::Leaf(mats) => {
                for mat in mats.iter() {
                    if mat.is_none() {
                        continue;
                    }
                    for x in mat.iter() {
                        for y in x.iter() {
                            for item in y.iter() {
                                if !item.is_empty() {
                                    return false;
                                }
                            }
                        }
                    }
                }
                true
            }
            NodeContent::UniformLeaf(mat) => {
                for x in mat.iter() {
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
            NodeContent::HomogeneousLeaf(d) => d.is_empty(),
            NodeContent::Internal(_) => false,
            NodeContent::Nothing => true,
        }
    }

    pub(crate) fn is_all(&self, data: &T) -> bool {
        match self {
            NodeContent::Leaf(mats) => {
                for mat in mats.iter() {
                    if let Some(mat) = mat {
                        for x in mat.iter() {
                            for y in x.iter() {
                                for item in y.iter() {
                                    if *item != *data {
                                        return false;
                                    }
                                }
                            }
                        }
                    } else {
                        return false;
                    }
                }
                true
            }
            NodeContent::UniformLeaf(mat) => {
                for x in mat.iter() {
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
            NodeContent::HomogeneousLeaf(d) => d == data,
            NodeContent::Internal(_) | NodeContent::Nothing => false,
        }
    }
}

impl<T, const DIM: usize> PartialEq for NodeContent<T, DIM>
where
    T: Clone + PartialEq,
{
    fn eq(&self, other: &NodeContent<T, DIM>) -> bool {
        match self {
            NodeContent::Nothing => matches!(other, NodeContent::Nothing),
            NodeContent::Internal(_) => false, // Internal nodes comparison doesn't make sense
            NodeContent::HomogeneousLeaf(d) => {
                if let NodeContent::HomogeneousLeaf(od) = other {
                    d == od
                } else {
                    false
                }
            }
            NodeContent::UniformLeaf(mat) => {
                if let NodeContent::UniformLeaf(omat) = other {
                    mat == omat
                } else {
                    false
                }
            }
            NodeContent::Leaf(mats) => {
                if let NodeContent::Leaf(omats) = other {
                    mats == omats
                } else {
                    false
                }
            }
        }
    }
}

///####################################################################################
/// Octree
///####################################################################################
impl<T, const DIM: usize> Octree<T, DIM>
where
    T: Default + Clone + PartialEq + VoxelData,
{
    /// The root node is always the first item
    pub(crate) const ROOT_NODE_KEY: u32 = 0;
}

impl<T, const DIM: usize> Octree<T, DIM>
where
    T: Default + Clone + PartialEq + VoxelData,
{
    pub(crate) fn mat_index(bounds: &Cube, position: &V3c<u32>) -> V3c<usize> {
        // The position should be inside the bounds
        debug_assert!(
            bounds.min_position.x <= position.x as f32
                && bounds.min_position.y <= position.y as f32
                && bounds.min_position.z <= position.z as f32
                && bounds.min_position.x + bounds.size > position.x as f32
                && bounds.min_position.y + bounds.size > position.y as f32
                && bounds.min_position.z + bounds.size > position.z as f32
        );

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

    pub(crate) fn deallocate_children_of(&mut self, node: u32) {
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
    pub(crate) fn occupied_8bit(&self, node: u32) -> u8 {
        match self.nodes.get(node as usize) {
            NodeContent::Leaf(_) => {
                let leaf_occupied_bits = match self.node_children[node as usize].content {
                    NodeChildrenArray::OccupancyBitmap(occupied_bits) => occupied_bits,
                    _ => {
                        debug_assert!(false);
                        0
                    }
                };
                (((leaf_occupied_bits & 0x0000000000330033) > 0) as u8)
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
