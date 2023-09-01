pub mod math;
pub mod raytracing;

use crate::spatial::math::offset_region;
use crate::spatial::math::V3c;

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
    pub(crate) fn child_bounds_for(&self, octant: usize) -> Cube {
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
        let edges_epsilon = 0.00001;
        (point.x >= self.min_position.x as f32 - edges_epsilon)
            && (point.x < (self.min_position.x + self.size) as f32 + edges_epsilon)
            && (point.y >= self.min_position.y as f32 - edges_epsilon)
            && (point.y < (self.min_position.y + self.size) as f32 + edges_epsilon)
            && (point.z >= self.min_position.z as f32 - edges_epsilon)
            && (point.z < (self.min_position.z + self.size) as f32 + edges_epsilon)
    }

    /// Tells if the given point is iniside the points, coordinates in exclusive exclusive range,
    /// Edges excluded
    pub(crate) fn includes_point(&self, point: &V3c<f32>) -> bool {
        (point.x >= self.min_position.x as f32)
            && (point.x < (self.min_position.x + self.size) as f32)
            && (point.y > self.min_position.y as f32)
            && (point.y < (self.min_position.y + self.size) as f32)
            && (point.z > self.min_position.z as f32)
            && (point.z < (self.min_position.z + self.size) as f32)
    }
}
