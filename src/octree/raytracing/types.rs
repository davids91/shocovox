use crate::octree::{Cube, V3c};
use crate::spatial::raytracing::CubeRayIntersection;

pub(crate) struct NodeStackItem {
    pub(crate) bounds_intersection: CubeRayIntersection,
    pub(crate) bounds: Cube,
    pub(crate) node: u32,
    pub(crate) target_octant: u32,
    pub(crate) child_center: V3c<f32>,
}
