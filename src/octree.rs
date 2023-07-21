use crate::spatial::V3c;
use crate::spatial::{hash_region, offset_region};

pub enum Error {
    InvalidNodeSize(u32),
    InvalidPosition { x: u32, y: u32, z: u32 },
    InvalidOctant(usize),
    UnknownError,
}

// TODO: create trait for data instead of the generic T

///####################################################################################
/// Node
///####################################################################################
#[derive(Default)]
struct Node<T>
where
    T: Default,
{
    min_position: V3c<u32>,
    size: u32,
    content: Option<T>,
    children: [crate::object_pool::ItemKey; 8],
}

use crate::object_pool::{ItemKey, ObjectPool};
struct Octree<Content>
where
    Content: Default,
{
    pub auto_simplify: bool,
    root_node: ItemKey,
    nodes: ObjectPool<Node<Content>>,
}

impl<T> Node<T>
where
    T: Default,
{
    /// Returns whether the `Node` contains the given position.
    pub(crate) fn contains(&self, position: &V3c<u32>) -> bool {
        position.x >= self.min_position.x
            && position.x < self.min_position.x + self.size
            && position.y >= self.min_position.y
            && position.y < self.min_position.y + self.size
            && position.z >= self.min_position.z
            && position.z < self.min_position.z + self.size
    }

    /// Returns with the index of the child in the children array
    pub(crate) fn child_octant_for(&self, position: &V3c<u32>) -> usize {
        assert!(self.contains(position));
        hash_region(&(position - &self.min_position), self.size)
    }

    /// Returns with the immediate child of it at the position, should there be one there
    pub(crate) fn child_at(&self, position: &V3c<u32>) -> ItemKey {
        self.children[self.child_octant_for(position)]
    }

    pub(crate) fn is_leaf(&self) -> bool {
        self.content.is_some()
    }
}

