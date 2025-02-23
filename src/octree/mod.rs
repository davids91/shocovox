pub mod types;
pub mod update;

mod detail;
mod node;

#[cfg(test)]
mod tests;

pub use crate::spatial::math::vector::{V3c, V3cf32};
pub use types::{Albedo, Octree, OctreeEntry, VoxelData};

use crate::{
    object_pool::{empty_marker, ObjectPool},
    octree::{
        detail::{bound_contains, child_octant_for},
        types::{BrickData, NodeChildren, NodeContent, OctreeError},
    },
    spatial::{
        lut::OOB_OCTANT,
        math::{flat_projection, matrix_index_for, BITMAP_DIMENSION},
        Cube,
    },
};
use num_traits::Zero;
use std::{collections::HashMap, hash::Hash};

#[cfg(feature = "serialization")]
use serde::{de::DeserializeOwned, Serialize};

#[cfg(feature = "bytecode")]
use bendy::{decoding::FromBencode, encoding::ToBencode};

#[cfg(debug_assertions)]
use crate::spatial::math::position_in_bitmap_64bits;

//####################################################################################
//     ███████      █████████  ███████████ ███████████   ██████████ ██████████
//   ███░░░░░███   ███░░░░░███░█░░░███░░░█░░███░░░░░███ ░░███░░░░░█░░███░░░░░█
//  ███     ░░███ ███     ░░░ ░   ░███  ░  ░███    ░███  ░███  █ ░  ░███  █ ░
// ░███      ░███░███             ░███     ░██████████   ░██████    ░██████
// ░███      ░███░███             ░███     ░███░░░░░███  ░███░░█    ░███░░█
// ░░███     ███ ░░███     ███    ░███     ░███    ░███  ░███ ░   █ ░███ ░   █
//  ░░░███████░   ░░█████████     █████    █████   █████ ██████████ ██████████
//    ░░░░░░░      ░░░░░░░░░     ░░░░░    ░░░░░   ░░░░░ ░░░░░░░░░░ ░░░░░░░░░░
//  ██████████ ██████   █████ ███████████ ███████████   █████ █████
// ░░███░░░░░█░░██████ ░░███ ░█░░░███░░░█░░███░░░░░███ ░░███ ░░███
//  ░███  █ ░  ░███░███ ░███ ░   ░███  ░  ░███    ░███  ░░███ ███
//  ░██████    ░███░░███░███     ░███     ░██████████    ░░█████
//  ░███░░█    ░███ ░░██████     ░███     ░███░░░░░███    ░░███
//  ░███ ░   █ ░███  ░░█████     ░███     ░███    ░███     ░███
//  ██████████ █████  ░░█████    █████    █████   █████    █████
// ░░░░░░░░░░ ░░░░░    ░░░░░    ░░░░░    ░░░░░   ░░░░░    ░░░░░
//####################################################################################
impl<'a, T: VoxelData> From<(&'a Albedo, &'a T)> for OctreeEntry<'a, T> {
    fn from((albedo, data): (&'a Albedo, &'a T)) -> Self {
        OctreeEntry::Complex(albedo, data)
    }
}

#[macro_export]
macro_rules! voxel_data {
    ($data:expr) => {
        OctreeEntry::Informative($data)
    };
    () => {
        OctreeEntry::Empty
    };
}

impl<'a, T: VoxelData> From<&'a Albedo> for OctreeEntry<'a, T> {
    fn from(albedo: &'a Albedo) -> Self {
        OctreeEntry::Visual(albedo)
    }
}

