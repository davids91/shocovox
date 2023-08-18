use crate::octree::{V3c, Cube, Octree, hash_region};
use bendy::{decoding::FromBencode, encoding::ToBencode};

///####################################################################################
/// Node and support types, functions for Octree
///####################################################################################

/// Returns whether the given bound contains the given position.
pub(crate) fn bound_contains(bounds: &Cube, position: &V3c<u32>) -> bool {
    position.x >= bounds.min_position.x
        && position.x <= bounds.min_position.x + bounds.size
        && position.y >= bounds.min_position.y
        && position.y <= bounds.min_position.y + bounds.size
        && position.z >= bounds.min_position.z
        && position.z <= bounds.min_position.z + bounds.size
}

/// Returns with the octant value(i.e. index) of the child for the given position
pub(crate) fn child_octant_for(bounds: &Cube, position: &V3c<u32>) -> usize {
    assert!(bound_contains(bounds, position));
    hash_region(
        &(*position - bounds.min_position).into(),
        bounds.size as f32,
    )
}

#[cfg(feature = "serialization")]
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Default)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub(in crate::octree)enum NodeContent<T> {
    #[default]
    Nothing,
    Leaf(T),
}

impl<T> NodeContent<T> {
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
    T: Default + PartialEq + Clone + std::fmt::Debug,
{
    pub(in crate::octree) fn children_of(&self, node: usize) -> &[usize] {
        &self.node_children[(node * 8)..(node * 8 + 8)]
    }

    pub(in crate::octree) fn mutable_children_of(&mut self, node: usize) -> &mut [usize] {
        &mut self.node_children[(node * 8)..(node * 8 + 8)]
    }

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
            .resize(self.nodes.len() * 8, crate::object_pool::key_none_value());
        children
    }

    pub(in crate::octree) fn deallocate_children_of(&mut self, node: usize) {
        let mut to_deallocate = Vec::new();
        for child in self.children_of(node).iter() {
            if crate::object_pool::key_might_be_some(*child) {
                to_deallocate.push(*child);
            }
        }
        for child in to_deallocate {
            self.deallocate_children_of(child); // Recursion should be fine as depth is not expceted to be more, than 32
            self.nodes.free(child);
        }
        for child in self.mutable_children_of(node).iter_mut() {
            *child = crate::object_pool::key_none_value();
        }
    }

    pub(in crate::octree) fn get_node_leaf_data(&self, node: usize) -> Option<&T> {
        if crate::object_pool::key_might_be_some(node) {
            return self.nodes.get(node).as_leaf_ref();
        }
        None
    }

    pub(in crate::octree) fn simplify(&mut self, node: usize) -> bool {
        let mut data = NodeContent::Nothing;
        if crate::object_pool::key_might_be_some(node) {
            for i in 0..8 {
                let child_key = self.children_of(node)[i];
                if let Some(leaf_data) = self.get_node_leaf_data(child_key) {
                    if !data.is_leaf() {
                        data = NodeContent::Leaf(leaf_data.clone());
                    } else if data.leaf_data() != leaf_data {
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

///####################################################################################
/// Tests
///####################################################################################
#[cfg(test)]
mod octree_serialization_tests {
    use crate::octree::Octree;
    use crate::octree::V3c;

    #[test]
    fn test_octree_file_io() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();

        // This will set the area equal to 64 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, 5).ok();

        // This will clear an area equal to 8 1-sized nodes
        tree.clear_at_lod(&V3c::new(0, 0, 0), 2).ok();

        // save andd load into a new tree
        tree.save("test_junk_octree").ok();
        let tree_copy = Octree::<u32>::load("test_junk_octree").ok().unwrap();

        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    assert!(tree.get(&V3c::new(x, y, z)) == tree_copy.get(&V3c::new(x, y, z)));
                    if tree_copy.get(&V3c::new(x, y, z)).is_some()
                        && *tree_copy.get(&V3c::new(x, y, z)).unwrap() == 5
                    {
                        hits += 1;
                    }
                }
            }
        }

        // number of hits should be the number of nodes set minus the number of nodes cleared
        assert!(hits == (64 - 8));
    }
}