///####################################################################################
/// Octree
///####################################################################################
impl<T> Octree<T>
where
    T: Default + PartialEq + Clone + std::fmt::Debug,
{
    pub fn new(size: u32) -> Result<Self, Error> {
        if 0 == size || (size as f32).log(2.0).fract() != 0.0 {
            // Only multiples of two are valid sizes
            return Err(Error::InvalidNodeSize(size));
        }
        let mut nodes = ObjectPool::<Node<T>>::with_capacity(size.pow(3) as usize);
        Ok(Self {
            auto_simplify: true,
            root_node: nodes.push(Node {
                size,
                ..Default::default()
            }),
            nodes,
        })
    }

    pub fn insert(&mut self, position: &V3c<u32>, data: T) -> Result<(), Error> {
        self.insert_at_lod(position, 1, data)
    }

    pub fn insert_at_lod(
        &mut self,
        position: &V3c<u32>,
        min_node_size: u32,
        data: T,
    ) -> Result<(), Error> {
        if 0 == min_node_size || (min_node_size as f32).log(2.0).fract() != 0.0 {
            // Only multiples of two are valid sizes
            return Err(Error::InvalidNodeSize(min_node_size));
        }

        if !self.nodes.get(self.root_node).contains(position) {
            return Err(Error::InvalidPosition {
                x: position.x,
                y: position.y,
                z: position.z,
            });
        }

        // A vector does not consume significant resources in this case, e.g. a 4096*4096*4096 chunk has depth of 12
        let mut node_stack = vec![self.root_node];
        loop {
            let current_node_key = *node_stack.last().unwrap();
            let current_size = self.nodes.get(current_node_key).size;
            let target_child_octant = self.nodes.get(current_node_key).child_octant_for(position);
            if current_size > min_node_size {
                // iteration needs to go deeper, as current Node size is still larger, than the requested
                if self.nodes.get(current_node_key).children[target_child_octant].is_some() {
                    node_stack.push(self.nodes.get(current_node_key).children[target_child_octant]);
                } else {
                    // no children are available for the target octant
                    if self.nodes.get(current_node_key).is_leaf()
                        && *self.nodes.get(current_node_key).content.as_ref().unwrap() == data
                    {
                        // The current Node is a leaf, but the data stored equals the data to be set, so no need to go deeper as tha data already matches
                        break;
                    }
                    let child_size = &self.nodes.get(current_node_key).size / 2;
                    let current_node_min_position = self.nodes.get(current_node_key).min_position;
                    if self.nodes.get(current_node_key).is_leaf()
                        && *self.nodes.get(current_node_key).content.as_ref().unwrap() != data
                    {
                        // The current Node is a leaf, which essentially represents an area where all the contained space have the same data.
                        // The contained data does not match the given data to set the position to, so all of the Nodes' children need to be created
                        // as separate Nodes with the same data as their parent to keep integrity
                        let current_content = self.nodes.get(current_node_key).content.clone();
                        let new_children = [
                            self.nodes.push(Node {
                                min_position: current_node_min_position
                                    + offset_region(0) * child_size,
                                size: child_size,
                                content: current_content.clone(),
                                ..Default::default()
                            }),
                            self.nodes.push(Node {
                                min_position: current_node_min_position
                                    + offset_region(1) * child_size,
                                size: child_size,
                                content: current_content.clone(),
                                ..Default::default()
                            }),
                            self.nodes.push(Node {
                                min_position: current_node_min_position
                                    + offset_region(2) * child_size,
                                size: child_size,
                                content: current_content.clone(),
                                ..Default::default()
                            }),
                            self.nodes.push(Node {
                                min_position: current_node_min_position
                                    + offset_region(3) * child_size,
                                size: child_size,
                                content: current_content.clone(),
                                ..Default::default()
                            }),
                            self.nodes.push(Node {
                                min_position: current_node_min_position
                                    + offset_region(4) * child_size,
                                size: child_size,
                                content: current_content.clone(),
                                ..Default::default()
                            }),
                            self.nodes.push(Node {
                                min_position: current_node_min_position
                                    + offset_region(5) * child_size,
                                size: child_size,
                                content: current_content.clone(),
                                ..Default::default()
                            }),
                            self.nodes.push(Node {
                                min_position: current_node_min_position
                                    + offset_region(6) * child_size,
                                size: child_size,
                                content: current_content.clone(),
                                ..Default::default()
                            }),
                            self.nodes.push(Node {
                                min_position: current_node_min_position
                                    + offset_region(7) * child_size,
                                size: child_size,
                                content: current_content,
                                ..Default::default()
                            }),
                        ];
                        self.nodes.get_mut(current_node_key).content = None;
                        self.nodes.get_mut(current_node_key).children = new_children;
                        node_stack
                            .push(self.nodes.get(current_node_key).children[target_child_octant]);
                    } else {
                        // current Node is a non-leaf Node, which doesn't have the child at the requested position, so it is inserted
                        node_stack.push(self.nodes.push(Node {
                            min_position: current_node_min_position
                                + offset_region(target_child_octant) * child_size,
                            size: child_size,
                            ..Default::default()
                        }));

                        self.nodes.get_mut(current_node_key).children[target_child_octant] =
                            *node_stack.last().unwrap();
                    }
                }
            } else {
                // current_size == min_node_size, which is the desired depth, so set content of current node
                self.nodes.get_mut(current_node_key).content = Some(data);
                for ref mut child in self.nodes.get_mut(*node_stack.last().unwrap()).children {
                    // Erase all the children of the Node, should there be any, because the current Node is set to a leaf
                    self.nodes.free(*child);
                    *child = ItemKey::none_value();
                }
                break;
            }
        }

        if self.auto_simplify {
            for node_key in node_stack.into_iter().rev() {
                if !self.simplify(&node_key) {
                    break; // If any Nodes fail to simplify, no need to continue because their parents can not be simplified because of it
                }
            }
        }
        Ok(())
    }

    pub fn get(&self, position: &V3c<u32>) -> Option<&T> {
        if !self.nodes.get(self.root_node).contains(position) {
            return None;
        }

        let mut current_node_key = self.root_node;
        loop {
            if self.nodes.get(current_node_key).is_leaf() {
                return self.nodes.get(current_node_key).content.as_ref();
            }

            let child_at_position = self.nodes.get(current_node_key).child_at(position);
            if child_at_position.is_some() {
                current_node_key = child_at_position;
            } else {
                return None;
            }
        }
    }

    pub fn get_mut(&mut self, position: &V3c<u32>) -> Option<&mut T> {
        if !self.nodes.get(self.root_node).contains(position) {
            return None;
        }

        let mut current_node_key = self.root_node;
        loop {
            if self.nodes.get(current_node_key).is_leaf() {
                return self.nodes.get_mut(current_node_key).content.as_mut();
            }

            let child_at_position = self.nodes.get(current_node_key).child_at(position);
            if child_at_position.is_some() {
                current_node_key = child_at_position;
            } else {
                return None;
            }
        }
    }

    fn node(&self, key: &ItemKey) -> Option<&Node<T>> {
        if key.is_some() {
            return Some(self.nodes.get(*key));
        }
        None
    }

    pub fn simplify(&mut self, node: &ItemKey) -> bool {
        let mut data = None;
        if node.is_some() {
            for i in 0..8 {
                let child_key = &self.node(node).unwrap().children[i];
                if let Some(child) = self.node(child_key) {
                    if child.is_leaf() {
                        let leaf_data = child.content.clone().unwrap();
                        if data.as_ref().is_none() {
                            data = Some(leaf_data);
                        } else if *data.as_ref().unwrap() != leaf_data {
                            return false;
                        }
                    } else {
                        return false;
                    }
                } else {
                    return false;
                }
            }

            self.nodes.get_mut(*node).content = data;
            self.nodes.get_mut(*node).children = [
                ItemKey::none_value(),
                ItemKey::none_value(),
                ItemKey::none_value(),
                ItemKey::none_value(),
                ItemKey::none_value(),
                ItemKey::none_value(),
                ItemKey::none_value(),
                ItemKey::none_value(),
            ];
        }
        false
    }

    pub fn clear(&mut self, position: &V3c<u32>) -> Result<(), Error> {
        self.clear_at_lod(position, 1)
    }

    pub fn clear_at_lod(&mut self, position: &V3c<u32>, min_node_size: u32) -> Result<(), Error> {
        if 0 == min_node_size || (min_node_size as f32).log(2.0).fract() != 0.0 {
            // Only multiples of two are valid sizes
            return Err(Error::InvalidNodeSize(min_node_size));
        }

        if !self.nodes.get(self.root_node).contains(position) {
            return Err(Error::InvalidPosition {
                x: position.x,
                y: position.y,
                z: position.z,
            });
        }

        // A vector does not consume significant resources in this case, e.g. a 4096*4096*4096 chunk has depth of 12
        let mut node_stack = vec![self.root_node];
        let mut target_child_octant = 9; //This init value should not be used. In case there is only one node, there is parent of it;
        loop {
            let current_node_key = *node_stack.last().unwrap();
            let current_size = self.nodes.get(current_node_key).size;
            if current_size > min_node_size {
                // iteration needs to go deeper, as current Node size is still larger, than the requested
                target_child_octant = self.nodes.get(current_node_key).child_octant_for(position);
                if self.nodes.get(current_node_key).children[target_child_octant].is_some() {
                    node_stack.push(self.nodes.get(current_node_key).children[target_child_octant]);
                } else {
                    // no children are available for the target octant
                    if self.nodes.get(current_node_key).is_leaf() {
                        // The current Node is a leaf, which essentially represents an area where all the contained space have the same data.
                        // The contained data does not match the given data to set the position to, so all of the Nodes' children need to be created
                        // as separate Nodes with the same data as their parent to keep integrity
                        let child_size = &self.nodes.get(current_node_key).size / 2;
                        let current_node_min_position =
                            self.nodes.get(current_node_key).min_position;
                        let current_content = self.nodes.get(current_node_key).content.clone();
                        let new_children = [
                            self.nodes.push(Node {
                                min_position: current_node_min_position
                                    + offset_region(0) * child_size,
                                size: child_size,
                                content: current_content.clone(),
                                ..Default::default()
                            }),
                            self.nodes.push(Node {
                                min_position: current_node_min_position
                                    + offset_region(1) * child_size,
                                size: child_size,
                                content: current_content.clone(),
                                ..Default::default()
                            }),
                            self.nodes.push(Node {
                                min_position: current_node_min_position
                                    + offset_region(2) * child_size,
                                size: child_size,
                                content: current_content.clone(),
                                ..Default::default()
                            }),
                            self.nodes.push(Node {
                                min_position: current_node_min_position
                                    + offset_region(3) * child_size,
                                size: child_size,
                                content: current_content.clone(),
                                ..Default::default()
                            }),
                            self.nodes.push(Node {
                                min_position: current_node_min_position
                                    + offset_region(4) * child_size,
                                size: child_size,
                                content: current_content.clone(),
                                ..Default::default()
                            }),
                            self.nodes.push(Node {
                                min_position: current_node_min_position
                                    + offset_region(5) * child_size,
                                size: child_size,
                                content: current_content.clone(),
                                ..Default::default()
                            }),
                            self.nodes.push(Node {
                                min_position: current_node_min_position
                                    + offset_region(6) * child_size,
                                size: child_size,
                                content: current_content.clone(),
                                ..Default::default()
                            }),
                            self.nodes.push(Node {
                                min_position: current_node_min_position
                                    + offset_region(7) * child_size,
                                size: child_size,
                                content: current_content,
                                ..Default::default()
                            }),
                        ];
                        self.nodes.get_mut(current_node_key).content = None;
                        self.nodes.get_mut(current_node_key).children = new_children;
                        node_stack
                            .push(self.nodes.get(current_node_key).children[target_child_octant]);
                    } else {
                        // current Node is a non-leaf Node, which doesn't have the child at the requested position.
                        // Nothing to do, because child didn't exist in the first place
                        break;
                    }
                }
            } else {
                // current_size == min_node_size, which is the desired depth, so unset the current node and its children
                for ref mut child in self.nodes.get_mut(current_node_key).children {
                    self.nodes.free(*child);
                    *child = ItemKey::none_value();
                }
                self.nodes.free(current_node_key);

                // Set the parents child to None
                if node_stack.len() >= 2 && target_child_octant < 9 {
                    self.nodes
                        .get_mut(node_stack[node_stack.len() - 2])
                        .children[target_child_octant] = ItemKey::none_value();
                }
                break;
            }
        }

        Ok(())
    }

    //TODO: get, based on a ray
}

