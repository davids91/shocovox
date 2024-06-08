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
    OccupancyBitmap(u64), // In case of leaf nodes
}

#[derive(Debug, Copy, Clone)]
#[cfg_attr(test, derive(PartialEq))]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub(in crate::octree) struct NodeChildren<T: Default> {
    /// The key value to signify "no child" at a given slot
    pub(in crate::octree) empty_marker: T,

    /// The contained child key values
    pub(in crate::octree) content: NodeChildrenArray<T>,
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

impl VoxelData for Albedo {
    fn new(color: Albedo, _user_data: u32) -> Self {
        color
    }

    fn albedo(&self) -> Albedo {
        self.clone()
    }

    fn user_data(&self) -> u32 {
        0u32.into()
    }

    fn clear(&mut self) {
        self.r = 0;
        self.r = 0;
        self.b = 0;
        self.a = 0;
    }
}

impl From<u32> for Albedo {
    fn from(value: u32) -> Self {
        let a = (value & 0x000000FF) as u8;
        let b = ((value & 0x0000FF00) >> 8) as u8;
        let g = ((value & 0x00FF0000) >> 16) as u8;
        let r = ((value & 0xFF000000) >> 24) as u8;

        Albedo::default()
            .with_red(r)
            .with_green(g)
            .with_blue(b)
            .with_alpha(a)
    }
}

#[cfg_attr(feature = "serialization", derive(Serialize))]
pub struct Octree<T: Default + Clone + VoxelData, const DIM: usize = 1> {
    pub auto_simplify: bool,
    pub(in crate::octree) octree_size: u32,
    pub(in crate::octree) nodes: ObjectPool<NodeContent<T, DIM>>,
    pub(in crate::octree) node_children: Vec<NodeChildren<u32>>, // Children index values of each Node
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Albedo {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Default for Albedo {
    fn default() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }
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

#[test]
fn albedo_size_is_4_bytes() {
    const SIZE: usize = std::mem::size_of::<Albedo>();
    const EXPECTED_SIZE: usize = 4;
    assert_eq!(
        SIZE, EXPECTED_SIZE,
        "RGBA should be {} bytes wide but was {}",
        EXPECTED_SIZE, SIZE
    );
}
