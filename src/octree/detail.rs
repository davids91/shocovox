use crate::object_pool::key_none_value;
use crate::octree::types::{NodeChildren, NodeChildrenArray, NodeContent, Octree, VoxelData};
use crate::octree::{hash_region, Cube, V3c};

///####################################################################################
/// Utility functions
///####################################################################################

/// Returns whether the given bound contains the given position.
pub(in crate::octree) fn bound_contains(bounds: &Cube, position: &V3c<u32>) -> bool {
    position.x >= bounds.min_position.x
        && position.x < bounds.min_position.x + bounds.size
        && position.y >= bounds.min_position.y
        && position.y < bounds.min_position.y + bounds.size
        && position.z >= bounds.min_position.z
        && position.z < bounds.min_position.z + bounds.size
}

/// Returns with the octant value(i.e. index) of the child for the given position
pub(in crate::octree) fn child_octant_for(bounds: &Cube, position: &V3c<u32>) -> u32 {
    assert!(bound_contains(bounds, position));
    hash_region(
        &(*position - bounds.min_position).into(),
        bounds.size as f32,
    )
}

///####################################################################################
/// NodeChildrenArray + NodeChildren
///####################################################################################
impl<T> NodeChildren<T>
where
    T: Default + Clone,
{
    pub(in crate::octree) fn is_empty(&self) -> bool {
        matches!(&self.content, NodeChildrenArray::NoChildren)
    }

    pub(in crate::octree) fn new(default_key: T) -> Self {
        Self {
            default_key,
            content: NodeChildrenArray::default(),
        }
    }

    pub(in crate::octree) fn from(default_key: T, children: [T; 8]) -> Self {
        Self {
            default_key,
            content: NodeChildrenArray::Children(children),
        }
    }

    pub(in crate::octree) fn iter(&self) -> Option<std::slice::Iter<T>> {
        match &self.content {
            NodeChildrenArray::Children(c) => Some(c.iter()),
            _ => None,
        }
    }

    pub(in crate::octree) fn set(&mut self, children: [T; 8]) {
        self.content = NodeChildrenArray::Children(children)
    }

    pub(in crate::octree) fn get_full(&self) -> [T; 8] {
        match &self.content {
            NodeChildrenArray::Children(c) => c.clone(),
            _ => [
                self.default_key.clone(),
                self.default_key.clone(),
                self.default_key.clone(),
                self.default_key.clone(),
                self.default_key.clone(),
                self.default_key.clone(),
                self.default_key.clone(),
                self.default_key.clone(),
            ],
        }
    }
}

use std::{
    matches,
    ops::{Index, IndexMut},
};
impl<T> Index<u32> for NodeChildren<T>
where
    T: Default + Copy + Clone,
{
    type Output = T;
    fn index(&self, index: u32) -> &T {
        match &self.content {
            NodeChildrenArray::Children(c) => &c[index as usize],
            _ => &self.default_key,
        }
    }
}

impl<T> IndexMut<u32> for NodeChildren<T>
where
    T: Default + Copy + Clone,
{
    fn index_mut(&mut self, index: u32) -> &mut T {
        if let NodeChildrenArray::NoChildren = &mut self.content {
            self.content = NodeChildrenArray::Children([self.default_key; 8]);
        }
        match &mut self.content {
            NodeChildrenArray::Children(c) => &mut c[index as usize],
            _ => unreachable!(),
        }
    }
}

///####################################################################################
/// NodeContent
///####################################################################################
impl<T, const DIM: usize> NodeContent<T, DIM>
where
    T: PartialEq + Clone + Default,
{
    pub fn is_leaf(&self) -> bool {
        matches!(self, NodeContent::Leaf(_))
    }

    pub fn is_all(&self, data: &T) -> bool {
        match self {
            NodeContent::Leaf(d) => {
                for x in 0..d.len() {
                    for y in 0..d[x].len() {
                        for z in 0..d[x][y].len() {
                            if d[x][y][z] != *data {
                                return false;
                            }
                        }
                    }
                }
                true
            }
            _ => false,
        }
    }

    pub fn data(&self, x: usize, y: usize, z: usize) -> Option<&T> {
        match self {
            NodeContent::Leaf(t) => Some(&t[x][y][z]),
            _ => None,
        }
    }

    pub fn leaf_data(&self) -> &[[[T; DIM]; DIM]; DIM] {
        match self {
            NodeContent::Leaf(t) => t,
            _ => panic!("leaf_data was called for NodeContent<T> where there is no content!"),
        }
    }

    pub fn mut_leaf_data(&mut self) -> &mut [[[T; DIM]; DIM]; DIM] {
        match self {
            NodeContent::Leaf(t) => t,
            _ => panic!("leaf_data was called for NodeContent<T> where there is no content!"),
        }
    }

    pub fn as_leaf_ref(&self) -> Option<&[[[T; DIM]; DIM]; DIM]> {
        match self {
            NodeContent::Leaf(t) => Some(t),
            _ => None,
        }
    }

    pub fn as_mut_leaf_ref(&mut self) -> Option<&mut [[[T; DIM]; DIM]; DIM]> {
        match self {
            NodeContent::Leaf(t) => Some(t),
            _ => None,
        }
    }

    pub fn leaf_from(data: T) -> Self {
        NodeContent::Leaf(array_init::array_init(|_| {
            array_init::array_init(|_| array_init::array_init(|_| data.clone()))
        }))
    }
}

