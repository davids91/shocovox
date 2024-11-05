use crate::octree::{
    types::{NodeChildren, NodeChildrenArray, NodeContent, VoxelData},
    V3c,
};
use crate::spatial::{
    lut::OCTANT_OFFSET_REGION_LUT,
    math::{set_occupancy_in_bitmap_64bits, BITMAP_DIMENSION},
};

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
    /// Provides occupancy information for the part of the brick corresponmding
    /// to the given octant based on the contents of the brick
    pub(crate) fn is_empty_throughout(&self, octant: usize) -> bool {
        match self {
            BrickData::Empty => true,
            BrickData::Solid(voxel) => voxel.is_empty(),
            BrickData::Parted(brick) => {
                if 1 == DIM {
                    return brick[0][0][0].is_empty();
                }

                if 2 == DIM {
                    let octant_offset = V3c::<usize>::from(OCTANT_OFFSET_REGION_LUT[octant]);
                    return brick[octant_offset.x][octant_offset.y][octant_offset.z].is_empty();
                }

                let extent = BITMAP_DIMENSION as f32 / 2.;
                let octant_offset = V3c::<usize>::from(OCTANT_OFFSET_REGION_LUT[octant] * extent);
                for x in 0..extent as usize {
                    for y in 0..extent as usize {
                        for z in 0..extent as usize {
                            if !brick[octant_offset.x + x][octant_offset.y + y][octant_offset.z + z]
                                .is_empty()
                            {
                                return false;
                            }
                        }
                    }
                }
                true
            }
        }
    }

    /// Provides occupancy information for the part of the brick corresponding to
    /// part_octant and target octant. The Brick is subdivided on 2 levels,
    /// the larger target octant is set by @part_octant, the part inside that octant
    /// is set by @target_octant
    pub(crate) fn is_part_empty_throughout(
        &self,
        part_octant: usize,
        target_octant: usize,
    ) -> bool {
        match self {
            BrickData::Empty => true,
            BrickData::Solid(voxel) => voxel.is_empty(),
            BrickData::Parted(brick) => {
                if 1 == DIM {
                    brick[0][0][0].is_empty()
                } else if 2 == DIM {
                    let octant_offset = V3c::<usize>::from(OCTANT_OFFSET_REGION_LUT[part_octant]);
                    brick[octant_offset.x][octant_offset.y][octant_offset.z].is_empty()
                } else {
                    let outer_extent = BITMAP_DIMENSION as f32 / 2.;
                    let inner_extent = BITMAP_DIMENSION as f32 / 4.;
                    let octant_offset = V3c::<usize>::from(
                        OCTANT_OFFSET_REGION_LUT[part_octant] * outer_extent
                            + OCTANT_OFFSET_REGION_LUT[target_octant] * inner_extent,
                    );
                    for x in 0..inner_extent as usize {
                        for y in 0..inner_extent as usize {
                            for z in 0..inner_extent as usize {
                                if !brick[octant_offset.x + x][octant_offset.y + y]
                                    [octant_offset.z + z]
                                    .is_empty()
                                {
                                    return false;
                                }
                            }
                        }
                    }
                    true
                }
            }
        }
    }

    /// Calculates the Occupancy bitmap for the given Voxel brick
    pub(crate) fn calculate_brick_occupied_bits(brick: &[[[T; DIM]; DIM]; DIM]) -> u64 {
        let mut bitmap = 0;
        for x in 0..DIM {
            for y in 0..DIM {
                for z in 0..DIM {
                    if !brick[x][y][z].is_empty() {
                        set_occupancy_in_bitmap_64bits(
                            &V3c::new(x, y, z),
                            1,
                            DIM,
                            true,
                            &mut bitmap,
                        );
                    }
                }
            }
        }
        bitmap
    }

    /// Calculates the occupancy bitmap based on self
    pub(crate) fn calculate_occupied_bits(&self) -> u64 {
        match self {
            BrickData::Empty => 0,
            BrickData::Solid(voxel) => {
                if voxel.is_empty() {
                    0
                } else {
                    u64::MAX
                }
            }
            BrickData::Parted(brick) => Self::calculate_brick_occupied_bits(brick),
        }
    }

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
