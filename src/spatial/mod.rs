pub mod math;
pub mod raytracing;
pub mod tests;

use crate::spatial::math::{offset_region, vector::V3c};

pub(crate) const FLOAT_ERROR_TOLERANCE: f32 = 0.00001;

#[derive(Default, Clone, Copy, Debug)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub(crate) struct Cube {
    pub(crate) min_position: V3c<u32>,
    pub(crate) size: u32,
}

impl Cube {
    pub(crate) fn root_bounds(size: u32) -> Self {
        Self {
            min_position: V3c::unit(0),
            size,
        }
    }

    /// Creates a bounding box within an area described by the min_position and size, for the given octant
    pub(crate) fn child_bounds_for(&self, octant: u32) -> Cube {
        let child_size = self.size / 2;
        Cube {
            min_position: (self.min_position + (offset_region(octant) * child_size)),
            size: child_size,
        }
    }

    pub(crate) fn midpoint(&self) -> V3c<f32> {
        V3c::unit(self.size as f32 / 2.) + self.min_position.into()
    }

    /// True if the given point is inside the cube, with coordinates in inclusive, exclusive range
    /// Edges included
    pub(crate) fn contains_point(&self, point: &V3c<f32>) -> bool {
        (point.x >= self.min_position.x as f32 - FLOAT_ERROR_TOLERANCE)
            && (point.x < (self.min_position.x + self.size) as f32 + FLOAT_ERROR_TOLERANCE)
            && (point.y >= self.min_position.y as f32 - FLOAT_ERROR_TOLERANCE)
            && (point.y < (self.min_position.y + self.size) as f32 + FLOAT_ERROR_TOLERANCE)
            && (point.z >= self.min_position.z as f32 - FLOAT_ERROR_TOLERANCE)
            && (point.z < (self.min_position.z + self.size) as f32 + FLOAT_ERROR_TOLERANCE)
    }
}