///####################################################################################
/// Octree
///####################################################################################
impl<T, const DIM: usize> Octree<T, DIM>
where
    T: Default + Clone + VoxelData,
{
    /// The root node is always the first item
    pub(crate) const ROOT_NODE_KEY: u32 = 0;

    pub(crate) fn is_size_inadequate(size: u32) -> bool {
        0 == size || (size as f32 / DIM as f32).log(2.0).fract() != 0.0
    }
}

impl<T: Default + PartialEq + Clone + VoxelData, const DIM: usize> Octree<T, DIM> {
    pub(in crate::octree) fn mat_index(bounds: &Cube, position: &V3c<u32>) -> V3c<usize> {
        // --> In case the smallest possible node the contained matrix
        // starts at bounds min_position and ends in min_position + (DIM,DIM,DIM)
        // --> In case of greater Nodes the below ratio equation is relevant
        // mat[xyz]/DIM = (position - min_position) / bounds.size
        let mat_index =
            (V3c::<usize>::from(*position - bounds.min_position) * DIM) / bounds.size as usize;
        // The difference between the actual position and min bounds
        // must not be greater, than DIM at each dimension
        assert!(mat_index.x < DIM);
        assert!(mat_index.y < DIM);
        assert!(mat_index.z < DIM);
        mat_index
    }

    pub(in crate::octree) fn make_uniform_children(
        &mut self,
        content: [[[T; DIM]; DIM]; DIM],
    ) -> [u32; 8] {
        let children = [
            self.nodes.push(NodeContent::Leaf(content.clone())) as u32,
            self.nodes.push(NodeContent::Leaf(content.clone())) as u32,
            self.nodes.push(NodeContent::Leaf(content.clone())) as u32,
            self.nodes.push(NodeContent::Leaf(content.clone())) as u32,
            self.nodes.push(NodeContent::Leaf(content.clone())) as u32,
            self.nodes.push(NodeContent::Leaf(content.clone())) as u32,
            self.nodes.push(NodeContent::Leaf(content.clone())) as u32,
            self.nodes.push(NodeContent::Leaf(content)) as u32,
        ];
        self.node_children
            .resize(self.nodes.len(), NodeChildren::new(key_none_value()));
        children
    }

    pub(in crate::octree) fn deallocate_children_of(&mut self, node: u32) {
        let mut to_deallocate = Vec::new();
        if let Some(children) = self.node_children[node as usize].iter() {
            for child in children {
                if crate::object_pool::key_might_be_valid(*child) {
                    to_deallocate.push(*child);
                }
            }
            for child in to_deallocate {
                self.deallocate_children_of(child); // Recursion should be fine as depth is not expceted to be more, than 32
                self.nodes.free(child as usize);
            }
        }
        self.node_children[node as usize].content = NodeChildrenArray::NoChildren;
    }

    /// Updates the given node recursively to collapse nodes with uniform children into a leaf
    pub(in crate::octree) fn simplify(&mut self, node: u32) -> bool {
        let mut data = NodeContent::Nothing;
        if crate::object_pool::key_might_be_valid(node) {
            for i in 0..8 {
                let child_key = self.node_children[node as usize][i];
                if crate::object_pool::key_might_be_valid(child_key) {
                    if let Some(leaf_data) = self.nodes.get(child_key as usize).as_leaf_ref() {
                        if !data.is_leaf() {
                            data = NodeContent::Leaf(leaf_data.clone());
                        } else if data.leaf_data() != leaf_data {
                            return false;
                        }
                    } else {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            *self.nodes.get_mut(node as usize) = data;
            self.deallocate_children_of(node); // no need to use this as all the children are leaves, but it's more understanfdable this way
            true
        } else {
            false
        }
    }

    /// Count the number of children a Node has according to the stored cache of the children
    pub(in crate::octree) fn count_cached_children(&self, node: u32) -> u32 {
        let mut actual_count = 0;
        for i in 0..8 {
            let child_key = self.node_children[node as usize][i];
            if crate::object_pool::key_might_be_valid(child_key) {
                match self.nodes.get(child_key as usize) {
                    NodeContent::Leaf(_) => {
                        actual_count += (DIM as u32).pow(3);
                    }
                    NodeContent::Internal(c) => {
                        actual_count += c;
                    }
                    _ => {}
                }
            }
        }
        actual_count
    }
}
