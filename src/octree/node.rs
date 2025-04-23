use crate::octree::{
    empty_marker,
    types::{
        Albedo, BrickData, NodeChildren, NodeConnection, NodeContent, PaletteIndexValues, VoxelData,
    },
    BoxTreeEntry, V3c, BOX_NODE_CHILDREN_COUNT,
};
use crate::spatial::math::{flat_projection, set_occupied_bitmap_value};
use std::{
    fmt::{Debug, Error, Formatter},
    matches,
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
impl<T: Default + Debug> Debug for NodeChildren<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match &self {
            NodeChildren::NoChildren => write!(f, "NodeChildren::NoChildren"),
            NodeChildren::Children(array) => {
                write!(f, "NodeChildren::Children({:?})", array)
            }
            NodeChildren::OccupancyBitmap(mask) => {
                write!(f, "NodeChildren::OccupancyBitmap({:#10X})", mask)
            }
        }
    }
}
impl NodeConnection {
    pub(crate) fn child(&self, sectant: u8) -> usize {
        match &self {
            NodeChildren::Children(c) => c[sectant as usize] as usize,
            _ => empty_marker(),
        }
    }

    pub(crate) fn child_mut(&mut self, index: usize) -> Option<&mut u32> {
        if let NodeChildren::NoChildren = self {
            *self = NodeChildren::Children([empty_marker(); BOX_NODE_CHILDREN_COUNT]);
        }
        match self {
            NodeChildren::Children(c) => Some(&mut c[index]),
            _ => panic!("Attempted to modify NodeChild[{:?}] of {:?}", index, self),
        }
    }

    /// Provides a slice for iteration, if there are children to iterate on
    pub(crate) fn iter(&self) -> Option<std::slice::Iter<u32>> {
        match &self {
            NodeChildren::Children(c) => Some(c.iter()),
            _ => None,
        }
    }

