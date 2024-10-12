use crate::spatial::math::{
    hash_region, octant_bitmask, offset_region, set_occupancy_in_bitmap_64bits,
};
use crate::{
    object_pool::empty_marker,
    octree::{
        types::{Albedo, NodeChildren, NodeChildrenArray, NodeContent, Octree, VoxelData},
        BrickData, Cube, V3c,
    },
};

///####################################################################################
/// Utility functions
///####################################################################################

/// Returns whether the given bound contains the given position.
pub(crate) fn bound_contains(bounds: &Cube, position: &V3c<f32>) -> bool {
    position.x >= bounds.min_position.x
        && position.x < bounds.min_position.x + bounds.size
        && position.y >= bounds.min_position.y
        && position.y < bounds.min_position.y + bounds.size
        && position.z >= bounds.min_position.z
        && position.z < bounds.min_position.z + bounds.size
}

/// Returns with the octant value(i.e. index) of the child for the given position
pub(crate) fn child_octant_for(bounds: &Cube, position: &V3c<f32>) -> u8 {
    debug_assert!(bound_contains(bounds, position));
    hash_region(&(*position - bounds.min_position), bounds.size / 2.)
}

///####################################################################################
/// Type implements
///####################################################################################
impl VoxelData for Albedo {
    fn new(color: Albedo, _user_data: u32) -> Self {
        color
    }

    fn albedo(&self) -> Albedo {
        *self
    }

    fn user_data(&self) -> u32 {
        0u32
    }

