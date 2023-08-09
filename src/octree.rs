use crate::spatial::math::{hash_region, offset_region, V3c};
use crate::spatial::Cube;

pub enum Error {
    InvalidNodeSize(u32),
    InvalidPosition { x: u32, y: u32, z: u32 },
}

///####################################################################################
/// Node
///####################################################################################
#[derive(Default)]
struct Node<T>
where
    T: Default,
{
    bounds: Cube,
    content: Option<T>,
    children: [crate::object_pool::ItemKey; 8],
}

impl<T> Node<T>
where
    T: Default,
{
    /// Returns whether the `Node` contains the given position.
    pub(crate) fn contains(&self, position: &V3c<u32>) -> bool {
        position.x >= self.bounds.min_position.x
            && position.x <= self.bounds.min_position.x + self.bounds.size
            && position.y >= self.bounds.min_position.y
            && position.y <= self.bounds.min_position.y + self.bounds.size
            && position.z >= self.bounds.min_position.z
            && position.z <= self.bounds.min_position.z + self.bounds.size
    }

    /// Returns with the index of the child in the children array
    pub(crate) fn child_octant_for(&self, position: &V3c<u32>) -> usize {
        assert!(self.contains(position));
        hash_region(
            &(*position - self.bounds.min_position).into(),
            self.bounds.size as f32,
        )
    }

    /// Returns with the immediate child of it at the position, should there be one there
    pub(crate) fn child_at(&self, position: &V3c<u32>) -> ItemKey {
        self.children[self.child_octant_for(position)]
    }

    pub(crate) fn is_leaf(&self) -> bool {
        self.content.is_some()
    }

    pub(crate) fn bounds_at(&self, octant: usize) -> Cube {
        Cube::child_bounds_for(self.bounds.min_position, self.bounds.size, octant)
    }
}

