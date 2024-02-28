use crate::object_pool::ObjectPool;

#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Octree<T>
where
    T: Default + Clone + VoxelData,
{
    pub auto_simplify: bool,
    pub(in crate::octree) root_size: u32,
    pub(in crate::octree) nodes: ObjectPool<NodeContent<T>>, //None means the Node is an internal node, Some(...) means the Node is a leaf
    pub(in crate::octree) node_children: Vec<NodeChildren<u32>>, // Children index values of each Node
}

#[derive(Default)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
#[derive(Clone)]
pub(crate) enum NodeContent<T>
where
    T: Clone,
{
    #[default]
    Nothing,
    Internal(u32), // cache data to store the enclosed nodes
    Leaf(T),
}

/// error types during usage or creation of the octree
#[derive(Debug)]
pub enum OctreeError {
    InvalidNodeSize(u32),
    InvalidPosition { x: u32, y: u32, z: u32 },
}

#[derive(Debug, Default, Copy, Clone)]
pub(in crate::octree) enum NodeChildrenArray<T: Default> {
    #[default]
    NoChildren,
    Children([T; 8]),
}

#[derive(Debug, Copy, Clone)]
pub(in crate::octree) struct NodeChildren<T: Default> {
    pub(in crate::octree) default_key: T,
    pub(in crate::octree) content: NodeChildrenArray<T>,
}

pub trait VoxelData {
    fn new(r: u8, g: u8, b: u8, user_data: Option<u32>) -> Self;
    fn albedo(&self) -> [u8; 3]; // 0-255 RGB
    fn user_data(&self) -> Option<u32>;
}

impl VoxelData for u32 {
    fn new(r: u8, g: u8, b: u8, _user_data: Option<u32>) -> Self {
        r as u32 & 0x000000FF | ((g as u32 & 0x000000FF) << 8) | ((b as u32 & 0x000000FF) << 16)
    }
    fn albedo(&self) -> [u8; 3] {
        [
            (self & 0x000000FF) as u8,
            ((self & 0x0000FF00) >> 8) as u8,
            ((self & 0x00FF0000) >> 16) as u8,
        ]
    }
    fn user_data(&self) -> Option<u32> {
        None
    }
}