use crate::{
    object_pool::empty_marker,
    octree::{
        types::{Albedo, NodeChildren, NodeContent, Octree, PaletteIndexValues, VoxelData},
        BrickData, Cube, V3c,
    },
    spatial::{
        lut::OCTANT_OFFSET_REGION_LUT,
        math::{flat_projection, hash_region, BITMAP_DIMENSION},
    },
};
use num_traits::Zero;
use std::{hash::Hash, ops::Add};

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
    debug_assert!(
        bound_contains(bounds, position),
        "Position {:?}, out of {:?}",
        position,
        bounds
    );
    hash_region(&(*position - bounds.min_position), bounds.size / 2.)
}

impl<T: Zero + PartialEq> VoxelData for T {
    fn is_empty(&self) -> bool {
        *self == T::zero()
    }
}

//####################################################################################
//     █████████   █████       ███████████  ██████████ ██████████      ███████
//   ███░░░░░███ ░░███       ░░███░░░░░███░░███░░░░░█░░███░░░░███   ███░░░░░███
//  ░███    ░███  ░███        ░███    ░███ ░███  █ ░  ░███   ░░███ ███     ░░███
//  ░███████████  ░███        ░██████████  ░██████    ░███    ░███░███      ░███
//  ░███░░░░░███  ░███        ░███░░░░░███ ░███░░█    ░███    ░███░███      ░███
//  ░███    ░███  ░███      █ ░███    ░███ ░███ ░   █ ░███    ███ ░░███     ███
//  █████   █████ ███████████ ███████████  ██████████ ██████████   ░░░███████░
// ░░░░░   ░░░░░ ░░░░░░░░░░░ ░░░░░░░░░░░  ░░░░░░░░░░ ░░░░░░░░░░      ░░░░░░░
//####################################################################################

impl Albedo {
    pub fn with_red(mut self, r: u8) -> Self {
        self.r = r;
        self
    }

    pub fn with_green(mut self, g: u8) -> Self {
        self.g = g;
        self
    }

    pub fn with_blue(mut self, b: u8) -> Self {
        self.b = b;
        self
    }

    pub fn with_alpha(mut self, a: u8) -> Self {
        self.a = a;
        self
    }

