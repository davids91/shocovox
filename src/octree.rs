use crate::spatial::math::{hash_region, offset_region, V3c};
use crate::spatial::Cube;

pub enum Error {
    InvalidNodeSize(u32),
    InvalidPosition { x: u32, y: u32, z: u32 },
}

/// Returns whether the given bound contains the given position.
pub(crate) fn bound_contains(bounds: &Cube, position: &V3c<u32>) -> bool {
    position.x >= bounds.min_position.x
        && position.x <= bounds.min_position.x + bounds.size
        && position.y >= bounds.min_position.y
        && position.y <= bounds.min_position.y + bounds.size
        && position.z >= bounds.min_position.z
        && position.z <= bounds.min_position.z + bounds.size
}

pub(crate) fn child_octant_for(bounds: &Cube, position: &V3c<u32>) -> usize {
    assert!(bound_contains(bounds, position));
    hash_region(
        &(*position - bounds.min_position).into(),
        bounds.size as f32,
    )
}

///####################################################################################
/// Octree
///####################################################################################
use crate::object_pool::ObjectPool;
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct Octree<Content: Default> {
    pub auto_simplify: bool,
    root_node: usize,
    root_size: u32,
    nodes: ObjectPool<Option<Content>>, //None means the Node is an internal node, Some(...) means the Node is a leaf
    node_children: Vec<usize>,
}

#[cfg(feature = "serialization")]
use std::fs::File;
#[cfg(feature = "serialization")]
use std::io::Read;
#[cfg(feature = "serialization")]
use std::io::Write;

impl<
        #[cfg(feature = "serialization")] T: Default + serde::Serialize + serde::de::DeserializeOwned,
        #[cfg(not(feature = "serialization"))] T: Default,
    > Octree<T>
