use crate::octree::{
    types::{
        Albedo, BrickData, NodeChildren, NodeChildrenArray, NodeContent, VoxelContent, VoxelData,
    },
    V3c,
};
use crate::spatial::{
    lut::OCTANT_OFFSET_REGION_LUT,
    math::{flat_projection, set_occupancy_in_bitmap_64bits},
};

use std::{
    fmt::{Debug, Error, Formatter},
    matches,
    ops::{Index, IndexMut},
};

//####################################################################################
//  ██████   █████    ███████    ██████████   ██████████
// ░░██████ ░░███   ███░░░░░███ ░░███░░░░███ ░░███░░░░░█
//  ░███░███ ░███  ███     ░░███ ░███   ░░███ ░███  █ ░
//  ░███░░███░███ ░███      ░███ ░███    ░███ ░██████
//  ░███ ░░██████ ░███      ░███ ░███    ░███ ░███░░█
//  ░███  ░░█████ ░░███     ███  ░███    ███  ░███ ░   █
//  █████  ░░█████ ░░░███████░   ██████████   ██████████
// ░░░░░    ░░░░░    ░░░░░░░    ░░░░░░░░░░   ░░░░░░░░░░
//    █████████  █████   █████ █████ █████       ██████████   ███████████   ██████████ ██████   █████
//   ███░░░░░███░░███   ░░███ ░░███ ░░███       ░░███░░░░███ ░░███░░░░░███ ░░███░░░░░█░░██████ ░░███
//  ███     ░░░  ░███    ░███  ░███  ░███        ░███   ░░███ ░███    ░███  ░███  █ ░  ░███░███ ░███
// ░███          ░███████████  ░███  ░███        ░███    ░███ ░██████████   ░██████    ░███░░███░███
// ░███          ░███░░░░░███  ░███  ░███        ░███    ░███ ░███░░░░░███  ░███░░█    ░███ ░░██████
// ░░███     ███ ░███    ░███  ░███  ░███      █ ░███    ███  ░███    ░███  ░███ ░   █ ░███  ░░█████
//  ░░█████████  █████   █████ █████ ███████████ ██████████   █████   █████ ██████████ █████  ░░█████
//   ░░░░░░░░░  ░░░░░   ░░░░░ ░░░░░ ░░░░░░░░░░░ ░░░░░░░░░░   ░░░░░   ░░░░░ ░░░░░░░░░░ ░░░░░    ░░░░░
//####################################################################################
impl<T: Default + Debug> Debug for NodeChildrenArray<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match &self {
            NodeChildrenArray::NoChildren => write!(f, "NodeChildrenArray::NoChildren"),
            NodeChildrenArray::Children(array) => {
                write!(f, "NodeChildrenArray::Children({:?})", array)
            }
            NodeChildrenArray::OccupancyBitmap(mask) => {
                write!(f, "NodeChildrenArray::OccupancyBitmap({:#10X})", mask)
            }
        }
    }
}
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

