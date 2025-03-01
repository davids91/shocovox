use crate::object_pool::ObjectPool;
use std::{collections::HashMap, error::Error, hash::Hash};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

/// error types during usage or creation of the octree
#[derive(Debug)]
pub enum OctreeError {
    /// Octree creation was attempted with an invalid octree size
    InvalidSize(u32),

    /// Octree creation was attempted with an invalid brick dimension
    InvalidBrickDimension(u32),

    /// Octree creation was attempted with an invalid structure parameter ( refer to error )
    InvalidStructure(Box<dyn Error>),

    /// Octree query was attempted with an invalid position
    InvalidPosition { x: u32, y: u32, z: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OctreeEntry<'a, T: VoxelData> {
    /// No information available in octree query
    Empty,

    /// Albedo data is available in octree query
    Visual(&'a Albedo),

    /// User data is avaliable in octree query
    Informative(&'a T),

    /// Both user data and color information is available in octree query
    Complex(&'a Albedo, &'a T),
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub(crate) enum BrickData<T>
where
    T: Clone + PartialEq + Clone,
{
    /// Brick is empty
    Empty,

    /// Brick is an NxNxN matrix, size is determined by the parent entity
    Parted(Vec<T>),

    /// Brick is a single item T, which takes up the entirety of the brick
    Solid(T),
}

#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub(crate) enum NodeContent<T>
where
    T: Clone + PartialEq + Clone,
{
    /// Node is empty
    #[default]
    Nothing,

    /// Internal node + cache data to store the occupancy of the enclosed nodes
    Internal(u64),

    /// Node contains 8 children, each with their own brickdata
    Leaf([BrickData<T>; 8]),

    /// Node has one child, which takes up the entirety of the node with its brick data
    UniformLeaf(BrickData<T>),
}

#[derive(Default, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub(crate) enum NodeChildren<T: Default> {
    #[default]
    NoChildren,
    Children([T; 8]),
    OccupancyBitmap(u64), // In case of leaf nodes
}

/// Trait for User Defined Voxel Data
pub trait VoxelData {
    /// Determines if the voxel is to be hit by rays in the raytracing algorithms
    fn is_empty(&self) -> bool;
}

/// Color properties of a voxel
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Albedo {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub(crate) type PaletteIndexValues = u32;
pub(crate) type NodeData = NodeContent<PaletteIndexValues>;
pub(crate) type NodeConnection = NodeChildren<u32>;

/// Sparse Octree of Nodes, where each node contains a brick of voxels.
/// A Brick is a 3 dimensional matrix, each element of it containing a voxel.
/// A Brick can be indexed directly, as opposed to the octree which is essentially a
/// tree-graph where each node has 8 children.
/// Generic argument determines the type of the user provided data type
#[cfg_attr(feature = "serialization", derive(Serialize))]
#[derive(Clone)]
pub struct Octree<T = u32>
where
    T: Default + Clone + Eq + Hash,
{
    /// Feature flag to enable/disable simplification attempts during octree update operations
    pub auto_simplify: bool,

    /// Size of one brick in a leaf node (dim^3)
    pub(crate) brick_dim: u32,

    /// Extent of the octree
    pub(crate) octree_size: u32,

    /// Storing data at each position through palette index values
    pub(crate) nodes: ObjectPool<NodeData>,

    /// Node Connections
    pub(crate) node_children: Vec<NodeConnection>,

    /// The albedo colors used by the octree. Maximum 65535 colors can be used at once
    /// because of a limitation on GPU raytracing, to spare space index values refering the palettes
    /// are stored on 2 Bytes
    pub(crate) voxel_color_palette: Vec<Albedo>, // referenced by @nodes
    pub(crate) voxel_data_palette: Vec<T>, // referenced by @nodes

    /// Cache variable to help find colors inside the color palette
    #[cfg_attr(feature = "serialization", serde(skip_serializing, skip_deserializing))]
    pub(crate) map_to_color_index_in_palette: HashMap<Albedo, usize>,

    /// Cache variable to help find user data in the palette
    #[cfg_attr(feature = "serialization", serde(skip_serializing, skip_deserializing))]
    pub(crate) map_to_data_index_in_palette: HashMap<T, usize>,
}
