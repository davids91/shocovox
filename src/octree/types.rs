use crate::{object_pool::ObjectPool, octree::BOX_NODE_CHILDREN_COUNT};
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
pub enum BoxTreeEntry<'a, T: VoxelData> {
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
    Leaf([BrickData<T>; BOX_NODE_CHILDREN_COUNT]),

    /// Node has one child, which takes up the entirety of the node with its brick data
    UniformLeaf(BrickData<T>),
}

#[derive(Default, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub(crate) enum NodeChildren<T: Default> {
    #[default]
    NoChildren,
    Children([T; BOX_NODE_CHILDREN_COUNT]),
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
pub type OctreeMIPMapStrategy = HashMap<usize, MIPResamplingMethods>;

/// Implemented methods for MIP sampling. Default is set for
/// all MIP leveles not mentioned in the strategy
#[derive(Debug, Default, Clone, PartialEq)]
pub enum MIPResamplingMethods {
    /// MIP sampled from the MIPs below it, each voxel is the gamma corrected
    /// average of the voxels the cell contains on the same space one level below.
    /// (Gamma is set to be 2.)
    /// Warning: this introduces a significant amount of new colors into the palette
    #[default]
    BoxFilter,

    /// MIP sampled from the MIPs below it, each voxel is chosen from
    /// voxels the cell contains on the same space one level below,
    /// Introduces no new colors as current colors are reused
    PointFilter,

    /// Same as @PointFilter, but the voxels are sampled from
    /// the lowest possible level, instead of MIPs.
    /// It takes the most dominant voxel from the bottom, thus "BD"
    /// --> Bottom Dominant (It's nothing kinky)
    /// On level 1 it behaves like the regular version of itself
    PointFilterBD,

    /// MIP sampled from the MIPs below it, similar voxels are grouped together
    /// the sampled voxel is the average of the largest group
    /// @Albedo color range is assumed(0-255)
    /// f32 parameter is the threshold for similarity with a 0.001 resolution
    Posterize(f32),

    /// Same as @Posterize, but the voxels are sampled from
    /// the lowest possible level, instead of MIPs.
    /// It takes the most dominant voxel from the bottom, thus "BD"
    /// f32 parameter is the threshold for similarity with a 0.001 resolution
    /// On level 1 it behaves like the regular version of itself
    PosterizeBD(f32),
}

/// A helper object for setting Octree MIP map resampling strategy
pub struct StrategyUpdater<'a, T: Default + Clone + Eq + Hash>(pub(crate) &'a mut BoxTree<T>);

/// Configuration object for storing MIP map strategy
/// Don't forget to @recalculate_mip after you've enabled it, as it is
/// only updated on octree updates otherwise.
/// Activating MIP maps will require a larger GPU view (see @OctreeGPUHost::create_new_view)
/// As the MIP bricks will take space from other bricks.
#[derive(Clone)]
pub struct MIPMapStrategy {
    /// Decides if the strategy is enabled, see @Octree/node_mips
    pub(crate) enabled: bool,

    /// The MIP resampling strategy for different MIP levels
    pub(crate) resampling_methods: HashMap<usize, MIPResamplingMethods>,

    /// Color similarity threshold to reduce adding
    /// new colors during MIP operations for each MIP level. Has a resolution of 0.001
    pub(crate) resampling_color_matching_thresholds: HashMap<usize, f32>,
}

/// Sparse 64Tree of Voxel Bricks, where each leaf node contains a brick of voxels.
/// A Brick is a 3 dimensional matrix, each element of it containing a voxel.
/// A Brick can be indexed directly, as opposed to the octree which is essentially a
/// tree-graph where each node has 64 children.
#[cfg_attr(feature = "serialization", derive(Serialize))]
#[derive(Clone)]
pub struct BoxTree<T = u32>
where
    T: Default + Clone + Eq + Hash,
{
    /// Size of one brick in a leaf node (dim^3)
    pub(crate) brick_dim: u32,

    /// Extent of the octree
    pub(crate) boxtree_size: u32,

    /// Storing data at each position through palette index values
    pub(crate) nodes: ObjectPool<NodeData>,

    /// Node Connections
    pub(crate) node_children: Vec<NodeConnection>,

    /// Brick data for each node containing a simplified representation, or all empties if the feature is disabled
    pub(crate) node_mips: Vec<BrickData<PaletteIndexValues>>,

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

    /// Feature flag to enable/disable simplification attempts during octree update operations
    pub auto_simplify: bool,

    /// The stored MIP map strategy
    pub(crate) mip_map_strategy: MIPMapStrategy,
}
