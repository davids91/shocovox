/// As in: Look-up Tables
pub mod lut;

pub mod math;

#[cfg(feature = "raytracing")]
pub mod raytracing;

mod tests;

use crate::{
    octree::BOX_NODE_DIMENSION, spatial::lut::SECTANT_OFFSET_LUT, spatial::math::vector::V3c,
};

/// Provides update scope within the given bounds
/// * `bounds` - The confines of the update
/// * `position` - the starting position of the update
/// * `update_size` - the size of the update, guaranteed to be at least 1
pub(crate) fn update_size_within(bounds: &Cube, position: &V3c<u32>, update_size: u32) -> u32 {
    debug_assert!(
        bounds.contains(&((*position).into())),
        "Expected position {:?} to be inside bounds {:?}",
        position,
        bounds
    );
    let update_scope_within = bounds.min_position + V3c::unit(bounds.size) - V3c::from(*position);
    (update_scope_within
        .x
        .min(update_scope_within.y)
        .min(update_scope_within.z) as u32)
        .min(update_size)
        .max(1)
}

#[derive(Default, Clone, Copy, Debug)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub(crate) struct Cube {
    pub(crate) min_position: V3c<f32>,
    pub(crate) size: f32,
}

impl Cube {
    /// Creates boundaries starting from (0,0,0), with the given size
    pub(crate) fn root_bounds(size: f32) -> Self {
        Self {
            min_position: V3c::unit(0.),
            size,
        }
    }

    pub(crate) fn contains(&self, position: &V3c<f32>) -> bool {
        position.x >= self.min_position.x
            && position.y >= self.min_position.y
            && position.z >= self.min_position.z
            && position.x < (self.min_position.x + self.size)
            && position.y < (self.min_position.y + self.size)
            && position.z < (self.min_position.z + self.size)
    }

    /// Creates a bounding box within an area described by the min_position and size, for the given sectant
    pub(crate) fn child_bounds_for(&self, sectant: u8) -> Cube {
        Cube {
            min_position: (self.min_position + (SECTANT_OFFSET_LUT[sectant as usize] * self.size)),
            size: self.size / BOX_NODE_DIMENSION as f32,
        }
    }
}
