use crate::octree::{Cube, V3c};
use crate::spatial::raytracing::CubeHit;

pub(crate) struct NodeStackItem {
    pub(crate) bounds_intersection: CubeHit,
    pub(crate) bounds: Cube,
    pub(crate) node: u32,
    pub(crate) target_octant: u32,
    pub(crate) child_center: V3c<f32>,
}