///####################################################################################
/// Tests
///####################################################################################
#[cfg(test)]
mod octree_tests {
    use super::Octree;
    use crate::octree::V3c;

    #[test]
    fn test_simple_insert_and_get() {
        let mut tree = Octree::<f32>::new(2).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), 5.0).ok();
        tree.insert(&V3c::new(0, 1, 0), 6.0).ok();
        tree.insert(&V3c::new(0, 0, 1), 7.0).ok();

        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == 6.0);
        assert!(*tree.get(&V3c::new(0, 0, 1)).unwrap() == 7.0);
        assert!(tree.get(&V3c::new(1, 1, 1)).is_none());
    }

    #[test]
    fn test_get_mut() {
        let mut tree = Octree::<f32>::new(2).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), 5.0).ok();
        tree.insert(&V3c::new(0, 1, 0), 6.0).ok();
        tree.insert(&V3c::new(0, 0, 1), 7.0).ok();

        assert!(*tree.get_mut(&V3c::new(1, 0, 0)).unwrap() == 5.0);
        assert!(*tree.get_mut(&V3c::new(0, 1, 0)).unwrap() == 6.0);
        assert!(*tree.get_mut(&V3c::new(0, 0, 1)).unwrap() == 7.0);
        assert!(tree.get_mut(&V3c::new(1, 1, 1)).is_none());
    }

    #[test]
    fn test_insert_at_lod() {
        let mut tree = Octree::<f32>::new(4).ok().unwrap();
        tree.auto_simplify = false;

        // This will set the area equal to 8 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 2, 5.0).ok();

        assert!(*tree.get(&V3c::new(0, 0, 0)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(0, 0, 1)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(0, 1, 1)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(1, 0, 1)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(1, 1, 0)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(1, 1, 1)).unwrap() == 5.0);

        // This will set the area equal to 64 1-sized nodes:
        // a size-4 node includes 2 levels,
        // 1-sized nodes at the bottom level doesn't have children,
        // 2-sized nodes above have 8 children each
        // so one 4-sized node has 8*8 = 64 children
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, 1.0).ok();
        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    if tree.get(&V3c::new(x, y, z)).is_some()
                        && *tree.get(&V3c::new(x, y, z)).unwrap() == 1.0
                    {
                        hits += 1;
                    }
                }
            }
        }
        assert!(hits == 64);
    }

    #[test]
    fn test_insert_at_lod_with_simplify() {
        let mut tree = Octree::<f32>::new(4).ok().unwrap();

        // This will set the area equal to 8 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 2, 5.0).ok();

        assert!(*tree.get(&V3c::new(0, 0, 0)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(0, 0, 1)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(0, 1, 1)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(1, 0, 1)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(1, 1, 0)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(1, 1, 1)).unwrap() == 5.0);

        // This will set the area equal to 64 1-sized nodes:
        // a size-4 node includes 2 levels,
        // 1-sized nodes at the bottom level doesn't have children,
        // 2-sized nodes above have 8 children each
        // so one 4-sized node has 8*8 = 64 children
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, 1.0).ok();
        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    if tree.get(&V3c::new(x, y, z)).is_some()
                        && *tree.get(&V3c::new(x, y, z)).unwrap() == 1.0
                    {
                        hits += 1;
                    }
                }
            }
        }
        assert!(hits == 64);
    }

    #[test]
    fn test_simplifyable_insert_and_get() {
        let mut tree = Octree::<f32>::new(2).ok().unwrap();

        // The below set of values should be simplified to a single node
        tree.insert(&V3c::new(0, 0, 0), 5.0).ok();
        tree.insert(&V3c::new(0, 0, 1), 5.0).ok();
        tree.insert(&V3c::new(0, 1, 0), 5.0).ok();
        tree.insert(&V3c::new(0, 1, 1), 5.0).ok();
        tree.insert(&V3c::new(1, 0, 0), 5.0).ok();
        tree.insert(&V3c::new(1, 0, 1), 5.0).ok();
        tree.insert(&V3c::new(1, 1, 0), 5.0).ok();
        tree.insert(&V3c::new(1, 1, 1), 5.0).ok();

        // The below should brake the simplified node back to its party
        tree.insert(&V3c::new(0, 0, 0), 4.0).ok();

        // Integrity should be kept
        assert!(*tree.get(&V3c::new(0, 0, 0)).unwrap() == 4.0);
        assert!(*tree.get(&V3c::new(0, 0, 1)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(0, 1, 1)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(1, 0, 1)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(1, 1, 0)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(1, 1, 1)).unwrap() == 5.0);
    }

    #[test]
    fn test_simple_clear() {
        let mut tree = Octree::<f32>::new(2).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), 5.0).ok();
        tree.insert(&V3c::new(0, 1, 0), 6.0).ok();
        tree.insert(&V3c::new(0, 0, 1), 7.0).ok();
        tree.clear(&V3c::new(0, 0, 1)).ok();

        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == 6.0);
        assert!(tree.get(&V3c::new(0, 0, 1)).is_none());
        assert!(tree.get(&V3c::new(1, 1, 1)).is_none());
    }

    #[test]
    fn test_simplifyable_clear() {
        let mut tree = Octree::<f32>::new(2).ok().unwrap();

        // The below set of values should be simplified to a single node
        tree.insert(&V3c::new(0, 0, 0), 5.0).ok();
        tree.insert(&V3c::new(0, 0, 1), 5.0).ok();
        tree.insert(&V3c::new(0, 1, 0), 5.0).ok();
        tree.insert(&V3c::new(0, 1, 1), 5.0).ok();
        tree.insert(&V3c::new(1, 0, 0), 5.0).ok();
        tree.insert(&V3c::new(1, 0, 1), 5.0).ok();
        tree.insert(&V3c::new(1, 1, 0), 5.0).ok();
        tree.insert(&V3c::new(1, 1, 1), 5.0).ok();

        // The below should brake the simplified node back to its party
        tree.clear(&V3c::new(0, 0, 0)).ok();

        // Integrity should be kept
        assert!(tree.get(&V3c::new(0, 0, 0)).is_none());
        assert!(*tree.get(&V3c::new(0, 0, 1)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(0, 1, 1)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(1, 0, 1)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(1, 1, 0)).unwrap() == 5.0);
        assert!(*tree.get(&V3c::new(1, 1, 1)).unwrap() == 5.0);
    }

    #[test]
    fn test_clear_at_lod() {
        let mut tree = Octree::<f32>::new(4).ok().unwrap();

        // This will set the area equal to 64 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, 5.0).ok();

        // This will clear an area equal to 8 1-sized nodes
        tree.clear_at_lod(&V3c::new(0, 0, 0), 2).ok();

        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    if tree.get(&V3c::new(x, y, z)).is_some()
                        && *tree.get(&V3c::new(x, y, z)).unwrap() == 5.0
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
