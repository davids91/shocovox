pub mod raytracing;
pub mod bytecode;
pub mod detail;

use crate::octree::detail::{NodeContent, bound_contains, child_octant_for};
use crate::spatial::math::{V3c, hash_region, offset_region};
use crate::spatial::Cube;
use bendy::{decoding::FromBencode, encoding::ToBencode};



/// error types during usage or creation of the octree
pub enum OctreeError {
    InvalidNodeSize(u32),
    InvalidPosition { x: u32, y: u32, z: u32 },
}

use crate::object_pool::ObjectPool;
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Octree<T: Default + ToBencode + FromBencode> {
    pub auto_simplify: bool,
    root_node: usize,
    root_size: u32,
    nodes: ObjectPool<NodeContent<T>>, //None means the Node is an internal node, Some(...) means the Node is a leaf
    node_children: Vec<usize>,
}

impl<
        #[cfg(feature = "serialization")] T: Default + ToBencode + FromBencode + Serialize + DeserializeOwned,
        #[cfg(not(feature = "serialization"))] T: Default + ToBencode + FromBencode,
    > Octree<T>
where
    T: Default + PartialEq + Clone + std::fmt::Debug,
{
    /// converts the data structure to a byte representation
    pub fn to_bytes(&self) -> Vec<u8> {
        self.to_bencode().ok().unwrap()
    }

    /// parses the data structure from a byte string
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self::from_bencode(&bytes).ok().unwrap()
    }

    /// saves the data structure to the given file path
    pub fn save(&mut self, path: &str) -> Result<(), std::io::Error> {
        use std::fs::File;
        use std::io::Write;
        let mut file = File::create(path)?;
        file.write_all(&self.to_bytes())?;
        Ok(())
    }

    /// loads the data structure from the given file path
    pub fn load(path: &str) -> Result<Self, std::io::Error> {
        use std::fs::File;
        use std::io::Read;
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(Self::from_bytes(bytes))
    }

    /// creates an octree with the given sie which must be a multiple of 2
    pub fn new(size: u32) -> Result<Self, OctreeError> {
        if 0 == size || (size as f32).log(2.0).fract() != 0.0 {
            // Only multiples of two are valid sizes
            return Err(OctreeError::InvalidNodeSize(size));
        }
        let mut nodes = ObjectPool::<NodeContent<T>>::with_capacity(size.pow(3) as usize);
        let mut node_children = Vec::with_capacity((size.pow(3) * 8) as usize);
        node_children.resize(8, crate::object_pool::key_none_value());
        Ok(Self {
            auto_simplify: true,
            root_node: nodes.push(NodeContent::default()),
            root_size: size,
            nodes,
            node_children,
        })
    }

    /// Inserts the given data into the octree into the intended voxel position
    pub fn insert(&mut self, position: &V3c<u32>, data: T) -> Result<(), OctreeError> {
        self.insert_at_lod(position, 1, data)
    }

    /// Sets the given data for the octree in the given lod(level of detail) size
    pub fn insert_at_lod(
        &mut self,
        position: &V3c<u32>,
        min_node_size: u32,
        data: T,
    ) -> Result<(), OctreeError> {
        if 0 == min_node_size || (min_node_size as f32).log(2.0).fract() != 0.0 {
            // Only multiples of two are valid sizes
            return Err(OctreeError::InvalidNodeSize(min_node_size));
        }

        let root_bounds = Cube::root_bounds(self.root_size);
        if !bound_contains(&root_bounds, position) {
            return Err(OctreeError::InvalidPosition {
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
                    if self.nodes.get(current_node_key).is_leaf()
                        && *self.nodes.get(current_node_key).leaf_data() == data
                    {
                        // The current Node is a leaf, but the data stored equals the data to be set, so no need to go deeper as tha data already matches
                        break;
                    }
                    if self.nodes.get(current_node_key).is_leaf()
                        && *self.nodes.get(current_node_key).leaf_data() != data
                    {
                        // The current Node is a leaf, which essentially represents an area where all the contained space have the same data.
                        // The contained data does not match the given data to set the position to, so all of the Nodes' children need to be created
                        // as separate Nodes with the same data as their parent to keep integrity
                        let content = self.nodes.get(current_node_key).clone();
                        let new_children = self.make_uniform_children(content.leaf_data().clone());
                        *self.nodes.get_mut(current_node_key) = NodeContent::default();
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
                        let child_key = self.nodes.push(NodeContent::default());
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
                *self.nodes.get_mut(current_node_key) = NodeContent::Leaf(data);
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

    /// Provides immutable reference to the data, if there is any at the given position
    pub fn get(&self, position: &V3c<u32>) -> Option<&T> {
        let mut current_bounds = Cube::root_bounds(self.root_size);
        if !bound_contains(&current_bounds, position) {
            return None;
        }

        let mut current_node_key = self.root_node;
        loop {
            if self.nodes.get(current_node_key).is_leaf() {
                return self.nodes.get(current_node_key).as_leaf_ref();
            }
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

    /// Provides mutable reference to the data, if there is any at the given position
    pub fn get_mut(&mut self, position: &V3c<u32>) -> Option<&mut T> {
        let mut current_bounds = Cube::root_bounds(self.root_size);
        if !bound_contains(&current_bounds, position) {
            return None;
        }

        let mut current_node_key = self.root_node;
        loop {
            if self.nodes.get(current_node_key).is_leaf() {
                return self.nodes.get_mut(current_node_key).as_mut_leaf_ref();
            }
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

    /// clears the voxel at the given position
    pub fn clear(&mut self, position: &V3c<u32>) -> Result<(), OctreeError> {
        self.clear_at_lod(position, 1)
    }

    /// Clears the data at the given position and lod size
    pub fn clear_at_lod(
        &mut self,
        position: &V3c<u32>,
        min_node_size: u32,
    ) -> Result<(), OctreeError> {
        if 0 == min_node_size || (min_node_size as f32).log(2.0).fract() != 0.0 {
            // Only multiples of two are valid sizes
            return Err(OctreeError::InvalidNodeSize(min_node_size));
        }
        let root_bounds = Cube::root_bounds(self.root_size);
        if !bound_contains(&root_bounds, position) {
            return Err(OctreeError::InvalidPosition {
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
                    if self.nodes.get(current_node_key).is_leaf() {
                        // The current Node is a leaf, which essentially represents an area where all the contained space have the same data.
                        // The contained data does not match the given data to set the position to, so all of the Nodes' children need to be created
                        // as separate Nodes with the same data as their parent to keep integrity
                        let current_content = self.nodes.get(current_node_key);
                        assert!(current_content.is_leaf());
                        let new_children =
                            self.make_uniform_children(current_content.leaf_data().clone());
                        *self.nodes.get_mut(current_node_key) = NodeContent::Nothing;
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
        let mut tree = Octree::<u32>::new(2).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), 5).ok();
        tree.insert(&V3c::new(0, 1, 0), 6).ok();
        tree.insert(&V3c::new(0, 0, 1), 7).ok();

        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == 6);
        assert!(*tree.get(&V3c::new(0, 0, 1)).unwrap() == 7);
        assert!(tree.get(&V3c::new(1, 1, 1)).is_none());
    }

    #[test]
    fn test_get_mut() {
        let mut tree = Octree::<u32>::new(2).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), 5).ok();
        tree.insert(&V3c::new(0, 1, 0), 6).ok();
        tree.insert(&V3c::new(0, 0, 1), 7).ok();

        assert!(*tree.get_mut(&V3c::new(1, 0, 0)).unwrap() == 5);
        assert!(*tree.get_mut(&V3c::new(0, 1, 0)).unwrap() == 6);
        assert!(*tree.get_mut(&V3c::new(0, 0, 1)).unwrap() == 7);
        assert!(tree.get_mut(&V3c::new(1, 1, 1)).is_none());
    }

    #[test]
    fn test_insert_at_lod() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();
        tree.auto_simplify = false;

        // This will set the area equal to 8 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 2, 5).ok();

        assert!(*tree.get(&V3c::new(0, 0, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 0, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 0, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 1, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 1, 1)).unwrap() == 5);

        // This will set the area equal to 64 1-sized nodes:
        // a size-4 node includes 2 levels,
        // 1-sized nodes at the bottom level doesn't have children,
        // 2-sized nodes above have 8 children each
        // so one 4-sized node has 8*8 = 64 children
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, 1).ok();
        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    if tree.get(&V3c::new(x, y, z)).is_some()
                        && *tree.get(&V3c::new(x, y, z)).unwrap() == 1
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
        let mut tree = Octree::<u32>::new(4).ok().unwrap();

        // This will set the area equal to 8 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 2, 5).ok();

        assert!(*tree.get(&V3c::new(0, 0, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 0, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 0, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 1, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 1, 1)).unwrap() == 5);

        // This will set the area equal to 64 1-sized nodes:
        // a size-4 node includes 2 levels,
        // 1-sized nodes at the bottom level doesn't have children,
        // 2-sized nodes above have 8 children each
        // so one 4-sized node has 8*8 = 64 children
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, 1).ok();
        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    if tree.get(&V3c::new(x, y, z)).is_some()
                        && *tree.get(&V3c::new(x, y, z)).unwrap() == 1
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
        let mut tree = Octree::<u32>::new(2).ok().unwrap();

        // The below set of values should be simplified to a single node
        tree.insert(&V3c::new(0, 0, 0), 5).ok();
        tree.insert(&V3c::new(0, 0, 1), 5).ok();
        tree.insert(&V3c::new(0, 1, 0), 5).ok();
        tree.insert(&V3c::new(0, 1, 1), 5).ok();
        tree.insert(&V3c::new(1, 0, 0), 5).ok();
        tree.insert(&V3c::new(1, 0, 1), 5).ok();
        tree.insert(&V3c::new(1, 1, 0), 5).ok();
        tree.insert(&V3c::new(1, 1, 1), 5).ok();

        // The below should brake the simplified node back to its party
        tree.insert(&V3c::new(0, 0, 0), 4).ok();

        // Integrity should be kept
        assert!(*tree.get(&V3c::new(0, 0, 0)).unwrap() == 4);
        assert!(*tree.get(&V3c::new(0, 0, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 0, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 1, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 1, 1)).unwrap() == 5);
    }

    #[test]
    fn test_simple_clear() {
        let mut tree = Octree::<u32>::new(2).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), 5).ok();
        tree.insert(&V3c::new(0, 1, 0), 6).ok();
        tree.insert(&V3c::new(0, 0, 1), 7).ok();
        tree.clear(&V3c::new(0, 0, 1)).ok();

        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == 6);
        assert!(tree.get(&V3c::new(0, 0, 1)).is_none());
        assert!(tree.get(&V3c::new(1, 1, 1)).is_none());
    }

    #[test]
    fn test_simplifyable_clear() {
        let mut tree = Octree::<u32>::new(2).ok().unwrap();

        // The below set of values should be simplified to a single node
        tree.insert(&V3c::new(0, 0, 0), 5).ok();
        tree.insert(&V3c::new(0, 0, 1), 5).ok();
        tree.insert(&V3c::new(0, 1, 0), 5).ok();
        tree.insert(&V3c::new(0, 1, 1), 5).ok();
        tree.insert(&V3c::new(1, 0, 0), 5).ok();
        tree.insert(&V3c::new(1, 0, 1), 5).ok();
        tree.insert(&V3c::new(1, 1, 0), 5).ok();
        tree.insert(&V3c::new(1, 1, 1), 5).ok();

        // The below should brake the simplified node back to its party
        tree.clear(&V3c::new(0, 0, 0)).ok();

        // Integrity should be kept
        assert!(tree.get(&V3c::new(0, 0, 0)).is_none());
        assert!(*tree.get(&V3c::new(0, 0, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 0, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 1, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 1, 1)).unwrap() == 5);
    }

    #[test]
    fn test_clear_at_lod() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();

        // This will set the area equal to 64 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, 5).ok();

        // This will clear an area equal to 8 1-sized nodes
        tree.clear_at_lod(&V3c::new(0, 0, 0), 2).ok();

        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    if tree.get(&V3c::new(x, y, z)).is_some()
                        && *tree.get(&V3c::new(x, y, z)).unwrap() == 5
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
