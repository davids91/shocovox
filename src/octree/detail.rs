use crate::object_pool::key_none_value;
use crate::octree::types::{NodeChildren, NodeChildrenArray, NodeContent, Octree, VoxelData};
use crate::octree::{hash_region, Cube, V3c};
use crate::spatial::math::octant_bitmask;

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
pub(in crate::octree) fn child_octant_for(bounds: &Cube, position: &V3c<u32>) -> u8 {
    debug_assert!(bound_contains(bounds, position));
    hash_region(
        &(*position - bounds.min_position).into(),
        bounds.size as f32,
    )
}

///####################################################################################
/// NodeChildren
///####################################################################################
impl<T> NodeChildren<T>
where
    T: Default + Clone + PartialEq,
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

    pub(in crate::octree) fn clear(&mut self, child_index: usize) {
        debug_assert!(child_index < 8);
        if let NodeChildrenArray::Children(c) = &mut self.content {
            c[child_index] = self.default_key.clone();
            if 8 == c.iter().filter(|e| **e == self.default_key).count() {
                self.content = NodeChildrenArray::NoChildren;
            }
        }
    }

    pub(in crate::octree) fn set(&mut self, children: [T; 8]) {
        self.content = NodeChildrenArray::Children(children)
    }

    fn occupied_bits(&self) -> u8 {
        match &self.content {
            NodeChildrenArray::Children(c) => {
                let mut result = 0;
                for (child_octant, child) in c.iter().enumerate().take(8) {
                    if *child != self.default_key {
                        result |= octant_bitmask(child_octant as u8);
                    }
                }
                result
            }
            _ => 0,
        }
    }

    #[cfg(feature = "bevy_wgpu")]
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
    T: VoxelData + PartialEq + Clone + Default,
{
    pub fn is_leaf(&self) -> bool {
        matches!(self, NodeContent::Leaf(_))
    }

    pub fn is_empty(&self) -> bool {
        match self {
            NodeContent::Leaf(d) => {
                for x in d.iter() {
                    for y in x.iter() {
                        for item in y.iter() {
                            if !item.is_empty() {
                                return false;
                            }
                        }
                    }
                }
                true
            }
            NodeContent::Nothing => true,
            NodeContent::Internal(_) => false,
        }
    }

    pub fn is_all(&self, data: &T) -> bool {
        match self {
            NodeContent::Leaf(d) => {
                for x in d.iter() {
                    for y in x.iter() {
                        for item in y.iter() {
                            if *item != *data {
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
        debug_assert!(mat_index.x < DIM);
        debug_assert!(mat_index.y < DIM);
        debug_assert!(mat_index.z < DIM);
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
                if self.nodes.key_is_valid(*child as usize) {
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
        if self.nodes.key_is_valid(node as usize) {
            match self.nodes.get(node as usize) {
                NodeContent::Leaf(_) | NodeContent::Nothing => {
                    return true;
                }
                _ => {}
            }
            for i in 0..8 {
                let child_key = self.node_children[node as usize][i];
                if self.nodes.key_is_valid(child_key as usize) {
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

    /// Calculates the occupied bits of a Node; For empty nodes(Nodecontent::Nothing) as well;
    /// As they might be empty by fault and to correct them the occupied bits is required.
    /// Leaf nodes are all oocupied by default
    pub(in crate::octree) fn occupied_bits(&self, node: u32) -> u8 {
        match self.nodes.get(node as usize) {
            NodeContent::Leaf(_) => 0xFF,
            _ => self.node_children[node as usize].occupied_bits(),
        }
    }
}
