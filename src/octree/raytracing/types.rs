use crate::octree::{Cube, V3c};

pub(crate) struct NodeStackItem {
    pub(crate) intersection_exit_distance: f32,
    pub(crate) bounds: Cube,
    pub(crate) node: u32,
    pub(crate) occupied_bits: u8,
    pub(crate) target_octant: u8,
    pub(crate) child_center: V3c<f32>,
}