    fn clear(&mut self) {
        self.r = 0;
        self.g = 0;
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

///####################################################################################
/// Octree
///####################################################################################
impl<T, const DIM: usize> Octree<T, DIM>
where
    T: Default + Clone + PartialEq + VoxelData,
{
    /// The root node is always the first item
    pub(crate) const ROOT_NODE_KEY: u32 = 0;
}

impl<T, const DIM: usize> Octree<T, DIM>
where
    T: Default + Clone + Copy + PartialEq + VoxelData,
{
    /// Provides an index value inside the brick contained in the given bounds
    /// Requires that position is larger, than the min_position of the bounds
    /// It takes into consideration the size of the bounds as well
    pub(crate) fn mat_index(bounds: &Cube, position: &V3c<u32>) -> V3c<usize> {
        // The position should be inside the bounds
        debug_assert!(
            bounds.min_position.x <= position.x as f32
                && bounds.min_position.y <= position.y as f32
                && bounds.min_position.z <= position.z as f32
                && bounds.min_position.x + bounds.size > position.x as f32
                && bounds.min_position.y + bounds.size > position.y as f32
                && bounds.min_position.z + bounds.size > position.z as f32
        );

        // --> In case the smallest possible node the contained matrix of voxels
        // starts at bounds min_position and ends in min_position + (DIM,DIM,DIM)
        // --> In case of bigger Nodes the below ratio equation is relevant
        // mat[xyz]/DIM = (position - min_position) / bounds.size
        let mat_index = (V3c::<usize>::from(*position - bounds.min_position.into()) * DIM)
            / bounds.size as usize;
        // The difference between the actual position and min bounds
        // must not be greater, than DIM at each dimension
        debug_assert!(mat_index.x < DIM);
        debug_assert!(mat_index.y < DIM);
        debug_assert!(mat_index.z < DIM);
        mat_index
    }

    /// Subdivides the node into multiple nodes. It guarantees that there will be a child at the target octant
    /// * `node_key` - The key of the node to subdivide. It must be a leaf
    /// * `target octant` - The octant that must have a child
    pub(crate) fn subdivide_leaf_to_nodes(&mut self, node_key: usize, target_octant: usize) {
        // Since the node is expected to be a leaf, by default it is supposed that it is fully occupied
        let mut node_content = NodeContent::Internal(0xFF);
        std::mem::swap(&mut node_content, self.nodes.get_mut(node_key));
        let mut node_new_children = [empty_marker(); 8];
        match node_content {
            NodeContent::Nothing | NodeContent::Internal(_) => {
                panic!("Non-leaf node expected to be Leaf")
            }
            NodeContent::Leaf(bricks) => {
                debug_assert!(
                    matches!(
                        self.node_children[node_key].content,
                        NodeChildrenArray::OccupancyBitmaps(_)
                    ),
                    "Expected OccupancyBitmaps instead of: {:?}",
                    self.node_children[node_key].content
                );

                // All contained bricks shall be converted to leaf nodes
                let node_children_occupied_bits =
                    if let NodeChildrenArray::OccupancyBitmaps(occupied_bits) =
                        self.node_children[node_key].content
                    {
                        occupied_bits
                    } else {
                        [0; 8]
                    };

                for octant in 0..8 {
                    match &bricks[octant] {
                        BrickData::Empty => {
                            if let NodeContent::Internal(occupied_bits) =
                                self.nodes.get_mut(node_key)
                            {
                                if octant != target_octant {
                                    // Reset the occupied bit for the node, as its child in this octant is empty
                                    *occupied_bits &= !octant_bitmask(octant as u8);
                                } else {
                                    // Push in an empty leaf child
                                    node_new_children[octant] =
                                        self.nodes.push(NodeContent::Nothing) as u32;
                                    self.node_children.resize(
                                        self.node_children
                                            .len()
                                            .max(node_new_children[octant] as usize + 1),
                                        NodeChildren::new(empty_marker()),
                                    );
                                    self.node_children[node_new_children[octant] as usize]
                                        .content = NodeChildrenArray::NoChildren;
                                }
                            }
                        }
                        BrickData::Parted(brick) => {
                            // Push in the new child
                            node_new_children[octant] = self
                                .nodes
                                .push(NodeContent::UniformLeaf(BrickData::Parted(brick.clone())))
                                as u32;
                            // Potentially Resize node children array to accomodate the new child
                            self.node_children.resize(
                                self.node_children
                                    .len()
                                    .max(node_new_children[octant] as usize + 1),
                                NodeChildren::new(empty_marker()),
                            );
                            // Set the occupancy bitmap for the new leaf child node
                            self.node_children[node_new_children[octant] as usize].content =
                                NodeChildrenArray::OccupancyBitmap(
                                    node_children_occupied_bits[octant],
                                );
                        }
                        BrickData::Solid(voxel) => {
                            node_new_children[octant] = self
                                .nodes
                                .push(NodeContent::UniformLeaf(BrickData::Solid(*voxel)))
                                as u32;
                            // Potentially Resize node children array to accomodate the new child
                            self.node_children.resize(
                                self.node_children
                                    .len()
                                    .max(node_new_children[octant] as usize + 1),
                                NodeChildren::new(empty_marker()),
                            );
                            debug_assert_eq!(
                                node_children_occupied_bits[octant], u64::MAX,
                                "Child should be all occupied if it has Solid Brickdata, instead it's {:?}",
                                node_children_occupied_bits[octant]
                            );

                            // Set the occupancy bitmap for the new leaf child node
                            self.node_children[node_new_children[octant] as usize].content =
                                NodeChildrenArray::OccupancyBitmap(u64::MAX);
                        }
                    };
                }
            }
            NodeContent::UniformLeaf(brick) => {
                // The leaf will be divided into 8 bricks, and the contents will be mapped from the current brick
                debug_assert!(
                    matches!(
                        self.node_children[node_key].content,
                        NodeChildrenArray::OccupancyBitmap(_)
                    ),
                    "Expected single OccupancyBitmap instead of: {:?}",
                    self.node_children[node_key].content
                );
                match brick {
                    BrickData::Empty => {
                        let mut new_occupied_bits = 0;

                        // Push in an empty leaf child to the target octant
                        node_new_children[target_octant] =
                            self.nodes.push(NodeContent::Nothing) as u32;
                        self.node_children.resize(
                            self.node_children
                                .len()
                                .max(node_new_children[target_octant] as usize + 1),
                            NodeChildren::new(empty_marker()),
                        );
                        self.node_children[node_new_children[target_octant] as usize].content =
                            NodeChildrenArray::NoChildren;

                        // Set the occupied bit for the node, as its child in this octant is not empty
                        new_occupied_bits |= octant_bitmask(target_octant as u8);
                        if let NodeContent::Internal(occupied_bits) = self.nodes.get_mut(node_key) {
                            *occupied_bits = new_occupied_bits;
                        }
                    }
                    BrickData::Solid(voxel) => {
                        for octant in 0..8 {
                            node_new_children[octant] = self
                                .nodes
                                .push(NodeContent::UniformLeaf(BrickData::Solid(voxel)))
                                as u32;
                            self.node_children.resize(
                                self.node_children
                                    .len()
                                    .max(node_new_children[octant] as usize + 1),
                                NodeChildren::new(empty_marker()),
                            );
                            self.node_children[node_new_children[octant] as usize].content =
                                NodeChildrenArray::OccupancyBitmap(u64::MAX);
                        }
                    }
                    BrickData::Parted(brick) => {
                        let mut node_children_occupied_bits = [0u64; 8];

                        // Each brick is mapped to take up one subsection of the current data
                        for octant in 0..8usize {
                            // Set the data of the new child
                            let brick_offset = V3c::<usize>::from(offset_region(octant as u8)) * 2;
                            let mut new_brick_data = Box::new(
                                [[[brick[brick_offset.x][brick_offset.y][brick_offset.z]; DIM];
                                    DIM]; DIM],
                            );
                            for x in 0..DIM {
                                for y in 0..DIM {
                                    for z in 0..DIM {
                                        set_occupancy_in_bitmap_64bits(
                                            x,
                                            y,
                                            z,
                                            DIM,
                                            !brick[x][y][z].is_empty(),
                                            &mut node_children_occupied_bits[octant],
                                        );
                                        if x < 2 && y < 2 && z < 2 {
                                            continue;
                                        }
                                        new_brick_data[x][y][z] = brick[brick_offset.x + x / 2]
                                            [brick_offset.y + y / 2][brick_offset.z + z / 2];
                                    }
                                }
                            }

                            // Push in the new child
                            node_new_children[octant] = self
                                .nodes
                                .push(NodeContent::UniformLeaf(BrickData::Parted(brick.clone())))
                                as u32;

                            // Potentially Resize node children array to accomodate the new child
                            self.node_children.resize(
                                self.node_children
                                    .len()
                                    .max(node_new_children[octant] as usize + 1),
                                NodeChildren::new(empty_marker()),
                            );
                            // Set the occupancy bitmap for the new leaf child node
                            self.node_children[node_new_children[octant] as usize].content =
                                NodeChildrenArray::OccupancyBitmap(
                                    node_children_occupied_bits[octant],
                                );
                        }
                    }
                }
            }
        }
        self.node_children[node_key].content = NodeChildrenArray::Children(node_new_children);
    }

    /// Erase all children of the node under the given key, and set its children to "No children"
    pub(crate) fn deallocate_children_of(&mut self, node: u32) {
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

    /// Calculates the occupied bits of a Node; For empty nodes(Nodecontent::Nothing) as well;
    /// As they might be empty by fault and to correct them the occupied bits is required.
    /// Leaf node occupancy bitmap should not be calculated by this function
    pub(crate) fn occupied_8bit(&self, node: u32) -> u8 {
        match self.nodes.get(node as usize) {
            NodeContent::Leaf(_) | NodeContent::UniformLeaf(_) => {
                match self.node_children[node as usize].content {
                    NodeChildrenArray::OccupancyBitmap(occupied_bits) => {
                        (((occupied_bits & 0x0000000000330033) > 0) as u8)
                            | (((occupied_bits & 0x0000000000cc00cc) > 0) as u8) << 1
                            | (((occupied_bits & 0x0033003300000000) > 0) as u8) << 2
                            | (((occupied_bits & 0x00cc00cc00000000) > 0) as u8) << 3
                            | (((occupied_bits & 0x0000000033003300) > 0) as u8) << 4
                            | (((occupied_bits & 0x00000000cc00cc00) > 0) as u8) << 5
                            | (((occupied_bits & 0x3300330000000000) > 0) as u8) << 6
                            | (((occupied_bits & 0xcc00cc0000000000) > 0) as u8) << 7
                    }
                    NodeChildrenArray::OccupancyBitmaps(occupied_bits) => {
                        (((occupied_bits[0]) > 0) as u8)
                            | (((occupied_bits[1]) > 0) as u8) << 1
                            | (((occupied_bits[2]) > 0) as u8) << 2
                            | (((occupied_bits[3]) > 0) as u8) << 3
                            | (((occupied_bits[4]) > 0) as u8) << 4
                            | (((occupied_bits[5]) > 0) as u8) << 5
                            | (((occupied_bits[6]) > 0) as u8) << 6
                            | (((occupied_bits[7]) > 0) as u8) << 7
                    }

                    _ => {
                        debug_assert!(false);
                        0
                    }
                }
            }
            _ => self.node_children[node as usize].occupied_bits(),
        }
    }
}
