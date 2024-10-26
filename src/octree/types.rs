use crate::object_pool::ObjectPool;
use std::error::Error;

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub(crate) enum BrickData<T, const DIM: usize>
where
    T: Clone + PartialEq + Clone + VoxelData,
{
    Empty,
    Parted(Box<[[[T; DIM]; DIM]; DIM]>),
    Solid(T),
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub(crate) enum NodeContent<T, const DIM: usize>
where
    T: Clone + PartialEq + Clone + VoxelData,
{
    #[default]
    Nothing,
    Internal(u64), // cache data to store the occupancy of the enclosed nodes
    Leaf([BrickData<T, DIM>; 8]),
    UniformLeaf(BrickData<T, DIM>),
}

/// error types during usage or creation of the octree
#[derive(Debug)]
pub enum OctreeError {
    InvalidSize(u32),
    InvalidBrickDimension(u32),
    InvalidStructure(Box<dyn Error>),
    InvalidPosition { x: u32, y: u32, z: u32 },
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
#[cfg_attr(test, derive(Eq))]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub(crate) enum NodeChildrenArray<T: Default> {
    #[default]
    NoChildren,
    Children([T; 8]),
    OccupancyBitmap(u64), // In case of leaf nodes
}

#[derive(Debug, Copy, Clone)]
#[cfg_attr(test, derive(PartialEq, Eq))]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub(crate) struct NodeChildren<T: Default> {
    /// The key value to signify "no child" at a given slot
    pub(crate) empty_marker: T,

    /// The contained child key values
    pub(crate) content: NodeChildrenArray<T>,
}

pub trait VoxelData {
    fn new(color: Albedo, user_data: u32) -> Self;
    /// The color to display during raytracing 0-255 RGBA
    fn albedo(&self) -> Albedo;
    /// User defined data
    fn user_data(&self) -> u32;
    /// Determines if the voxel is to be hit by rays in the raytracing algorithms
    fn is_empty(&self) -> bool {
        self.albedo().is_transparent() && self.user_data() == 0
    }
    /// Implementation to clear the contained data, as well as albedo
    fn clear(&mut self);
}

/// Sparse Octree of Nodes, where each node contains a brick of voxels.
/// A Brick is a 3 dimensional matrix, each element of it containing a voxel.
/// A Brick can be indexed directly, as opposed to the octree which is essentially a
/// tree-graph where each node has 8 children.
#[cfg_attr(feature = "serialization", derive(Serialize))]
pub struct Octree<T, const DIM: usize = 1>
where
    T: Default + Clone + PartialEq + VoxelData,
{
    pub auto_simplify: bool,
    pub(crate) octree_size: u32,
    pub(crate) nodes: ObjectPool<NodeContent<T, DIM>>,
    pub(crate) node_children: Vec<NodeChildren<u32>>, // Children index values of each Node
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Albedo {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Albedo {
    pub fn with_red(mut self, r: u8) -> Self {
        self.r = r;
        self
    }

    pub fn with_green(mut self, g: u8) -> Self {
        self.g = g;
        self
    }

    pub fn with_blue(mut self, b: u8) -> Self {
        self.b = b;
        self
    }

    pub fn with_alpha(mut self, a: u8) -> Self {
        self.a = a;
        self
    }

    pub fn is_transparent(&self) -> bool {
        self.a == 0
    }
}
