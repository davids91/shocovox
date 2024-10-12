use crate::octree::types::{NodeChildren, NodeChildrenArray, NodeContent, VoxelData};
use crate::spatial::math::octant_bitmask;

///####################################################################################
/// NodeChildren
///####################################################################################
impl<T> NodeChildren<T>
where
    T: Default + Clone + Eq,
{
    /// Returns with true if empty
    pub(crate) fn is_empty(&self) -> bool {
        match &self.content {
            NodeChildrenArray::NoChildren => true,
            NodeChildrenArray::Children(_) => false,
            NodeChildrenArray::OccupancyBitmap(mask) => 0 == *mask,
            NodeChildrenArray::OccupancyBitmaps(masks) => 0 == masks.iter().fold(0, |m, x| m | x),
        }
    }

    /// Creates a new default element, with the given empty_marker
    pub(crate) fn new(empty_marker: T) -> Self {
        Self {
            empty_marker,
            content: NodeChildrenArray::default(),
        }
    }

    /// Provides a slice for iteration, if there are children to iterate on
    pub(crate) fn iter(&self) -> Option<std::slice::Iter<T>> {
        match &self.content {
            NodeChildrenArray::Children(c) => Some(c.iter()),
            _ => None,
        }
    }

    /// Erases content, if any
    pub(crate) fn clear(&mut self, child_index: usize) {
        debug_assert!(child_index < 8);
        if let NodeChildrenArray::Children(c) = &mut self.content {
            c[child_index] = self.empty_marker.clone();
            if 8 == c.iter().filter(|e| **e == self.empty_marker).count() {
                self.content = NodeChildrenArray::NoChildren;
            }
        }
    }

    /// Provides lvl2 occupancy bitmap based on the availability of the children
    pub(crate) fn occupied_bits(&self) -> u8 {
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

use super::types::BrickData;
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
/// BrickData
///####################################################################################
impl<T, const DIM: usize> BrickData<T, DIM>
where
    T: VoxelData + PartialEq + Clone + Copy + Default,
{
    /// In case all contained voxels are the same, returns with a reference to the data
    pub(crate) fn get_homogeneous_data(&self) -> Option<&T> {
        match self {
            BrickData::Empty => None,
            BrickData::Solid(voxel) => Some(voxel),
            BrickData::Parted(brick) => {
                for x in brick.iter() {
                    for y in x.iter() {
                        for voxel in y.iter() {
                            if *voxel != brick[0][0][0] {
                                return None;
                            }
                        }
                    }
                }
                Some(&brick[0][0][0])
            }
        }
    }

    /// Tries to simplify brick data, returns true if the view was simplified during function call
    pub(crate) fn simplify(&mut self) -> bool {
        if let Some(homogeneous_type) = self.get_homogeneous_data() {
            if homogeneous_type.is_empty() {
                *self = BrickData::Empty;
            } else {
                *self = BrickData::Solid(*homogeneous_type);
            }
            true
        } else {
            false
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
    #[cfg(debug_assertions)]
    pub(crate) fn count_non_empties(&self) -> usize {
        match self {
            NodeContent::Nothing | NodeContent::Internal(_) => 0,
            NodeContent::Leaf(bricks) => {
                let mut c = 0;
                for mat in bricks.iter() {
                    c += if matches!(mat, BrickData::Empty) {
                        0
                    } else {
                        1
                    };
                }
                c
            }
            NodeContent::UniformLeaf(brick) => {
                if matches!(brick, BrickData::Empty) {
                    0
                } else {
                    1
                }
            }
        }
    }

    /// Returns with true if it doesn't contain any data
    pub(crate) fn is_empty(&self) -> bool {
        match self {
            NodeContent::UniformLeaf(brick) => match brick {
                BrickData::Empty => true,
                BrickData::Solid(voxel) => voxel.is_empty(),
                BrickData::Parted(brick) => {
                    for x in brick.iter() {
                        for y in x.iter() {
                            for voxel in y.iter() {
                                if !voxel.is_empty() {
                                    return false;
                                }
                            }
                        }
                    }
                    true
                }
            },
            NodeContent::Leaf(bricks) => {
                for mat in bricks.iter() {
                    match mat {
                        BrickData::Empty => {
                            continue;
                        }
                        BrickData::Solid(voxel) => {
                            if !voxel.is_empty() {
                                return false;
                            }
                        }
                        BrickData::Parted(brick) => {
                            for x in brick.iter() {
                                for y in x.iter() {
                                    for voxel in y.iter() {
                                        if !voxel.is_empty() {
                                            return false;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                true
            }
            NodeContent::Internal(_) => false,
            NodeContent::Nothing => true,
        }
    }

    /// Returns with true if all contained elements equal the given data
    pub(crate) fn is_all(&self, data: &T) -> bool {
        match self {
            NodeContent::UniformLeaf(brick) => match brick {
                BrickData::Empty => false,
                BrickData::Solid(voxel) => voxel == data,
                BrickData::Parted(_brick) => {
                    if let Some(homogeneous_type) = brick.get_homogeneous_data() {
                        homogeneous_type == data
                    } else {
                        false
                    }
                }
            },
            NodeContent::Leaf(bricks) => {
                for mat in bricks.iter() {
                    let brick_is_all_data = match mat {
                        BrickData::Empty => false,
                        BrickData::Solid(voxel) => voxel == data,
                        BrickData::Parted(_brick) => {
                            if let Some(homogeneous_type) = mat.get_homogeneous_data() {
                                homogeneous_type == data
                            } else {
                                false
                            }
                        }
                    };
                    if !brick_is_all_data {
                        return false;
                    }
                }
                true
            }
            NodeContent::Internal(_) | NodeContent::Nothing => false,
        }
    }
}

impl<T, const DIM: usize> PartialEq for NodeContent<T, DIM>
where
    T: Clone + PartialEq + Clone + VoxelData,
{
    fn eq(&self, other: &NodeContent<T, DIM>) -> bool {
        match self {
            NodeContent::Nothing => matches!(other, NodeContent::Nothing),
            NodeContent::Internal(_) => false, // Internal nodes comparison doesn't make sense
            NodeContent::UniformLeaf(brick) => {
                if let NodeContent::UniformLeaf(obrick) = other {
                    brick == obrick
                } else {
                    false
                }
            }
            NodeContent::Leaf(bricks) => {
                if let NodeContent::Leaf(obricks) = other {
                    bricks == obricks
                } else {
                    false
                }
            }
        }
    }
}