where
    T: Default + PartialEq + Clone + std::fmt::Debug,
{
    #[cfg(feature = "serialization")]
    pub fn save(&mut self, path: &str) -> Result<(), std::io::Error> {
        let bytes = bendy::serde::to_bytes(&self).ok().unwrap();
        let mut file = File::create(path)?;
        file.write_all(&bytes)?;
        Ok(())
    }

    #[cfg(feature = "serialization")]
    pub fn load(path: &str) -> Result<Self, std::io::Error> {
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(bendy::serde::from_bytes(&bytes).ok().unwrap())
    }

    pub fn new(size: u32) -> Result<Self, Error> {
        if 0 == size || (size as f32).log(2.0).fract() != 0.0 {
            // Only multiples of two are valid sizes
            return Err(Error::InvalidNodeSize(size));
        }
        let mut nodes = ObjectPool::<Option<T>>::with_capacity(size.pow(3) as usize);
        let mut node_children = Vec::with_capacity((size.pow(3) * 8) as usize);
        node_children.resize(8, crate::object_pool::key_none_value());
        Ok(Self {
            auto_simplify: true,
            root_node: nodes.push(None),
            root_size: size,
            nodes,
            node_children,
        })
    }

    fn children_of(&self, node: usize) -> &[usize] {
        &self.node_children[(node * 8)..(node * 8 + 8)]
    }
    fn mutable_children_of(&mut self, node: usize) -> &mut [usize] {
        &mut self.node_children[(node * 8)..(node * 8 + 8)]
    }

    fn make_uniform_children(&mut self, content: T) -> [usize; 8] {
        let children = [
            self.nodes.push(Some(content.clone())),
            self.nodes.push(Some(content.clone())),
            self.nodes.push(Some(content.clone())),
            self.nodes.push(Some(content.clone())),
            self.nodes.push(Some(content.clone())),
            self.nodes.push(Some(content.clone())),
            self.nodes.push(Some(content.clone())),
            self.nodes.push(Some(content)),
        ];
        self.node_children
            .resize(self.nodes.len() * 8, crate::object_pool::key_none_value());
        children
    }

    fn deallocate_children_of(&mut self, node: usize) {
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

        let root_bounds = Cube::root_bounds(self.root_size);
        if !bound_contains(&root_bounds, position) {
            return Err(Error::InvalidPosition {
                x: position.x,
                y: position.y,
                z: position.z,
            });
        }

        // A vector does not consume significant resources in this case, e.g. a 4096*4096*4096 chunk has depth of 12
        let mut node_stack = vec![(self.root_node, root_bounds)];
        loop {
            let (current_node_key, current_bounds) = *node_stack.last().unwrap();
            let target_child_octant = child_octant_for(&current_bounds, position);

            if current_bounds.size > min_node_size {
                // iteration needs to go deeper, as current Node size is still larger, than the requested
                if crate::object_pool::key_might_be_some(
                    self.children_of(current_node_key)[target_child_octant],
                ) {
                    node_stack.push((
                        self.children_of(current_node_key)[target_child_octant],
                        Cube {
                            min_position: current_bounds.min_position
                                + offset_region(target_child_octant) * current_bounds.size / 2,
                            size: current_bounds.size / 2,
                        },
                    ));
                } else {
                    // no children are available for the target octant
                    if self.nodes.get(current_node_key).is_some()
                        && *self.nodes.get(current_node_key).as_ref().unwrap() == data
                    {
                        // The current Node is a leaf, but the data stored equals the data to be set, so no need to go deeper as tha data already matches
                        break;
                    }
                    if self.nodes.get(current_node_key).is_some()
                        && *self.nodes.get(current_node_key).as_ref().unwrap() != data
                    {
                        // The current Node is a leaf, which essentially represents an area where all the contained space have the same data.
                        // The contained data does not match the given data to set the position to, so all of the Nodes' children need to be created
                        // as separate Nodes with the same data as their parent to keep integrity
                        let current_content = self.nodes.get(current_node_key).clone();
                        let new_children = self.make_uniform_children(current_content.unwrap());
                        *self.nodes.get_mut(current_node_key) = None;
                        self.mutable_children_of(current_node_key)
                            .copy_from_slice(&new_children);
                        node_stack.push((
                            self.children_of(current_node_key)[target_child_octant],
                            Cube {
                                min_position: current_bounds.min_position
                                    + offset_region(target_child_octant) * current_bounds.size / 2,
                                size: current_bounds.size / 2,
                            },
                        ));
                    } else {
                        // current Node is a non-leaf Node, which doesn't have the child at the requested position, so it is inserted
                        let child_key = self.nodes.push(None);
                        self.node_children
                            .resize(self.nodes.len() * 8, crate::object_pool::key_none_value());

                        node_stack.push((
                            child_key,
                            Cube {
                                min_position: current_bounds.min_position
                                    + offset_region(target_child_octant) * current_bounds.size / 2,
                                size: current_bounds.size / 2,
                            },
                        ));
                        self.mutable_children_of(current_node_key)[target_child_octant] =
                            node_stack.last().unwrap().0;
                    }
                }
            } else {
                // current_bounds.size == min_node_size, which is the desired depth, so set content of current node
                *self.nodes.get_mut(current_node_key) = Some(data);
                self.deallocate_children_of(node_stack.last().unwrap().0);
                break;
            }
        }

        if self.auto_simplify {
            for (node_key, _node_bounds) in node_stack.into_iter().rev() {
                if !self.simplify(node_key) {
                    break; // If any Nodes fail to simplify, no need to continue because their parents can not be simplified because of it
                }
            }
        }
        Ok(())
    }

    pub fn get(&self, position: &V3c<u32>) -> Option<&T> {
        let mut current_bounds = Cube::root_bounds(self.root_size);
        if !bound_contains(&current_bounds, position) {
            return None;
        }

        let mut current_node_key = self.root_node;
        loop {
            if self.nodes.get(current_node_key).is_some() {
                return self.nodes.get(current_node_key).as_ref();
            }
            let current_node = self.nodes.get(current_node_key);
            let child_octant_at_position = child_octant_for(&current_bounds, position);
            let child_at_position = self.children_of(current_node_key)[child_octant_at_position];
            if crate::object_pool::key_might_be_some(child_at_position) {
                current_node_key = child_at_position;
                current_bounds = Cube::child_bounds_for(&current_bounds, child_octant_at_position);
            } else {
                return None;
            }
        }
    }

    pub fn get_mut(&mut self, position: &V3c<u32>) -> Option<&mut T> {
        let mut current_bounds = Cube::root_bounds(self.root_size);
        if !bound_contains(&current_bounds, position) {
            return None;
        }

        let mut current_node_key = self.root_node;
        loop {
            if self.nodes.get(current_node_key).is_some() {
                return self.nodes.get_mut(current_node_key).as_mut();
            }
            let current_node = self.nodes.get(current_node_key);
            let child_octant_at_position = child_octant_for(&current_bounds, position);
            let child_at_position = self.children_of(current_node_key)[child_octant_at_position];
            if crate::object_pool::key_might_be_some(child_at_position) {
                current_node_key = child_at_position;
                current_bounds = Cube::child_bounds_for(&current_bounds, child_octant_at_position);
            } else {
                return None;
            }
        }
    }

    fn node(&self, node: usize) -> Option<&Option<T>> {
        if crate::object_pool::key_might_be_some(node) {
            return Some(self.nodes.get(node));
        }
        None
    }

    fn simplify(&mut self, node: usize) -> bool {
        let mut data = None;
        if crate::object_pool::key_might_be_some(node) {
            for i in 0..8 {
                let child_key = self.children_of(node)[i];
                if let Some(child) = self.node(child_key) {
                    if child.is_some() {
                        let leaf_data = child.clone().unwrap();
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
            *self.nodes.get_mut(node) = data;
            self.deallocate_children_of(node); // no need to use this as all the children are leaves, but it's more understanfdable this way
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self, position: &V3c<u32>) -> Result<(), Error> {
        self.clear_at_lod(position, 1)
    }

    pub fn clear_at_lod(&mut self, position: &V3c<u32>, min_node_size: u32) -> Result<(), Error> {
        if 0 == min_node_size || (min_node_size as f32).log(2.0).fract() != 0.0 {
            // Only multiples of two are valid sizes
            return Err(Error::InvalidNodeSize(min_node_size));
        }
        let root_bounds = Cube::root_bounds(self.root_size);
        if !bound_contains(&root_bounds, position) {
            return Err(Error::InvalidPosition {
                x: position.x,
                y: position.y,
                z: position.z,
            });
        }

        // A vector does not consume significant resources in this case, e.g. a 4096*4096*4096 chunk has depth of 12
        let mut node_stack = vec![(self.root_node, root_bounds)];
        let mut target_child_octant = 9; //This init value should not be used. In case there is only one node, there is parent of it;
        loop {
            let (current_node_key, current_bounds) = *node_stack.last().unwrap();
            if current_bounds.size > min_node_size {
                // iteration needs to go deeper, as current Node size is still larger, than the requested
                target_child_octant = child_octant_for(&current_bounds, position);
                if crate::object_pool::key_might_be_some(
                    self.children_of(current_node_key)[target_child_octant],
                ) {
                    //Iteration needs to go deeper
                    node_stack.push((
                        self.children_of(current_node_key)[target_child_octant],
                        Cube {
                            min_position: current_bounds.min_position
                                + offset_region(target_child_octant) * current_bounds.size / 2,
                            size: current_bounds.size / 2,
                        },
                    ));
                } else {
                    // no children are available for the target octant
                    if self.nodes.get(current_node_key).is_some() {
                        // The current Node is a leaf, which essentially represents an area where all the contained space have the same data.
                        // The contained data does not match the given data to set the position to, so all of the Nodes' children need to be created
                        // as separate Nodes with the same data as their parent to keep integrity
                        let current_content = self.nodes.get(current_node_key).clone();
                        let new_children = self.make_uniform_children(current_content.unwrap());
                        *self.nodes.get_mut(current_node_key) = None;
                        self.mutable_children_of(current_node_key)
                            .copy_from_slice(&new_children);
                        node_stack.push((
                            self.children_of(current_node_key)[target_child_octant],
                            Cube {
                                min_position: current_bounds.min_position
                                    + offset_region(target_child_octant) * current_bounds.size / 2,
                                size: current_bounds.size / 2,
                            },
                        ));
                    } else {
                        // current Node is a non-leaf Node, which doesn't have the child at the requested position.
                        // Nothing to do, because child didn't exist in the first place
                        break;
                    }
                }
            } else {
                // current_bounds.size == min_node_size, which is the desired depth, so unset the current node and its children
                self.deallocate_children_of(current_node_key);
                self.nodes.free(current_node_key);

                // Set the parents child to None
                if node_stack.len() >= 2 && target_child_octant < 9 {
                    self.mutable_children_of(node_stack[node_stack.len() - 2].0)
                        [target_child_octant] = crate::object_pool::key_none_value();
                }
                break;
            }
        }

        Ok(())
    }

    #[cfg(feature = "raytracing")]
    /// provides the collision point of the ray with the contained voxel field
    /// return reference of the data, collision point and normal at impact, should there be any
    pub fn get_by_ray(&self, ray: &crate::spatial::Ray) -> Option<(&T, V3c<f32>, V3c<f32>)> {
        todo!()
    }
}

///####################################################################################
/// Tests
///####################################################################################
#[cfg(test)]
mod octree_tests {
    use super::Octree;
    use crate::octree::V3c;

    #[cfg(feature = "raytracing")]
    use crate::spatial::Ray;

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

    #[cfg(feature = "serialization")]
    #[test]
    fn test_octree_file_io() {
        let mut tree = Octree::<f32>::new(4).ok().unwrap();

        // This will set the area equal to 64 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, 5.0).ok();

        // This will clear an area equal to 8 1-sized nodes
        tree.clear_at_lod(&V3c::new(0, 0, 0), 2).ok();

        // save andd load into a new tree
        tree.save("test_junk_octree").ok();
        let tree_copy = Octree::<f32>::load("test_junk_octree").ok().unwrap();

        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    assert!(tree.get(&V3c::new(x, y, z)) == tree_copy.get(&V3c::new(x, y, z)));
                    if tree_copy.get(&V3c::new(x, y, z)).is_some()
                        && *tree_copy.get(&V3c::new(x, y, z)).unwrap() == 5.0
                    {
                        hits += 1;
                    }
                }
            }
        }

        // number of hits should be the number of nodes set minus the number of nodes cleared
        assert!(hits == (64 - 8));
    }

    #[cfg(feature = "raytracing")]
    use rand::{rngs::ThreadRng, Rng};

    #[cfg(feature = "raytracing")]
    fn make_ray_point_to(target: &V3c<f32>, rng: &mut ThreadRng) -> Ray {
        let origin = V3c {
            x: rng.gen_range(4..10) as f32,
            y: rng.gen_range(4..10) as f32,
            z: rng.gen_range(4..10) as f32,
        };
        Ray {
            direction: (*target - origin).normalized(),
            origin,
        }
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_get_by_ray() {
        let mut rng = rand::thread_rng();
        let mut tree = Octree::<f32>::new(4).ok().unwrap();
        let mut filled = Vec::new();
        let mut not_filled = Vec::new();
        for x in 1..2 {
            for y in 1..2 {
                for z in 1..2 {
                    if 10 > rng.gen_range(0..20) {
                        let pos = V3c::new(x, y, z);
                        tree.insert(&pos, 5.0).ok();
                        filled.push(pos);
                    } else {
                        not_filled.push(V3c::new(x, y, z));
                    }
                }
            }
        }

        for p in filled.into_iter() {
            let ray = make_ray_point_to(&V3c::new(p.x as f32, p.y as f32, p.z as f32), &mut rng);
            assert!(tree.get_by_ray(&ray).is_some());
            assert!(*tree.get_by_ray(&ray).unwrap().0 == 5.0);
        }
        for p in not_filled.into_iter() {
            let ray = make_ray_point_to(&V3c::new(p.x as f32, p.y as f32, p.z as f32), &mut rng);
            assert!(tree.get_by_ray(&ray).is_none());
        }
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_get_by_ray_from_inside() {
        let mut rng = rand::thread_rng();
        let mut tree = Octree::<f32>::new(16).ok().unwrap();
        let mut filled = Vec::new();
        for x in 1..4 {
            for y in 1..4 {
                for z in 1..4 {
                    if 10 > rng.gen_range(0..20) {
                        let pos = V3c::new(x, y, z);
                        tree.insert(&pos, 5.0).ok();
                        filled.push(pos);
                    }
                }
            }
        }

        for p in filled.into_iter() {
            let pos = V3c::new(p.x as f32, p.y as f32, p.z as f32);
            let ray = make_ray_point_to(&pos, &mut rng);
            assert!(tree.get(&pos.into()).is_some());
            assert!(tree.get_by_ray(&ray).is_some());
            assert!(*tree.get_by_ray(&ray).unwrap().0 == 5.0);
        }
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_edge_case_unreachable() {
        let mut tree = Octree::<f32>::new(4).ok().unwrap();
        tree.insert(&V3c::new(3, 0, 0), 0.).ok();
        tree.insert(&V3c::new(3, 3, 0), 1.).ok();
        tree.insert(&V3c::new(0, 3, 0), 2.).ok();

        for y in 0..4 {
            tree.insert(&V3c::new(0, y, y), 3.1).ok();
            tree.insert(&V3c::new(1, y, y), 3.1).ok();
            tree.insert(&V3c::new(2, y, y), 3.1).ok();
            tree.insert(&V3c::new(3, y, y), 3.1).ok();
        }

        let ray = Ray {
            origin: V3c {
                x: 10.0,
                y: 10.0,
                z: -5.0,
            },
            direction: V3c {
                x: -0.66739213,
                y: -0.6657588,
                z: 0.333696,
            },
        };
        let _ = tree.get_by_ray(&ray); //Should not fail with unreachable code panic
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_edge_case_cube_edges() {
        let mut tree = Octree::<f32>::new(4).ok().unwrap();
        tree.insert(&V3c::new(3, 0, 0), 0.).ok();
        tree.insert(&V3c::new(3, 3, 0), 1.).ok();
        tree.insert(&V3c::new(0, 3, 0), 2.).ok();

        for y in 0..4 {
            tree.insert(&V3c::new(0, y, y), 3.1).ok();
            tree.insert(&V3c::new(1, y, y), 3.2).ok();
            tree.insert(&V3c::new(2, y, y), 3.3).ok();
            tree.insert(&V3c::new(3, y, y), 3.4).ok();
        }

        let ray = Ray {
            origin: V3c {
                x: 10.0,
                y: 10.0,
                z: -5.0,
            },
            direction: (V3c {
                x: 3.0,
                y: 1.9,
                z: 2.0,
            } - V3c {
                x: 10.0,
                y: 10.0,
                z: -5.0,
            })
            .normalized(),
        };

        //Should reach position 3, 2, 2
        assert!(tree.get_by_ray(&ray).is_some_and(|v| *v.0 == 3.4));
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_edge_case_ray_behind_octree() {
        let mut tree = Octree::<f32>::new(4).ok().unwrap();
        tree.insert(&V3c::new(0, 3, 0), 5.).ok();
        let origin = V3c::new(2., 2., -5.);
        let ray = Ray {
            direction: (V3c::new(0., 3., 0.) - origin).normalized(),
            origin,
        };
        assert!(tree.get(&V3c::new(0, 3, 0)).is_some());
        assert!(*tree.get(&V3c::new(0, 3, 0)).unwrap() == 5.);
        assert!(tree.get_by_ray(&ray).is_some());
        assert!(*tree.get_by_ray(&ray).unwrap().0 == 5.);
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_edge_case_overlapping_voxels() {
        let mut tree = Octree::<f32>::new(4).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 0), 5.).ok();
        tree.insert(&V3c::new(1, 0, 0), 6.).ok();

        let test_ray = Ray {
            origin: V3c {
                x: 2.0,
                y: 4.0,
                z: -2.0,
            },
            direction: V3c {
                x: -0.23184556,
                y: -0.79392403,
                z: 0.5620785,
            },
        };
        assert!(tree.get_by_ray(&test_ray).is_some_and(|hit| *hit.0 == 6.));
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_edge_case_edge_raycast() {
        let mut tree = Octree::<f32>::new(4).ok().unwrap();

        for x in 0..4 {
            for z in 0..4 {
                tree.insert(&V3c::new(x, 0, z), 5.0).ok();
            }
        }
        let ray = Ray {
            origin: V3c {
                x: 2.0,
                y: 4.0,
                z: -2.0,
            },
            direction: V3c {
                x: -0.47839317,
                y: -0.71670955,
                z: 0.50741255,
            },
        };
        let result = tree.get_by_ray(&ray);
        assert!(result.is_none() || *result.unwrap().0 == 5.0);
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_edge_case_voxel_corner() {
        let mut tree = Octree::<f32>::new(4).ok().unwrap();

        for x in 0..4 {
            for z in 0..4 {
                tree.insert(&V3c::new(x, 0, z), 5.0).ok();
            }
        }

        let ray = Ray {
            origin: V3c {
                x: 2.0,
                y: 4.0,
                z: -2.0,
            },
            direction: V3c {
                x: -0.27100056,
                y: -0.7961219,
                z: 0.54106253,
            },
        };
        assert!(tree.get_by_ray(&ray).is_some());
        assert!(*tree.get_by_ray(&ray).unwrap().0 == 5.0);
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_edge_case_bottom_edge() {
        let mut tree = Octree::<f32>::new(4).ok().unwrap();

        for x in 0..4 {
            for z in 0..4 {
                tree.insert(&V3c::new(x, 0, z), 5.0).ok();
            }
        }

        let ray = Ray {
            origin: V3c {
                x: 2.0,
                y: 4.0,
                z: -2.0,
            },
            direction: V3c {
                x: -0.379010856,
                y: -0.822795153,
                z: 0.423507959,
            },
        };
        assert!(tree.get_by_ray(&ray).is_some());
        assert!(*tree.get_by_ray(&ray).unwrap().0 == 5.0);
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_edge_case_loop_stuck() {
        let mut tree = Octree::<f32>::new(4).ok().unwrap();
        tree.insert(&V3c::new(3, 0, 0), 0.).ok();
        tree.insert(&V3c::new(3, 3, 0), 1.).ok();
        tree.insert(&V3c::new(0, 3, 0), 2.).ok();

        for y in 0..4 {
            tree.insert(&V3c::new(0, y, y), 3.1).ok();
            tree.insert(&V3c::new(1, y, y), 3.2).ok();
            tree.insert(&V3c::new(2, y, y), 3.3).ok();
            tree.insert(&V3c::new(3, y, y), 3.4).ok();
        }

        let ray = Ray {
            origin: V3c {
                x: 0.024999974,
                y: 10.0,
                z: 0.0,
            },
            direction: V3c {
                x: -0.0030831057,
                y: -0.98595166,
                z: 0.16700225,
            },
        };
        let _ = tree.get_by_ray(&ray); //should not cause infinite loop
    }
}