impl<'a, T: VoxelData> OctreeEntry<'a, T> {
    pub fn albedo(&self) -> Option<&'a Albedo> {
        match self {
            OctreeEntry::Empty => None,
            OctreeEntry::Visual(albedo) => Some(albedo),
            OctreeEntry::Informative(_) => None,
            OctreeEntry::Complex(albedo, _) => Some(albedo),
        }
    }

    pub fn data(&self) -> Option<&'a T> {
        match self {
            OctreeEntry::Empty => None,
            OctreeEntry::Visual(_) => None,
            OctreeEntry::Informative(data) => Some(data),
            OctreeEntry::Complex(_, data) => Some(data),
        }
    }

    pub fn is_none(&self) -> bool {
        match self {
            OctreeEntry::Empty => true,
            OctreeEntry::Visual(albedo) => **albedo == Albedo::zero(),
            OctreeEntry::Informative(data) => data.is_empty(),
            OctreeEntry::Complex(albedo, data) => **albedo == Albedo::zero() && data.is_empty(),
        }
    }

    pub fn is_some(&self) -> bool {
        !self.is_none()
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
impl<
        #[cfg(all(feature = "bytecode", feature = "serialization"))] T: FromBencode
            + ToBencode
            + Serialize
            + DeserializeOwned
            + Default
            + Eq
            + Clone
            + Hash
            + VoxelData,
        #[cfg(all(feature = "bytecode", not(feature = "serialization")))] T: FromBencode + ToBencode + Default + Eq + Clone + Hash + VoxelData,
        #[cfg(all(not(feature = "bytecode"), feature = "serialization"))] T: Serialize + DeserializeOwned + Default + Eq + Clone + Hash + VoxelData,
        #[cfg(all(not(feature = "bytecode"), not(feature = "serialization")))] T: Default + Eq + Clone + Hash + VoxelData,
    > Octree<T>
{
    /// converts the data structure to a byte representation
    #[cfg(feature = "bytecode")]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.to_bencode()
            .expect("Failed to serialize Octree to Bytes")
    }

    /// parses the data structure from a byte string
    #[cfg(feature = "bytecode")]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self::from_bencode(&bytes).expect("Failed to serialize Octree from Bytes")
    }

    /// saves the data structure to the given file path
    #[cfg(feature = "bytecode")]
    pub fn save(&self, path: &str) -> Result<(), std::io::Error> {
        use std::fs::File;
        use std::io::Write;
        let mut file = File::create(path)?;
        file.write_all(&self.to_bytes())?;
        Ok(())
    }

    /// loads the data structure from the given file path
    #[cfg(feature = "bytecode")]
    pub fn load(path: &str) -> Result<Self, std::io::Error> {
        use std::fs::File;
        use std::io::Read;
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(Self::from_bytes(bytes))
    }

    /// creates an octree with the given size
    /// * `brick_dimension` - must be one of `(2^x)` and smaller than the size of the octree
    /// * `size` - must be `brick_dimension * (2^x)`, e.g: brick_dimension == 2 --> size can be 2,4,8,16,32...
    pub fn new(size: u32, brick_dimension: u32) -> Result<Self, OctreeError> {
        if 0 == size || (brick_dimension as f32).log(2.0).fract() != 0.0 {
            return Err(OctreeError::InvalidBrickDimension(brick_dimension));
        }
        if brick_dimension > size
            || 0 == size
            || (size as f32 / brick_dimension as f32).log(2.0).fract() != 0.0
        {
            return Err(OctreeError::InvalidSize(size));
        }
        if brick_dimension >= size {
            return Err(OctreeError::InvalidStructure(
                "Octree size must be larger, than the brick dimension".into(),
            ));
        }
        let node_count_estimation = (size / brick_dimension).pow(3);
        let mut nodes = ObjectPool::with_capacity(node_count_estimation.min(1024) as usize);
        let root_node_key = nodes.push(NodeContent::Nothing); // The first element is the root Node
        assert!(root_node_key == 0);
        Ok(Self {
            auto_simplify: true,
            albedo_mip_maps: false,
            octree_size: size,
            brick_dim: brick_dimension,
            nodes,
            node_children: vec![NodeChildren::default()],
            node_mips: vec![BrickData::Empty],
            voxel_color_palette: vec![],
            voxel_data_palette: vec![],
            map_to_color_index_in_palette: HashMap::new(),
            map_to_data_index_in_palette: HashMap::new(),
        })
    }

    /// Getter function for the octree
    /// * Returns immutable reference to the data at the given position, if there is any
    pub fn get(&self, position: &V3c<u32>) -> OctreeEntry<T> {
        self.get_internal(
            Self::ROOT_NODE_KEY as usize,
            Cube::root_bounds(self.octree_size as f32),
            position,
        )
    }

    /// Internal Getter function for the octree, to be able to call get from within the tree itself
    /// * Returns immutable reference to the data of the given node at the given position, if there is any
    fn get_internal(
        &self,
        mut current_node_key: usize,
        mut current_bounds: Cube,
        position: &V3c<u32>,
    ) -> OctreeEntry<T> {
        let position = V3c::from(*position);
        if !bound_contains(&current_bounds, &position) {
            return OctreeEntry::Empty;
        }

        loop {
            match self.nodes.get(current_node_key) {
                NodeContent::Nothing => return OctreeEntry::Empty,
                NodeContent::Leaf(bricks) => {
                    // In case brick_dimension == octree size, the root node can not be a leaf...
                    debug_assert!(self.brick_dim < self.octree_size);

                    // Hash the position to the target child
                    let child_octant_at_position = child_octant_for(&current_bounds, &position);

                    // If the child exists, query it for the voxel
                    match &bricks[child_octant_at_position as usize] {
                        BrickData::Empty => {
                            return OctreeEntry::Empty;
                        }
                        BrickData::Parted(brick) => {
                            current_bounds =
                                Cube::child_bounds_for(&current_bounds, child_octant_at_position);
                            let mat_index = matrix_index_for(
                                &current_bounds,
                                &V3c::from(position),
                                self.brick_dim,
                            );
                            let mat_index = flat_projection(
                                mat_index.x as usize,
                                mat_index.y as usize,
                                mat_index.z as usize,
                                self.brick_dim as usize,
                            );
                            if !NodeContent::pix_points_to_empty(
                                &brick[mat_index],
                                &self.voxel_color_palette,
                                &self.voxel_data_palette,
                            ) {
                                return NodeContent::pix_get_ref(
                                    &brick[mat_index],
                                    &self.voxel_color_palette,
                                    &self.voxel_data_palette,
                                );
                            }
                            return OctreeEntry::Empty;
                        }
                        BrickData::Solid(voxel) => {
                            return NodeContent::pix_get_ref(
                                voxel,
                                &self.voxel_color_palette,
                                &self.voxel_data_palette,
                            );
                        }
                    }
                }
                NodeContent::UniformLeaf(brick) => match brick {
                    BrickData::Empty => {
                        return OctreeEntry::Empty;
                    }
                    BrickData::Parted(brick) => {
                        let mat_index =
                            matrix_index_for(&current_bounds, &V3c::from(position), self.brick_dim);
                        let mat_index = flat_projection(
                            mat_index.x as usize,
                            mat_index.y as usize,
                            mat_index.z as usize,
                            self.brick_dim as usize,
                        );
                        if NodeContent::pix_points_to_empty(
                            &brick[mat_index],
                            &self.voxel_color_palette,
                            &self.voxel_data_palette,
                        ) {
                            return OctreeEntry::Empty;
                        }
                        return NodeContent::pix_get_ref(
                            &brick[mat_index],
                            &self.voxel_color_palette,
                            &self.voxel_data_palette,
                        );
                    }
                    BrickData::Solid(voxel) => {
                        if NodeContent::pix_points_to_empty(
                            voxel,
                            &self.voxel_color_palette,
                            &self.voxel_data_palette,
                        ) {
                            return OctreeEntry::Empty;
                        }
                        return NodeContent::pix_get_ref(
                            voxel,
                            &self.voxel_color_palette,
                            &self.voxel_data_palette,
                        );
                    }
                },
                NodeContent::Internal(occupied_bits) => {
                    // Hash the position to the target child
                    let child_octant_at_position = child_octant_for(&current_bounds, &position);
                    let child_at_position =
                        self.node_children[current_node_key].child(child_octant_at_position);

                    // There is a valid child at the given position inside the node, recurse into it
                    if self.nodes.key_is_valid(child_at_position as usize) {
                        #[cfg(debug_assertions)]
                        {
                            // calculate the corresponding position in the nodes occupied bits
                            let pos_in_node = matrix_index_for(
                                &current_bounds,
                                &(position.into()),
                                BITMAP_DIMENSION as u32,
                            );

                            let should_bit_be_empty = self.should_bitmap_be_empty_at_position(
                                current_node_key,
                                &current_bounds,
                                &position,
                            );

                            let pos_in_bitmap = position_in_bitmap_64bits(&pos_in_node, 4);
                            let is_bit_empty = 0 == (occupied_bits & (0x01 << pos_in_bitmap));

                            // the corresponding bit should be set
                            debug_assert!(
                                 should_bit_be_empty == is_bit_empty,
                                  "Node[{:?}] under {:?} \n has a child(node[{:?}]) in octant[{:?}](global position: {:?}), which is incompatible with the occupancy bitmap: {:#10X};\nbecause: (should be empty: {} <> is empty: {})\n child node: {:?}; child node children: {:?};",
                                  current_node_key,
                                  current_bounds,
                                  self.node_children[current_node_key].child(child_octant_at_position),
                                  child_octant_at_position,
                                  position, occupied_bits,
                                  should_bit_be_empty, is_bit_empty,
                                  self.nodes.get(self.node_children[current_node_key].child(child_octant_at_position)),
                                  self.node_children[self.node_children[current_node_key].child(child_octant_at_position)]
                            );
                        }
                        current_node_key = child_at_position as usize;
                        current_bounds =
                            Cube::child_bounds_for(&current_bounds, child_octant_at_position);
                    } else {
                        return OctreeEntry::Empty;
                    }
                }
            }
        }
    }

    /// Tells the radius of the area covered by the octree
    pub fn get_size(&self) -> u32 {
        self.octree_size
    }

    /// Enables or disables mipmap feature for albedo values
    pub fn switch_albedo_mip_maps(&mut self, enabled: bool) {
        let mips_on_previously = self.albedo_mip_maps;
        self.albedo_mip_maps = enabled;

        // go through every node and set its mip-maps in case the feature is just enabled
        // and if there's anything to iterate into
        if !self.albedo_mip_maps
            || mips_on_previously == enabled
            || *self.nodes.get(Self::ROOT_NODE_KEY as usize) == NodeContent::Nothing
        {
            return;
        }

        self.node_mips = vec![BrickData::Empty; self.nodes.len()];
        // Generating MIPMAPs need to happen while traveling the graph in a DFS
        // in order to generate MIPs for the leaf nodes first
        let mut node_stack = vec![(
            Self::ROOT_NODE_KEY as usize,
            Cube::root_bounds(self.octree_size as f32),
            0,
        )];
        while !node_stack.is_empty() {
            let (current_node_key, current_bounds, target_octant) = node_stack.last().unwrap();

            // evaluate current node and return to its parent node
            if OOB_OCTANT == *target_octant {
                self.recalculate_mip(*current_node_key, current_bounds);
                node_stack.pop();
                if let Some(parent) = node_stack.last_mut() {
                    parent.2 += 1;
                }
                continue;
            }

            match self.nodes.get(*current_node_key) {
                NodeContent::Nothing => unreachable!("BFS shouldn't evaluate empty children"),
                NodeContent::Internal(_occupied_bits) => {
                    let target_child_key =
                        self.node_children[*current_node_key].child(*target_octant);
                    if self.nodes.key_is_valid(target_child_key)
                        && !matches!(self.nodes.get(target_child_key), NodeContent::Nothing)
                    {
                        debug_assert!(
                            matches!(
                                self.node_children[target_child_key],
                                NodeChildren::OccupancyBitmap(_) | NodeChildren::Children(_)
                            ),
                            "Expected node[{}] child[{}] to have children or occupancy instead of: {:?}",
                            current_node_key, target_octant, self.node_children[target_child_key]
                        );
                        node_stack.push((
                            target_child_key,
                            current_bounds.child_bounds_for(*target_octant),
                            0,
                        ));
                    } else {
                        node_stack.last_mut().unwrap().2 += 1;
                    }
                }
                NodeContent::Leaf(_) | NodeContent::UniformLeaf(_) => {
                    debug_assert!(
                        matches!(
                            self.node_children[*current_node_key],
                            NodeChildren::OccupancyBitmap(_)
                        ),
                        "Expected node[{}] to have occupancy bitmaps instead of: {:?}",
                        current_node_key,
                        self.node_children[*current_node_key]
                    );
                    // Set current child iterator to OOB, to evaluate it and move on
                    node_stack.last_mut().unwrap().2 = OOB_OCTANT;
                }
            }
        }
    }

    #[cfg(test)]
    /// Sample the MIP of the root node, or its children
    /// * `octant` - the child to sample, in case `OOB_OCTANT` the root MIP is sampled
    /// * `position` - the position inside the MIP, expected to be in range `0..self.brick_dim` for all components
    pub(crate) fn sample_root_mip(&self, octant: u8, position: &V3c<u32>) -> OctreeEntry<T> {
        let node_key = if OOB_OCTANT == octant {
            Self::ROOT_NODE_KEY as usize
        } else {
            self.node_children[Self::ROOT_NODE_KEY as usize].child(octant) as usize
        };

        if !self.nodes.key_is_valid(node_key) {
            return OctreeEntry::Empty;
        }
        match &self.node_mips[node_key] {
            BrickData::Empty => OctreeEntry::Empty,
            BrickData::Solid(voxel) => NodeContent::pix_get_ref(
                &voxel,
                &self.voxel_color_palette,
                &self.voxel_data_palette,
            ),
            BrickData::Parted(brick) => {
                let flat_index = flat_projection(
                    position.x as usize,
                    position.y as usize,
                    position.z as usize,
                    self.brick_dim as usize,
                );
                NodeContent::pix_get_ref(
                    &brick[flat_index],
                    &self.voxel_color_palette,
                    &self.voxel_data_palette,
                )
            }
        }
    }
}
