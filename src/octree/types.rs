use crate::object_pool::ObjectPool;

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Default, Clone)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub(crate) enum NodeContent<T: Clone, const DIM: usize = 1> {
    #[default]
    Nothing,
    Internal(u8), // cache data to store the enclosed nodes
    Leaf([[[T; DIM]; DIM]; DIM]),
}

/// error types during usage or creation of the octree
#[derive(Debug)]
pub enum OctreeError {
    InvalidNodeSize(u32),
    InvalidBrickDimension(u32),
    InvalidPosition { x: u32, y: u32, z: u32 },
}

#[derive(Debug, Default, Copy, Clone)]
#[cfg_attr(test, derive(PartialEq))]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub(in crate::octree) enum NodeChildrenArray<T: Default> {
    #[default]
    NoChildren,
    Children([T; 8]),
    OccupancyBitmask(u64), // In case of leaf nodes
}

#[derive(Debug, Copy, Clone)]
#[cfg_attr(test, derive(PartialEq))]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub(in crate::octree) struct NodeChildren<T: Default> {
    /// The key value to signify "no child" at a given slot
    pub(in crate::octree) default_key: T,

    /// The contained child key values
    pub(in crate::octree) content: NodeChildrenArray<T>,
}

pub trait VoxelData {
    fn new(r: u8, g: u8, b: u8, a: u8, user_data: u32) -> Self;
    /// The color to display during raytracing 0-255 RGBA
    fn albedo(&self) -> [u8; 4];
    /// User defined data
    fn user_data(&self) -> u32;
    /// determines if the voxel is to be hit by rays in the raytracing algorithms
    fn is_empty(&self) -> bool {
        [0, 0, 0, 0] == self.albedo() && 0 == self.user_data()
    }
    /// Implementation to clear the contained data, as well as albedo
    fn clear(&mut self);
}

impl VoxelData for u32 {
    fn new(r: u8, g: u8, b: u8, a: u8, _user_data: u32) -> Self {
        r as u32 & 0x000000FF
            | ((g as u32 & 0x000000FF) << 8)
            | ((b as u32 & 0x000000FF) << 16)
            | ((a as u32 & 0x000000FF) << 24)
    }
    fn albedo(&self) -> [u8; 4] {
        [
            (self & 0x000000FF) as u8,
            ((self & 0x0000FF00) >> 8) as u8,
            ((self & 0x00FF0000) >> 16) as u8,
            ((self & 0xFF000000) >> 24) as u8,
        ]
    }
    fn user_data(&self) -> u32 {
        0
    }
    fn clear(&mut self) {
        *self = 0;
    }
}

#[cfg_attr(feature = "serialization", derive(Serialize))]
pub struct Octree<T: Default + Clone + VoxelData, const DIM: usize = 1> {
    pub auto_simplify: bool,
    pub(in crate::octree) octree_size: u32,
    pub(in crate::octree) nodes: ObjectPool<NodeContent<T, DIM>>,
    pub(in crate::octree) node_children: Vec<NodeChildren<u32>>, // Children index values of each Node
}
