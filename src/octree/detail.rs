#[cfg(feature = "raytracing")]
use crate::object_pool::{key_might_be_valid, key_none_value};
use crate::octree::{hash_region, Cube, Octree, V3c};
use bendy::{decoding::FromBencode, encoding::ToBencode};

#[cfg(feature = "serialization")]
use serde::{de::DeserializeOwned, Deserialize, Serialize};

///####################################################################################
/// Utility functions
///####################################################################################

/// Returns whether the given bound contains the given position.
pub(in crate::octree) fn bound_contains(bounds: &Cube, position: &V3c<u32>) -> bool {
    position.x >= bounds.min_position.x
        && position.x <= bounds.min_position.x + bounds.size
        && position.y >= bounds.min_position.y
        && position.y <= bounds.min_position.y + bounds.size
        && position.z >= bounds.min_position.z
        && position.z <= bounds.min_position.z + bounds.size
}

/// Returns with the octant value(i.e. index) of the child for the given position
pub(in crate::octree) fn child_octant_for(bounds: &Cube, position: &V3c<u32>) -> usize {
    assert!(bound_contains(bounds, position));
    hash_region(
        &(*position - bounds.min_position).into(),
        bounds.size as f32,
    )
}

///####################################################################################
/// NodeChildrenArray + NodeChildren
///####################################################################################
#[derive(Debug)]
#[derive(Default, Copy, Clone)]
pub(in crate::octree) enum NodeChildrenArray<T: Default> {
    #[default]
    NoChildren,
    Children([T; 8]),
}

#[derive(Debug)]
#[derive(Copy, Clone)]
pub(in crate::octree) struct NodeChildren<T: Default> {
    default_key: T,
    pub(in crate::octree) content: NodeChildrenArray<T>,
}

impl<T> NodeChildren<T>
where
    T: Default + Clone,
{
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
}

use std::ops::{Index, IndexMut};
impl<T> Index<usize> for NodeChildren<T>
where
    T: Default + Copy + Clone,
{
    type Output = T;
    fn index(&self, index: usize) -> &T {
        match &self.content {
            NodeChildrenArray::Children(c) => &c[index],
            _ => &self.default_key,
        }
    }
}

impl<T> IndexMut<usize> for NodeChildren<T>
where
    T: Default + Copy + Clone,
{
    fn index_mut(&mut self, index: usize) -> &mut T {
        if let NodeChildrenArray::NoChildren = &mut self.content {
            self.content = NodeChildrenArray::Children([self.default_key; 8]);
        }
        match &mut self.content {
            NodeChildrenArray::Children(c) => &mut c[index],
            _ => unreachable!(),
        }
    }
}

///####################################################################################
/// NodeContent
///####################################################################################
#[derive(Default)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
#[derive(Clone)]
pub(in crate::octree) enum NodeContent<T>
where
    T: Clone,
{
    #[default]
    Nothing,
    Leaf(T),
}

impl<T> NodeContent<T>
where
    T: Clone,
{
    pub fn is_leaf(&self) -> bool {
        match self {
            NodeContent::Leaf(_) => true,
            _ => false,
        }
    }

    pub fn leaf_data(&self) -> &T {
        match self {
            NodeContent::Leaf(t) => &t,
            _ => panic!("leaf_data was called for NodeContent<T> where there is no content!"),
        }
    }

    pub fn as_leaf_ref(&self) -> Option<&T> {
        match self {
            NodeContent::Leaf(t) => Some(&t),
            _ => None,
        }
    }

    pub fn as_mut_leaf_ref(&mut self) -> Option<&mut T> {
        match self {
            NodeContent::Leaf(t) => Some(t),
            _ => None,
        }
    }
}

///####################################################################################
/// Octree
///####################################################################################
#[cfg(feature = "serialization")]
use serde::{de::DeserializeOwned, Serialize};

impl<
        #[cfg(feature = "serialization")] T: Default + ToBencode + FromBencode + Serialize + DeserializeOwned,
        #[cfg(not(feature = "serialization"))] T: Default + ToBencode + FromBencode,
    > Octree<T>
where
    T: Default + PartialEq + Clone,
{
    pub(in crate::octree) fn make_uniform_children(&mut self, content: T) -> [usize; 8] {
        let children = [
            self.nodes.push(NodeContent::Leaf(content.clone())),
            self.nodes.push(NodeContent::Leaf(content.clone())),
            self.nodes.push(NodeContent::Leaf(content.clone())),
            self.nodes.push(NodeContent::Leaf(content.clone())),
            self.nodes.push(NodeContent::Leaf(content.clone())),
            self.nodes.push(NodeContent::Leaf(content.clone())),
            self.nodes.push(NodeContent::Leaf(content.clone())),
            self.nodes.push(NodeContent::Leaf(content)),
        ];
        self.node_children
            .resize(self.nodes.len(), NodeChildren::new(key_none_value()));
        children
    }

    pub(in crate::octree) fn deallocate_children_of(&mut self, node: usize) {
        let mut to_deallocate = Vec::new();
        if let Some(children) = self.node_children[node].iter() {
            for child in children {
                if crate::object_pool::key_might_be_valid(*child) {
                    to_deallocate.push(*child);
                }
            }
            for child in to_deallocate {
                self.deallocate_children_of(child); // Recursion should be fine as depth is not expceted to be more, than 32
                self.nodes.free(child);
            }
        }
        self.node_children[node].content = NodeChildrenArray::NoChildren;
    }

    pub(in crate::octree) fn simplify(&mut self, node: usize) -> bool {
        let mut data = NodeContent::Nothing;
        if crate::object_pool::key_might_be_valid(node) {
            for i in 0..8 {
                let child_key = self.node_children[node][i];
                if crate::object_pool::key_might_be_valid(child_key) {
                    if let Some(leaf_data) = self.nodes.get(child_key).as_leaf_ref() {
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
            *self.nodes.get_mut(node) = data;
            self.deallocate_children_of(node); // no need to use this as all the children are leaves, but it's more understanfdable this way
            true
        } else {
            false
        }
    }
}