//####################################################################################
//  ███████████  ███████████   █████   █████████  █████   ████
// ░░███░░░░░███░░███░░░░░███ ░░███   ███░░░░░███░░███   ███░
//  ░███    ░███ ░███    ░███  ░███  ███     ░░░  ░███  ███
//  ░██████████  ░██████████   ░███ ░███          ░███████
//  ░███░░░░░███ ░███░░░░░███  ░███ ░███          ░███░░███
//  ░███    ░███ ░███    ░███  ░███ ░░███     ███ ░███ ░░███
//  ███████████  █████   █████ █████ ░░█████████  █████ ░░████
// ░░░░░░░░░░░  ░░░░░   ░░░░░ ░░░░░   ░░░░░░░░░  ░░░░░   ░░░░
//  ██████████     █████████   ███████████   █████████
// ░░███░░░░███   ███░░░░░███ ░█░░░███░░░█  ███░░░░░███
//  ░███   ░░███ ░███    ░███ ░   ░███  ░  ░███    ░███
//  ░███    ░███ ░███████████     ░███     ░███████████
//  ░███    ░███ ░███░░░░░███     ░███     ░███░░░░░███
//  ░███    ███  ░███    ░███     ░███     ░███    ░███
//  ██████████   █████   █████    █████    █████   █████
// ░░░░░░░░░░   ░░░░░   ░░░░░    ░░░░░    ░░░░░   ░░░░░
//####################################################################################
impl BrickData<VoxelContent> {
    /// Provides occupancy information for the part of the brick corresponding
    /// to the given octant based on the contents of the brick
    pub(crate) fn is_empty_throughout<V: VoxelData>(
        &self,
        octant: usize,
        brick_dim: usize,
        color_palette: &[Albedo],
        data_palette: &[V],
    ) -> bool {
        match self {
            BrickData::Empty => true,
            BrickData::Solid(voxel) => voxel.points_to_empty(color_palette, data_palette),
            BrickData::Parted(brick) => {
                if 1 == brick_dim {
                    debug_assert!(
                        1 == brick.len(),
                        "Expected brick length to align with given brick dimension: {}^3 != {}",
                        brick.len(),
                        brick_dim
                    );
                    return brick[0].points_to_empty(color_palette, data_palette);
                }

                if 2 == brick_dim {
                    debug_assert!(
                        8 == brick.len(),
                        "Expected brick length to align with given brick dimension: {}^3 != {}",
                        brick.len(),
                        brick_dim
                    );
                    let octant_offset = V3c::<usize>::from(OCTANT_OFFSET_REGION_LUT[octant]);
                    let octant_flat_offset =
                        flat_projection(octant_offset.x, octant_offset.y, octant_offset.z, 2);
                    return brick[octant_flat_offset].points_to_empty(color_palette, data_palette);
                }

                debug_assert!(
                    brick.len() > 8,
                    "Expected brick length to align with given brick dimension: {}^3 != {}",
                    brick.len(),
                    brick_dim
                );
                let octant_extent = brick_dim / 2usize;
                let octant_offset =
                    V3c::<usize>::from(OCTANT_OFFSET_REGION_LUT[octant]) * octant_extent;
                for x in octant_offset.x..(octant_offset.x + octant_extent) {
                    for y in octant_offset.y..(octant_offset.y + octant_extent) {
                        for z in octant_offset.z..(octant_offset.z + octant_extent) {
                            let octant_flat_offset = flat_projection(x, y, z, brick_dim);
                            if !brick[octant_flat_offset]
                                .points_to_empty(color_palette, data_palette)
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
    pub(crate) fn is_part_empty_throughout<V: VoxelData>(
        &self,
        part_octant: usize,
        target_octant: usize,
        brick_dim: usize,
        color_palette: &[Albedo],
        data_palette: &[V],
    ) -> bool {
        match self {
            BrickData::Empty => true,
            BrickData::Solid(voxel) => voxel.points_to_empty(color_palette, data_palette),
            BrickData::Parted(brick) => {
                if 1 == brick.len() {
                    // brick dimension is 1
                    brick[0].points_to_empty(color_palette, data_palette)
                } else if 8 == brick.len() {
                    // brick dimension is 2
                    let octant_offset = V3c::<usize>::from(OCTANT_OFFSET_REGION_LUT[part_octant]);
                    let octant_flat_offset =
                        flat_projection(octant_offset.x, octant_offset.y, octant_offset.z, 2);
                    brick[octant_flat_offset].points_to_empty(color_palette, data_palette)
                } else {
                    let outer_extent = brick_dim as f32 / 2.;
                    let inner_extent = brick_dim as f32 / 4.;
                    let octant_offset = V3c::<usize>::from(
                        OCTANT_OFFSET_REGION_LUT[part_octant] * outer_extent
                            + OCTANT_OFFSET_REGION_LUT[target_octant] * inner_extent,
                    );

                    for x in 0..inner_extent as usize {
                        for y in 0..inner_extent as usize {
                            for z in 0..inner_extent as usize {
                                let octant_flat_offset = flat_projection(
                                    octant_offset.x + x,
                                    octant_offset.y + y,
                                    octant_offset.z + z,
                                    brick_dim,
                                );
                                if !brick[octant_flat_offset]
                                    .points_to_empty(color_palette, data_palette)
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
    pub(crate) fn calculate_brick_occupied_bits<V: VoxelData>(
        brick: &[VoxelContent],
        brick_dimension: usize,
        color_palette: &[Albedo],
        data_palette: &[V],
    ) -> u64 {
        let mut bitmap = 0;
        for x in 0..brick_dimension {
            for y in 0..brick_dimension {
                for z in 0..brick_dimension {
                    let flat_index = flat_projection(x, y, z, brick_dimension);
                    if !brick[flat_index].points_to_empty(color_palette, data_palette) {
                        set_occupancy_in_bitmap_64bits(
                            &V3c::new(x, y, z),
                            1,
                            brick_dimension,
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
    pub(crate) fn calculate_occupied_bits<V: VoxelData>(
        &self,
        brick_dimension: usize,
        color_palette: &[Albedo],
        data_palette: &[V],
    ) -> u64 {
        match self {
            BrickData::Empty => 0,
            BrickData::Solid(voxel) => {
                if voxel.points_to_empty(color_palette, data_palette) {
                    0
                } else {
                    u64::MAX
                }
            }
            BrickData::Parted(brick) => Self::calculate_brick_occupied_bits(
                brick,
                brick_dimension,
                color_palette,
                data_palette,
            ),
        }
    }

    /// In case all contained voxels are the same, returns with a reference to the data
    pub(crate) fn get_homogeneous_data(&self) -> Option<&VoxelContent> {
        match self {
            BrickData::Empty => None,
            BrickData::Solid(voxel) => Some(voxel),
            BrickData::Parted(brick) => {
                for voxel in brick.iter() {
                    if *voxel != brick[0] {
                        return None;
                    }
                }
                Some(&brick[0])
            }
        }
    }

    /// Tries to simplify brick data, returns true if the view was simplified during function call
    pub(crate) fn simplify<V: VoxelData>(
        &mut self,
        color_palette: &[Albedo],
        data_palette: &[V],
    ) -> bool {
        if let Some(homogeneous_type) = self.get_homogeneous_data() {
            if homogeneous_type.points_to_empty(color_palette, data_palette) {
                *self = BrickData::Empty;
            } else {
                *self = BrickData::Solid(homogeneous_type.clone());
            }
            true
        } else {
            false
        }
    }
}

//####################################################################################
//  ██████   █████    ███████    ██████████   ██████████
// ░░██████ ░░███   ███░░░░░███ ░░███░░░░███ ░░███░░░░░█
//  ░███░███ ░███  ███     ░░███ ░███   ░░███ ░███  █ ░
//  ░███░░███░███ ░███      ░███ ░███    ░███ ░██████
//  ░███ ░░██████ ░███      ░███ ░███    ░███ ░███░░█
//  ░███  ░░█████ ░░███     ███  ░███    ███  ░███ ░   █
//  █████  ░░█████ ░░░███████░   ██████████   ██████████
// ░░░░░    ░░░░░    ░░░░░░░    ░░░░░░░░░░   ░░░░░░░░░░
//    █████████     ███████    ██████   █████ ███████████ ██████████ ██████   █████ ███████████
//   ███░░░░░███  ███░░░░░███ ░░██████ ░░███ ░█░░░███░░░█░░███░░░░░█░░██████ ░░███ ░█░░░███░░░█
//  ███     ░░░  ███     ░░███ ░███░███ ░███ ░   ░███  ░  ░███  █ ░  ░███░███ ░███ ░   ░███  ░
// ░███         ░███      ░███ ░███░░███░███     ░███     ░██████    ░███░░███░███     ░███
// ░███         ░███      ░███ ░███ ░░██████     ░███     ░███░░█    ░███ ░░██████     ░███
// ░░███     ███░░███     ███  ░███  ░░█████     ░███     ░███ ░   █ ░███  ░░█████     ░███
//  ░░█████████  ░░░███████░   █████  ░░█████    █████    ██████████ █████  ░░█████    █████
//   ░░░░░░░░░     ░░░░░░░    ░░░░░    ░░░░░    ░░░░░    ░░░░░░░░░░ ░░░░░    ░░░░░    ░░░░░
//####################################################################################

impl NodeContent<VoxelContent> {
    /// Returns with true if it doesn't contain any data
    pub(crate) fn is_empty<V: VoxelData>(
        &self,
        color_palette: &[Albedo],
        data_palette: &[V],
    ) -> bool {
        match self {
            NodeContent::UniformLeaf(brick) => match brick {
                BrickData::Empty => true,
                BrickData::Solid(voxel) => voxel.points_to_empty(color_palette, data_palette),
                BrickData::Parted(brick) => {
                    for voxel in brick.iter() {
                        if !voxel.points_to_empty(color_palette, data_palette) {
                            return false;
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
                            if !voxel.points_to_empty(color_palette, data_palette) {
                                return false;
                            }
                        }
                        BrickData::Parted(brick) => {
                            for voxel in brick.iter() {
                                if !voxel.points_to_empty(color_palette, data_palette) {
                                    return false;
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
    pub(crate) fn is_all(&self, data: &VoxelContent) -> bool {
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

    pub(crate) fn compare(&self, other: &NodeContent<VoxelContent>) -> bool {
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
