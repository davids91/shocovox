/// As in: Look-up Tables
pub mod lut;

pub mod math;

#[cfg(feature = "raytracing")]
pub mod raytracing;

mod tests;

use crate::{
    octree::BOX_NODE_DIMENSION, spatial::lut::SECTANT_OFFSET_LUT, spatial::math::vector::V3c,
};

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
    pub(crate) fn root_bounds(size: f32) -> Self {
        Self {
            min_position: V3c::unit(0.),
            size,
        }
    }

    /// Creates a bounding box within an area described by the min_position and size, for the given sectant
    pub(crate) fn child_bounds_for(&self, sectant: u8) -> Cube {
        Cube {
            min_position: (self.min_position + (SECTANT_OFFSET_LUT[sectant as usize] * self.size)),
            size: self.size / BOX_NODE_DIMENSION as f32,
        }
    }
}