///####################################################################################
/// Octree
///####################################################################################
use crate::object_pool::{ItemKey, ObjectPool};
pub struct Octree<Content>
where
    Content: Default,
{
    pub auto_simplify: bool,
    root_node: ItemKey,
    nodes: ObjectPool<Node<Content>>,
}

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
                bounds: Cube {
                    min_position: V3c::default(),
                    size,
                },
                ..Default::default()
            }),
            nodes,
        })
    }

    fn make_uniform_children(
        &mut self,
        min_position: V3c<u32>,
        child_size: u32,
        content: T,
    ) -> [ItemKey; 8] {
        [
            self.nodes.push(Node {
                bounds: Cube::child_bounds_for(min_position, child_size, 0),
                content: Some(content.clone()),
                ..Default::default()
            }),
            self.nodes.push(Node {
                bounds: Cube {
                    min_position: min_position + offset_region(1) * child_size,
                    size: child_size,
                },
                content: Some(content.clone()),
                ..Default::default()
            }),
            self.nodes.push(Node {
                bounds: Cube {
                    min_position: min_position + offset_region(2) * child_size,
                    size: child_size,
                },
                content: Some(content.clone()),
                ..Default::default()
            }),
            self.nodes.push(Node {
                bounds: Cube {
                    min_position: min_position + offset_region(3) * child_size,
                    size: child_size,
                },
                content: Some(content.clone()),
                ..Default::default()
            }),
            self.nodes.push(Node {
                bounds: Cube {
                    min_position: min_position + offset_region(4) * child_size,
                    size: child_size,
                },
                content: Some(content.clone()),
                ..Default::default()
            }),
            self.nodes.push(Node {
                bounds: Cube {
                    min_position: min_position + offset_region(5) * child_size,
                    size: child_size,
                },
                content: Some(content.clone()),
                ..Default::default()
            }),
            self.nodes.push(Node {
                bounds: Cube {
                    min_position: min_position + offset_region(6) * child_size,
                    size: child_size,
                },
                content: Some(content.clone()),
                ..Default::default()
            }),
            self.nodes.push(Node {
                bounds: Cube {
                    min_position: min_position + offset_region(7) * child_size,
                    size: child_size,
                },
                content: Some(content),
                ..Default::default()
            }),
        ]
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
            let current_size = self.nodes.get(current_node_key).bounds.size;
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
                    let current_node_min_position =
                        self.nodes.get(current_node_key).bounds.min_position;
                    if self.nodes.get(current_node_key).is_leaf()
                        && *self.nodes.get(current_node_key).content.as_ref().unwrap() != data
                    {
                        // The current Node is a leaf, which essentially represents an area where all the contained space have the same data.
                        // The contained data does not match the given data to set the position to, so all of the Nodes' children need to be created
                        // as separate Nodes with the same data as their parent to keep integrity
                        let current_content = self.nodes.get(current_node_key).content.clone();
                        let new_children = self.make_uniform_children(
                            current_node_min_position,
                            self.nodes.get(current_node_key).bounds.size,
                            current_content.unwrap(),
                        );
                        self.nodes.get_mut(current_node_key).content = None;
                        self.nodes.get_mut(current_node_key).children = new_children;
                        node_stack
                            .push(self.nodes.get(current_node_key).children[target_child_octant]);
                    } else {
                        // current Node is a non-leaf Node, which doesn't have the child at the requested position, so it is inserted
                        let child_size = &self.nodes.get(current_node_key).bounds.size / 2;
                        node_stack.push(self.nodes.push(Node {
                            bounds: Cube {
                                min_position: current_node_min_position
                                    + offset_region(target_child_octant) * child_size,
                                size: child_size,
                            },
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

    fn simplify(&mut self, node: &ItemKey) -> bool {
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
            let current_size = self.nodes.get(current_node_key).bounds.size;
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
                        let current_node_min_position =
                            self.nodes.get(current_node_key).bounds.min_position;
                        let current_content = self.nodes.get(current_node_key).content.clone();
                        let new_children = self.make_uniform_children(
                            current_node_min_position,
                            self.nodes.get(current_node_key).bounds.size,
                            current_content.unwrap(),
                        );
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

    /// provides the collision point of the ray with the contained voxel field
    /// return reference of the data, collision point and normal at impact, should there be any
    pub fn get_by_ray(&self, ray: &crate::spatial::Ray) -> Option<(&T, V3c<f32>, V3c<f32>)> {
        // println!("Root Node {:?}", self.node(&self.root_node).unwrap().bounds);
        // println!("Getting by {ray:?}");
        let mut current_d = 0.; // Current distance from the ray origin
        let mut last_hit; // The intersection of the ray with the currently examined node
        if let Some(hit) = self
            .node(&self.root_node)
            .unwrap()
            .bounds
            .intersect_ray(ray)
        {
            // println!("Intersects root");
            last_hit = hit;
            if let Some(entry_distance) = hit.impact_distance {
                // println!("..with entry point");
                current_d = entry_distance;
                if self.node(&self.root_node).unwrap().is_leaf() {
                    return Some((
                        self.node(&self.root_node)
                            .unwrap()
                            .content
                            .as_ref()
                            .unwrap(),
                        ray.point_at(entry_distance),
                        hit.impact_normal,
                    ));
                }
            }
            //Not having an entry distance means the ray starts inside the root node
        } else {
            return None;
        }

        let mut stack = vec![self.root_node];
        let mut advance_safeguard = 0;
        while !stack.is_empty() {
            let current_node = self.node(stack.last().unwrap()).unwrap();
            // println!("Current Node {:?}", current_node.bounds);
            // println!("Ray at {current_d:?}: {:?}", ray.point_at(current_d));
            if !current_node.bounds.contains_point(&ray.point_at(current_d)) {
                stack.pop();
                // println!("POP OOB");
                continue;
            }
            if current_node.is_leaf() {
                //the current node is a leaf, it's entry point distance was set in a previous loop in current_d
                // println!("HIT");
                return Some((
                    current_node.content.as_ref().unwrap(),
                    ray.point_at(if let Some(impact_distance) = last_hit.impact_distance {
                        impact_distance
                    } else {
                        // If there is no impact entry distance, then the ray is already inside bounds
                        0.
                    }),
                    last_hit.impact_normal,
                ));
            }

            // the child closest to the ray origin is revealed by the relative position
            // of the ray at the node entry to the node midpoint.
            // this guarantees that the child node bounds intersect with the ray
            let target_child_octant = hash_region(
                &(ray.point_at(current_d) - current_node.bounds.min_position.into()),
                current_node.bounds.size as f32,
            );
            assert!(target_child_octant < 8);
            // println!(
            //     "current octant offset base: {:?}",
            //     ray.point_at(current_d) - current_node.bounds.min_position.into()
            // );
            // println!(
            //     "target octant {target_child_octant:?} bounds: {:?}; target child key: {:?}",
            //     current_node.bounds_at(target_child_octant),
            //     current_node.children[target_child_octant]
            // );

            if let Some(hit) = current_node
                .bounds_at(target_child_octant)
                .intersect_ray(ray)
            {
                // println!(
                //     "Entry point: {:?}, exit point: (d:{:?}){:?}",
                //     if let Some(hit_point) = hit.impact_distance {
                //         format!("(d:{hit_point:?}){:?}", ray.point_at(hit_point))
                //     } else {
                //         "x".to_string()
                //     },
                //     hit.exit_distance,
                //     ray.point_at(hit.exit_distance)
                // );
                // println!("current node children: {:?}", current_node.children);
                let exit_correction = 0.0001;
                last_hit = hit;
                if current_node.children[target_child_octant].is_some() {
                    // There is a deeper level to explore! Update the ray as it shall march into this node
                    // If there's no impact distance(only exit distance) with the child bound, then the ray originates from it
                    // and no need to update current_d in that case
                    if let Some(impact_distance) = hit.impact_distance {
                        current_d = impact_distance;
                    }
                    // println!("PUSH to {current_d:?}");
                    stack.push(current_node.children[target_child_octant]);
                    advance_safeguard = 0;
                } else if current_node
                    .bounds
                    .contains_point(&ray.point_at(hit.exit_distance + exit_correction))
                    && advance_safeguard < 3
                // a little bit after exit distance to avoid node edges
                {
                    // the child node at the entry point of of the ray doesn't have content;
                    // but the current node still contains the ray after the exit point of it.
                    // Continue with the sibling based on the position after the current nodes exit point
                    //TODO: put a safeguard to advance!
                    // //ver 1
                    current_d = hit.exit_distance + exit_correction;
                    advance_safeguard += 1;
                    // // //ver 1 fix 1
                    // if let Some(impact_distance) = child_hit.impact_distance {
                    //     if impact_distance == child_hit.exit_distance {
                    //         // if the impact is at the edge of an inner boundary
                    //         current_d += exit_correction;
                    //         println!("edge correction...");
                    //     }
                    // }
                    // // ver 2 ( maybe needs ver 1 fix 1 )
                    // if let Some(impact_distance) = hit.impact_distance {
                    //     current_d += hit.exit_distance - impact_distance;
                    // } else {
                    //     current_d = hit.exit_distance + exit_correction
                    // }
                    // // ver 3?! idk anymore
                    // if let Some(impact_distance) = parent_hit.impact_distance {
                    //     current_d += (parent_hit.exit_distance - impact_distance) / 2.;
                    // } else {
                    //     current_d = child_hit.exit_distance;
                    // }
                    // println!("ADVANCE to {current_d:?}");
                } else {
                    // the current node doesn't contain the ray after the childs exit point
                    // so search needs to continue one level above
                    current_d = hit.exit_distance + exit_correction;
                    stack.pop();
                    advance_safeguard = 0;
                    // println!("POP");
                }
            } else {
                unreachable!();
            }
        }
        None // no node contained data intersecting with the ray
    }
}

///####################################################################################
/// Tests
///####################################################################################
#[cfg(test)]
mod octree_tests {
    use super::Octree;
    use crate::octree::Cube;
    use crate::octree::V3c;
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

    use rand::rngs::ThreadRng;
    use rand::Rng;
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
            // println!("element filled at {pos:?}");
            let ray = make_ray_point_to(&pos, &mut rng);
            assert!(tree.get(&pos.into()).is_some());
            assert!(tree.get_by_ray(&ray).is_some());
            assert!(*tree.get_by_ray(&ray).unwrap().0 == 5.0);
            // println!("=============\n=============\n=============\n=============\n=============\n");
        }
    }

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
        // println!("element filled at {:?}", V3c::new(3, 2, 2));
        assert!(tree.get_by_ray(&ray).is_some_and(|v| *v.0 == 3.4));
    }

    #[test]
    fn test_edge_case_ray_behind_octree() {
        let mut tree = Octree::<f32>::new(4).ok().unwrap();
        tree.insert(&V3c::new(0, 3, 0), 5.).ok();
        let origin = V3c::new(2., 2., -5.);
        let ray = Ray {
            direction: (V3c::new(0., 3., 0.) - origin).normalized(),
            origin,
        };
        println!("element filled at {:?}", &V3c::new(0, 3, 0));
        assert!(tree.get(&V3c::new(0, 3, 0)).is_some());
        assert!(*tree.get(&V3c::new(0, 3, 0)).unwrap() == 5.);
        assert!(tree.get_by_ray(&ray).is_some());
        assert!(*tree.get_by_ray(&ray).unwrap().0 == 5.);
    }

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

        //DEBUG
        let mut cube = Cube {
            min_position: V3c::unit(0),
            size: 4,
        };
        println!(
            "connection point: {:?}",
            if let Some(hit) = cube.intersect_ray(&ray) {
                if let Some(impact) = hit.impact_distance {
                    format!("entry: {:?}", ray.point_at(impact))
                } else {
                    format!("exit: {:?}", ray.point_at(hit.exit_distance))
                }
            } else {
                "x".to_string()
            }
        );
        cube = Cube {
            min_position: V3c::unit(0),
            size: 1,
        };
        // println!(
        //     "deepest connection point: {:?}",
        //     if let Some(hit) = cube.intersect_ray(&ray) {
        //         if let Some(impact) = hit.impact_distance {
        //             format!("entry: {:?}", ray.point_at(impact))
        //         } else {
        //             format!("exit: {:?}", ray.point_at(hit.exit_distance))
        //         }
        //     } else {
        //         "x".to_string()
        //     }
        // );
        let result = tree.get_by_ray(&ray);
        assert!(result.is_none() || *result.unwrap().0 == 5.0);
    }

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
}
