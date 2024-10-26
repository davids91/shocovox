pub mod lut;
pub mod math;

#[cfg(feature = "raytracing")]
pub mod raytracing;

mod tests;

use crate::spatial::math::{offset_region, vector::V3c};

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

    /// Creates a bounding box within an area described by the min_position and size, for the given octant
    pub(crate) fn child_bounds_for(&self, octant: u8) -> Cube {
        let child_size = self.size / 2.;
        Cube {
            min_position: (self.min_position + (offset_region(octant) * child_size)),
            size: child_size,
        }
    }
}