    /// Erases content, if any
    pub(crate) fn clear(&mut self, child_index: usize) {
        debug_assert!(child_index < 8);
        if let NodeChildren::Children(c) = self {
            c[child_index] = empty_marker();
            if 8 == c.iter().filter(|e| **e == empty_marker::<u32>()).count() {
                *self = NodeChildren::NoChildren;
            }
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
impl BrickData<PaletteIndexValues> {
    /// Calculates the Occupancy bitmap for the given Voxel brick
    pub(crate) fn calculate_brick_occupied_bits<V: VoxelData>(
        brick: &[PaletteIndexValues],
        brick_dimension: usize,
        color_palette: &[Albedo],
        data_palette: &[V],
    ) -> u64 {
        let mut bitmap = 0;
        for x in 0..brick_dimension {
            for y in 0..brick_dimension {
                for z in 0..brick_dimension {
                    let flat_index = flat_projection(x, y, z, brick_dimension);
                    if !NodeContent::pix_points_to_empty(
                        &brick[flat_index],
                        color_palette,
                        data_palette,
                    ) {
                        set_occupied_bitmap_value(
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
                if NodeContent::pix_points_to_empty(voxel, color_palette, data_palette) {
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
    pub(crate) fn get_homogeneous_data(&self) -> Option<&PaletteIndexValues> {
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

    pub(crate) fn contains_nothing<V: VoxelData>(
        &self,
        color_palette: &[Albedo],
        data_palette: &[V],
    ) -> bool {
        match self {
            BrickData::Empty => true,
            BrickData::Solid(voxel) => {
                NodeContent::pix_points_to_empty(voxel, color_palette, data_palette)
            }
            BrickData::Parted(brick) => {
                for voxel in brick.iter() {
                    if !NodeContent::pix_points_to_empty(voxel, color_palette, data_palette) {
                        return false;
                    }
                }
                true
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
            if NodeContent::pix_points_to_empty(homogeneous_type, color_palette, data_palette) {
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

impl NodeContent<PaletteIndexValues> {
    pub(crate) fn pix_visual(color_index: u16) -> PaletteIndexValues {
        (color_index as u32) | ((empty_marker::<u16>() as u32) << 16)
    }

    pub(crate) fn pix_informal(data_index: u16) -> PaletteIndexValues {
        (empty_marker::<u16>() as u32) | ((data_index as u32) << 16)
    }

    pub(crate) fn pix_complex(color_index: u16, data_index: u16) -> PaletteIndexValues {
        (color_index as u32) | ((data_index as u32) << 16)
    }

    pub(crate) fn pix_color_index(index: &PaletteIndexValues) -> usize {
        (index & 0x0000FFFF) as usize
    }
    pub(crate) fn pix_data_index(index: &PaletteIndexValues) -> usize {
        ((index & 0xFFFF0000) >> 16) as usize
    }

    pub(crate) fn pix_overwrite_color(
        mut index: PaletteIndexValues,
        delta: &PaletteIndexValues,
    ) -> PaletteIndexValues {
        index = (index & 0xFFFF0000) | (delta & 0x0000FFFF);
        index
    }

    pub(crate) fn pix_overwrite_data(
        mut index: PaletteIndexValues,
        delta: &PaletteIndexValues,
    ) -> PaletteIndexValues {
        index = (index & 0x0000FFFF) | (delta & 0xFFFF0000);
        index
    }

    pub(crate) fn pix_color_is_some(index: &PaletteIndexValues) -> bool {
        Self::pix_color_index(index) < empty_marker::<u16>() as usize
    }

    pub(crate) fn pix_color_is_none(index: &PaletteIndexValues) -> bool {
        !Self::pix_color_is_some(index)
    }

    pub(crate) fn pix_data_is_none(index: &PaletteIndexValues) -> bool {
        Self::pix_data_index(index) == empty_marker::<u16>() as usize
    }

    pub(crate) fn pix_data_is_some(index: &PaletteIndexValues) -> bool {
        !Self::pix_data_is_none(index)
    }

    pub(crate) fn pix_points_to_empty<V: VoxelData>(
        index: &PaletteIndexValues,
        color_palette: &[Albedo],
        data_palette: &[V],
    ) -> bool {
        debug_assert!(
            Self::pix_color_index(index) < color_palette.len() || Self::pix_color_is_none(index),
            "Expected color index to be inside bounds: {} <> {}",
            Self::pix_color_index(index),
            color_palette.len()
        );
        debug_assert!(
            Self::pix_data_index(index) < data_palette.len() || Self::pix_data_is_none(index),
            "Expected data
             index to be inside bounds: {} <> {}",
            Self::pix_data_index(index),
            color_palette.len()
        );
        (Self::pix_color_is_none(index)
            || color_palette[Self::pix_color_index(index)].is_transparent())
            && (Self::pix_data_is_none(index)
                || data_palette[Self::pix_data_index(index)].is_empty())
    }

    pub(crate) fn pix_get_ref<'a, V: VoxelData>(
        index: &PaletteIndexValues,
        color_palette: &'a [Albedo],
        data_palette: &'a [V],
    ) -> BoxTreeEntry<'a, V> {
        if Self::pix_data_is_none(index) && Self::pix_color_is_none(index) {
            return BoxTreeEntry::Empty;
        }
        if Self::pix_data_is_none(index) {
            debug_assert!(Self::pix_color_is_some(index));
            debug_assert!(Self::pix_color_index(index) < color_palette.len());
            return BoxTreeEntry::Visual(&color_palette[Self::pix_color_index(index)]);
        }

        if Self::pix_color_is_none(index) {
            debug_assert!(Self::pix_data_is_some(index));
            debug_assert!(Self::pix_data_index(index) < data_palette.len());
            return BoxTreeEntry::Informative(&data_palette[Self::pix_data_index(index)]);
        }

        debug_assert!(
            Self::pix_color_index(index) < color_palette.len(),
            "Expected data
             index to be inside bounds: {} <> {}",
            Self::pix_color_index(index),
            color_palette.len()
        );
        debug_assert!(
            Self::pix_data_index(index) < data_palette.len(),
            "Expected data
             index to be inside bounds: {} <> {}",
            Self::pix_data_index(index),
            data_palette.len()
        );
        BoxTreeEntry::Complex(
            &color_palette[Self::pix_color_index(index)],
            &data_palette[Self::pix_data_index(index)],
        )
    }

    /// Returns with true if content doesn't have any data
    pub(crate) fn is_empty<V: VoxelData>(
        &self,
        color_palette: &[Albedo],
        data_palette: &[V],
    ) -> bool {
        match self {
            NodeContent::UniformLeaf(brick) => match brick {
                BrickData::Empty => true,
                BrickData::Solid(voxel) => {
                    Self::pix_points_to_empty(voxel, color_palette, data_palette)
                }
                BrickData::Parted(brick) => {
                    for voxel in brick.iter() {
                        if !Self::pix_points_to_empty(voxel, color_palette, data_palette) {
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
                            if !Self::pix_points_to_empty(voxel, color_palette, data_palette) {
                                return false;
                            }
                        }
                        BrickData::Parted(brick) => {
                            for voxel in brick.iter() {
                                if !Self::pix_points_to_empty(voxel, color_palette, data_palette) {
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
    pub(crate) fn is_all(&self, data: &PaletteIndexValues) -> bool {
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

    pub(crate) fn compare(&self, other: &NodeContent<PaletteIndexValues>) -> bool {
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
