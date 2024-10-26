use crate::spatial::math::{hash_region, mask_for_octant_64_bits, offset_region};
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

    pub(crate) const BITMAP_SIZE: f32 = 4.;

    /// how long is one brick index step in a 4x4x4 bitmap space
    pub(crate) const BRICK_UNIT_IN_BITMAP_SPACE: f32 = Self::BITMAP_SIZE / DIM as f32;

    /// how long is one bitmap index step in a DIMxDIMxDIM space
    pub(crate) const BITMAP_UNIT_IN_BRICK_SPACE: f32 = DIM as f32 / Self::BITMAP_SIZE;
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

    pub(crate) fn node_empty_at(&self, node_key: usize, target_octant: usize) -> bool {
        match self.nodes.get(node_key) {
            NodeContent::Nothing => true,
            NodeContent::Leaf(bricks) => match &bricks[target_octant] {
                BrickData::Empty => true,
                BrickData::Solid(voxel) => voxel.is_empty(),
                BrickData::Parted(_brick) => {
                    if let Some(data) = bricks[target_octant].get_homogeneous_data() {
                        data.is_empty()
                    } else {
                        false
                    }
                }
            },
            NodeContent::UniformLeaf(brick) => match brick {
                BrickData::Empty => true,
                BrickData::Solid(voxel) => voxel.is_empty(),
                BrickData::Parted(_brick) => {
                    if let Some(data) = brick.get_homogeneous_data() {
                        data.is_empty()
                    } else {
                        false
                    }
                }
            },
            NodeContent::Internal(occupied_bits) => {
                debug_assert!(
                    !matches!(
                        self.node_children[node_key].content,
                        NodeChildrenArray::OccupancyBitmap(_)
                    ),
                    "Expected for internal node to not have OccupancyBitmap as assigned child: {:?}",
                    self.node_children[node_key].content,
                );

                if let NodeChildrenArray::Children(children) = self.node_children[node_key].content
                {
                    debug_assert!(
                        (self.nodes.key_is_valid(children[target_octant] as usize)
                            && 0 != occupied_bits & mask_for_octant_64_bits(target_octant as u8))
                            || (!self.nodes.key_is_valid(children[target_octant] as usize)
                                && 0 == occupied_bits
                                    & mask_for_octant_64_bits(target_octant as u8)),
                            "Expected occupancy bitmap({:?}) to align with child content({:?}) at octant({:?})!",
                            occupied_bits, children, target_octant
                    );
                    return self.nodes.key_is_valid(children[target_octant] as usize);
                }
                debug_assert!(matches!(
                    self.node_children[node_key].content,
                    NodeChildrenArray::NoChildren
                ));
                false
            }
        }
    }

    /// Subdivides the node into multiple nodes. It guarantees that there will be a child at the target octant
    /// * `node_key` - The key of the node to subdivide. It must be a leaf
    /// * `target octant` - The octant that must have a child
    pub(crate) fn subdivide_leaf_to_nodes(&mut self, node_key: usize, target_octant: usize) {
        // Since the node is expected to be a leaf, by default it is supposed that it is fully occupied
        let mut node_content = NodeContent::Internal(
            if let NodeChildrenArray::OccupancyBitmap(occupied_bits) =
                self.node_children[node_key].content
            {
                occupied_bits
            } else {
                panic!(
                    "Expected node to have OccupancyBitmap(_), instead of {:?}",
                    self.node_children[node_key].content
                )
            },
        );
        std::mem::swap(&mut node_content, self.nodes.get_mut(node_key));
        let mut node_new_children = [empty_marker(); 8];
        match node_content {
            NodeContent::Nothing | NodeContent::Internal(_) => {
                panic!("Non-leaf node expected to be Leaf")
            }
            NodeContent::Leaf(mut bricks) => {
                // All contained bricks shall be converted to leaf nodes
                for octant in 0..8 {
                    let mut brick = BrickData::Empty;
                    std::mem::swap(&mut brick, &mut bricks[octant]);
                    match brick {
                        BrickData::Empty => {
                            if octant == target_octant {
                                // Push in an empty leaf child
                                node_new_children[octant] =
                                    self.nodes.push(NodeContent::Nothing) as u32;
                                self.node_children.resize(
                                    self.node_children
                                        .len()
                                        .max(node_new_children[octant] as usize + 1),
                                    NodeChildren::new(empty_marker()),
                                );
                                self.node_children[node_new_children[octant] as usize].content =
                                    NodeChildrenArray::NoChildren;
                            }
                        }
                        BrickData::Solid(voxel) => {
                            node_new_children[octant] = self
                                .nodes
                                .push(NodeContent::UniformLeaf(BrickData::Solid(voxel)))
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
                                NodeChildrenArray::OccupancyBitmap(u64::MAX);
                        }
                        BrickData::Parted(brick) => {
                            // Calculcate the occupancy bitmap for the new leaf child node
                            // As it is a higher resolution, than the current bitmap, it needs to be bruteforced
                            self.node_children[node_new_children[octant] as usize].content =
                                NodeChildrenArray::OccupancyBitmap(
                                    bricks[octant].calculate_occupied_bits(),
                                );

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
                        }
                    };
                }
            }
            NodeContent::UniformLeaf(brick) => {
                // The leaf will be divided into 8 bricks, and the contents will be mapped from the current brick
                match brick {
                    BrickData::Empty => {
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
                            NodeChildrenArray::OccupancyBitmap(0);
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
                                        if x < 2 && y < 2 && z < 2 {
                                            continue;
                                        }
                                        new_brick_data[x][y][z] = brick[brick_offset.x + x / 2]
                                            [brick_offset.y + y / 2][brick_offset.z + z / 2];
                                    }
                                }
                            }

                            // Push in the new child
                            let child_occupied_bits =
                                BrickData::<T, DIM>::calculate_brick_occupied_bits(&new_brick_data);
                            node_new_children[octant] = self
                                .nodes
                                .push(NodeContent::UniformLeaf(BrickData::Parted(new_brick_data)))
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
                                NodeChildrenArray::OccupancyBitmap(child_occupied_bits);
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
}
