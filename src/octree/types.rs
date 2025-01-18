use crate::object_pool::ObjectPool;
use std::{collections::HashMap, error::Error, hash::Hash};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

/// error types during usage or creation of the octree
#[derive(Debug)]
pub enum OctreeError {
    InvalidSize(u32),
    InvalidBrickDimension(u32),
    InvalidStructure(Box<dyn Error>),
    InvalidPosition { x: u32, y: u32, z: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OctreeEntry<'a, T: VoxelData> {
    Empty,
    Visual(&'a Albedo),
    Informative(&'a T),
    Complex(&'a Albedo, &'a T),
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub(crate) enum BrickData<T>
where
    T: Clone + PartialEq + Clone,
{
    Empty,
    Parted(Vec<T>),
    Solid(T),
}

#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub(crate) enum NodeContent<T>
where
    T: Clone + PartialEq + Clone,
{
    #[default]
    Nothing,
    Internal(u64), // cache data to store the occupancy of the enclosed nodes
    Leaf([BrickData<T>; 8]),
    UniformLeaf(BrickData<T>),
}

#[derive(Default, Copy, Clone, PartialEq)]
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

/// Trait for User Defined Voxel Data
pub trait VoxelData {
    /// Determines if the voxel is to be hit by rays in the raytracing algorithms
    fn is_empty(&self) -> bool;
}

/// Index values in data and color palettes
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(crate) struct VoxelContent {
    pub(crate) color_index: u16,
    pub(crate) data_index: u16,
}

/// Color properties of a voxel
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Albedo {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Sparse Octree of Nodes, where each node contains a brick of voxels.
/// A Brick is a 3 dimensional matrix, each element of it containing a voxel.
/// A Brick can be indexed directly, as opposed to the octree which is essentially a
/// tree-graph where each node has 8 children.
/// Generic argument determines the type of the user provided data type
#[cfg_attr(feature = "serialization", derive(Serialize))]
#[derive(Clone)]
pub struct Octree<T = u32>
where
    T: Default + Clone + PartialEq + Hash,
{
    pub auto_simplify: bool,
    pub(crate) brick_dim: u32,   // Size of one brick in a leaf node (dim^3)
    pub(crate) octree_size: u32, // Extent of the octree
    pub(crate) nodes: ObjectPool<NodeContent<VoxelContent>>, // Storing data at each position through palette index values
    pub(crate) node_children: Vec<NodeChildren<u32>>,        // Node Connections
    pub(crate) voxel_color_palette: Vec<Albedo>,             // referenced by @nodes
    pub(crate) voxel_data_palette: Vec<T>,                   // referenced by @nodes
    pub(crate) map_to_color_index_in_palette: HashMap<Albedo, usize>,
    pub(crate) map_to_data_index_in_palette: HashMap<T, usize>,
}