    pub fn is_transparent(&self) -> bool {
        self.a == 0
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

impl Add for Albedo {
    type Output = Albedo;
    fn add(self, other: Albedo) -> Albedo {
        Albedo {
            r: self.r + other.r,
            g: self.g + other.g,
            b: self.b + other.b,
            a: self.a + other.a,
        }
    }
}

impl Zero for Albedo {
    fn zero() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }
    fn is_zero(&self) -> bool {
        self.is_empty()
    }
}

//####################################################################################
//     ███████      █████████  ███████████ ███████████   ██████████ ██████████
//   ███░░░░░███   ███░░░░░███░█░░░███░░░█░░███░░░░░███ ░░███░░░░░█░░███░░░░░█
//  ███     ░░███ ███     ░░░ ░   ░███  ░  ░███    ░███  ░███  █ ░  ░███  █ ░
// ░███      ░███░███             ░███     ░██████████   ░██████    ░██████
// ░███      ░███░███             ░███     ░███░░░░░███  ░███░░█    ░███░░█
// ░░███     ███ ░░███     ███    ░███     ░███    ░███  ░███ ░   █ ░███ ░   █
//  ░░░███████░   ░░█████████     █████    █████   █████ ██████████ ██████████
//    ░░░░░░░      ░░░░░░░░░     ░░░░░    ░░░░░   ░░░░░ ░░░░░░░░░░ ░░░░░░░░░░
//####################################################################################
impl<T> Octree<T>
where
    T: Default + Clone + Eq + Hash + VoxelData,
{
    /// The root node is always the first item
    pub(crate) const ROOT_NODE_KEY: u32 = 0;
}

impl<T> Octree<T>
where
    T: Default + Clone + Eq + Hash + VoxelData,
{
    /// Checks the content of the content of the node if it is empty at the given index,
    /// so the corresponding part of the occupied bits of the node can be set. The check targets
    /// the occupied bits, so it has a resolution of the occupied bit size.
    pub(crate) fn should_bitmap_be_empty_at_bitmap_index(
        &self,
        node_key: usize,
        index: &V3c<usize>,
    ) -> bool {
        let position = V3c::new(0.5, 0.5, 0.5) + (*index).into();
        let target_octant = hash_region(&position, BITMAP_DIMENSION as f32 / 2.);
        let target_octant_for_child = hash_region(
            &(position
                - (OCTANT_OFFSET_REGION_LUT[target_octant as usize] * BITMAP_DIMENSION as f32
                    / 2.)),
            BITMAP_DIMENSION as f32 / 4.,
        );

        self.should_bitmap_be_empty_at_octants(node_key, target_octant, target_octant_for_child)
    }

    /// Checks the content of the content of the node if it is empty at the given position,
    /// so the corresponding part of the occupied bits of the node can be set. The check targets
    /// the occupied bits, so it has a resolution of the occupied bit size.
    pub(crate) fn should_bitmap_be_empty_at_position(
        &self,
        node_key: usize,
        node_bounds: &Cube,
        position: &V3c<f32>,
    ) -> bool {
        let target_octant = hash_region(
            &(*position - node_bounds.min_position),
            node_bounds.size / 2.,
        );
        let target_octant_for_child = hash_region(
            &(*position - node_bounds.child_bounds_for(target_octant).min_position),
            node_bounds.size / 4.,
        );

        self.should_bitmap_be_empty_at_octants(node_key, target_octant, target_octant_for_child)
    }

    /// Checks the content of the content of the node at the given @target_octant,
    /// and the part of it under target_octant_for_child if it is empty, so the
    /// corresponding part of the occupied bits of the node can be set
    pub(crate) fn should_bitmap_be_empty_at_octants(
        &self,
        node_key: usize,
        target_octant: u8,
        target_octant_for_child: u8,
    ) -> bool {
        match self.nodes.get(node_key) {
            NodeContent::Nothing => true,
            NodeContent::Internal(_) => {
                let child_key = self.node_children[node_key].child(target_octant);
                if self.nodes.key_is_valid(child_key) {
                    self.node_empty_at(child_key, target_octant_for_child)
                } else {
                    true
                }
            }
            NodeContent::UniformLeaf(brick) => brick.is_part_empty_throughout(
                target_octant,
                target_octant_for_child,
                self.brick_dim as usize,
                &self.voxel_color_palette,
                &self.voxel_data_palette,
            ),
            NodeContent::Leaf(bricks) => bricks[target_octant as usize].is_empty_throughout(
                target_octant_for_child,
                self.brick_dim as usize,
                &self.voxel_color_palette,
                &self.voxel_data_palette,
            ),
        }
    }

    /// Returns with true if Node is empty at the given target octant. Uses occupied bits for Internal nodes.
    pub(crate) fn node_empty_at(&self, node_key: usize, target_octant: u8) -> bool {
        match self.nodes.get(node_key) {
            NodeContent::Nothing => true,
            NodeContent::Leaf(bricks) => match &bricks[target_octant as usize] {
                BrickData::Empty => true,
                BrickData::Solid(voxel) => NodeContent::pix_points_to_empty(
                    voxel,
                    &self.voxel_color_palette,
                    &self.voxel_data_palette,
                ),
                BrickData::Parted(_brick) => {
                    if let Some(data) = bricks[target_octant as usize].get_homogeneous_data() {
                        NodeContent::pix_points_to_empty(
                            data,
                            &self.voxel_color_palette,
                            &self.voxel_data_palette,
                        )
                    } else {
                        false
                    }
                }
            },
            NodeContent::UniformLeaf(brick) => match brick {
                BrickData::Empty => true,
                BrickData::Solid(voxel) => NodeContent::pix_points_to_empty(
                    voxel,
                    &self.voxel_color_palette,
                    &self.voxel_data_palette,
                ),
                BrickData::Parted(_brick) => {
                    if let Some(data) = brick.get_homogeneous_data() {
                        NodeContent::pix_points_to_empty(
                            data,
                            &self.voxel_color_palette,
                            &self.voxel_data_palette,
                        )
                    } else {
                        false
                    }
                }
            },
            NodeContent::Internal(_occupied_bits) => {
                debug_assert!(
                    !matches!(
                        self.node_children[node_key],
                        NodeChildren::OccupancyBitmap(_)
                    ),
                    "Expected for internal node to not have OccupancyBitmap as assigned child: {:?}",
                    self.node_children[node_key],
                );
                for child_octant in 0..8 {
                    let child_key = self.node_children[node_key].child(target_octant);
                    if self.nodes.key_is_valid(child_key)
                        && !self.node_empty_at(child_key, child_octant)
                    {
                        return false;
                    }
                }
                true
            }
        }
    }

    /// Subdivides the node into multiple nodes. It guarantees that there will be a child at the target octant
    /// * `node_key` - The key of the node to subdivide. It must be a leaf
    /// * `target octant` - The octant that must have a child
    pub(crate) fn subdivide_leaf_to_nodes(&mut self, node_key: usize, target_octant: usize) {
        // Since the node is expected to be a leaf, by default it is supposed that it is fully occupied
        let mut node_content = NodeContent::Internal(
            if let NodeChildren::OccupancyBitmap(occupied_bits) = self.node_children[node_key] {
                occupied_bits
            } else {
                panic!(
                    "Expected node to have OccupancyBitmap(_), instead of {:?}",
                    self.node_children[node_key]
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
                                    NodeChildren::default(),
                                );
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
                                NodeChildren::default(),
                            );

                            // Set the occupancy bitmap for the new leaf child node
                            self.node_children[node_new_children[octant] as usize] =
                                NodeChildren::OccupancyBitmap(u64::MAX);
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
                                NodeChildren::default(),
                            );

                            // Calculcate the occupancy bitmap for the new leaf child node
                            // As it is a higher resolution, than the current bitmap, it needs to be bruteforced
                            self.node_children[node_new_children[octant] as usize] =
                                NodeChildren::OccupancyBitmap(
                                    bricks[octant].calculate_occupied_bits(
                                        self.brick_dim as usize,
                                        &self.voxel_color_palette,
                                        &self.voxel_data_palette,
                                    ),
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
                            NodeChildren::default(),
                        );
                        self.node_children[node_new_children[target_octant] as usize] =
                            NodeChildren::OccupancyBitmap(0);
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
                                NodeChildren::default(),
                            );
                            self.node_children[node_new_children[octant] as usize] =
                                NodeChildren::OccupancyBitmap(u64::MAX);
                        }
                    }
                    BrickData::Parted(brick) => {
                        // Each brick is mapped to take up one subsection of the current data
                        for octant in 0..8usize {
                            // Set the data of the new child
                            let brick_offset =
                                V3c::<usize>::from(OCTANT_OFFSET_REGION_LUT[octant]) * 2;
                            let new_brick_flat_offset = flat_projection(
                                brick_offset.x,
                                brick_offset.y,
                                brick_offset.z,
                                self.brick_dim as usize,
                            );
                            let mut new_brick_data = vec![
                                brick[new_brick_flat_offset];
                                (self.brick_dim * self.brick_dim * self.brick_dim)
                                    as usize
                            ];
                            for x in 2..self.brick_dim as usize {
                                for y in 2..self.brick_dim as usize {
                                    for z in 2..self.brick_dim as usize {
                                        if x < 2 && y < 2 && z < 2 {
                                            continue;
                                        }
                                        let new_brick_flat_offset =
                                            flat_projection(x, y, z, self.brick_dim as usize);
                                        let brick_flat_offset = flat_projection(
                                            brick_offset.x + x / 2,
                                            brick_offset.y + y / 2,
                                            brick_offset.z + z / 2,
                                            self.brick_dim as usize,
                                        );
                                        new_brick_data[new_brick_flat_offset] =
                                            brick[brick_flat_offset];
                                    }
                                }
                            }

                            // Push in the new child
                            let child_occupied_bits = BrickData::calculate_brick_occupied_bits(
                                &new_brick_data,
                                self.brick_dim as usize,
                                &self.voxel_color_palette,
                                &self.voxel_data_palette,
                            );
                            node_new_children[octant] = self
                                .nodes
                                .push(NodeContent::UniformLeaf(BrickData::Parted(new_brick_data)))
                                as u32;

                            // Potentially Resize node children array to accomodate the new child
                            self.node_children.resize(
                                self.node_children
                                    .len()
                                    .max(node_new_children[octant] as usize + 1),
                                NodeChildren::default(),
                            );

                            // Set the occupancy bitmap for the new leaf child node
                            self.node_children[node_new_children[octant] as usize] =
                                NodeChildren::OccupancyBitmap(child_occupied_bits);
                        }
                    }
                }
            }
        }
        self.node_children[node_key] = NodeChildren::Children(node_new_children);
    }

    /// Tries to create a brick from the given node if possible. WARNING: Data loss may occur
    pub(crate) fn try_brick_from_node(&self, node_key: usize) -> BrickData<PaletteIndexValues> {
        if !self.nodes.key_is_valid(node_key) {
            return BrickData::Empty;
        }
        match self.nodes.get(node_key) {
            NodeContent::Nothing | NodeContent::Internal(_) | NodeContent::Leaf(_) => {
                BrickData::Empty
            }

            NodeContent::UniformLeaf(brick) => brick.clone(),
        }
    }

    /// Erase all children of the node under the given key, and set its children to "No children"
    pub(crate) fn deallocate_children_of(&mut self, node: usize) {
        if !self.nodes.key_is_valid(node) {
            return;
        }
        let mut to_deallocate = Vec::new();
        if let Some(children) = self.node_children[node].iter() {
            for child in children {
                if self.nodes.key_is_valid(*child as usize) {
                    to_deallocate.push(*child);
                }
            }
            for child in to_deallocate {
                self.deallocate_children_of(child as usize); // Recursion should be fine as depth is not expceted to be more, than 32
                self.nodes.free(child as usize);
            }
        }
        self.node_children[node] = NodeChildren::NoChildren;
    }

    /// Calculates the occupied bits of a Node; For empty nodes(Nodecontent::Nothing) as well;
    /// As they might be empty by fault and to correct them the occupied bits is required.
    pub(crate) fn stored_occupied_bits(&self, node_key: usize) -> u64 {
        match self.nodes.get(node_key) {
            NodeContent::Leaf(_) | NodeContent::UniformLeaf(_) => {
                match self.node_children[node_key] {
                    NodeChildren::OccupancyBitmap(occupied_bits) => occupied_bits,
                    NodeChildren::NoChildren => 0,
                    NodeChildren::Children(children) => {
                        debug_assert!(
                            false,
                            "Expected node[{node_key}] to not have children.\nnode:{:?}\nchildren: {:?}",
                            self.nodes.get(node_key),
                            children
                        );
                        0
                    }
                }
            }
            NodeContent::Nothing => 0,
            NodeContent::Internal(occupied_bits) => *occupied_bits,
        }
    }

    /// Stores the given occupied bits for the given node based on key
    pub(crate) fn store_occupied_bits(&mut self, node_key: usize, new_occupied_bits: u64) {
        match self.nodes.get_mut(node_key) {
            NodeContent::Internal(occupied_bits) => *occupied_bits = new_occupied_bits,
            NodeContent::Nothing => {
                self.node_children[node_key] = NodeChildren::OccupancyBitmap(new_occupied_bits)
            }
            NodeContent::Leaf(_) | NodeContent::UniformLeaf(_) => {
                match self.node_children[node_key] {
                    NodeChildren::NoChildren => {
                        self.node_children[node_key] =
                            NodeChildren::OccupancyBitmap(new_occupied_bits)
                    }
                    NodeChildren::OccupancyBitmap(ref mut occupied_bits) => {
                        *occupied_bits = new_occupied_bits;
                    }
                    NodeChildren::Children(_) => panic!(
                        "Expected Leaf node to have OccupancyBitmap instead of {:?}",
                        self.node_children[node_key]
                    ),
                }
            }
        }
    }
}
