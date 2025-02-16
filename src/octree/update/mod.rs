pub mod clear;
pub mod insert;

use crate::{
    object_pool::empty_marker,
    octree::{
        detail::bound_contains,
        types::{BrickData, NodeChildren, NodeContent, OctreeEntry, PaletteIndexValues},
        Albedo, Octree, VoxelData,
    },
    spatial::{
        lut::{BITMAP_MASK_FOR_OCTANT_LUT, OCTANT_OFFSET_REGION_LUT},
        math::{flat_projection, matrix_index_for, vector::V3c},
        Cube,
    },
};
use num_traits::Zero;
use std::hash::Hash;

#[cfg(feature = "bytecode")]
use bendy::{decoding::FromBencode, encoding::ToBencode};

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
    //####################################################################################
    // ███████████    █████████   █████       ██████████ ███████████ ███████████ ██████████
    // ░░███░░░░░███  ███░░░░░███ ░░███       ░░███░░░░░█░█░░░███░░░█░█░░░███░░░█░░███░░░░░█
    //  ░███    ░███ ░███    ░███  ░███        ░███  █ ░ ░   ░███  ░ ░   ░███  ░  ░███  █ ░
    //  ░██████████  ░███████████  ░███        ░██████       ░███        ░███     ░██████
    //  ░███░░░░░░   ░███░░░░░███  ░███        ░███░░█       ░███        ░███     ░███░░█
    //  ░███         ░███    ░███  ░███      █ ░███ ░   █    ░███        ░███     ░███ ░   █
    //  █████        █████   █████ ███████████ ██████████    █████       █████    ██████████
    // ░░░░░        ░░░░░   ░░░░░ ░░░░░░░░░░░ ░░░░░░░░░░    ░░░░░       ░░░░░    ░░░░░░░░░░
    //####################################################################################

    /// Updates the stored palette by adding the new colors and data in the given entry
    /// Since unused colors are not removed from the palette, possible "pollution" is possible,
    /// where unused colors remain in the palette.
    /// * Returns with the resulting PaletteIndexValues Entry
    pub(crate) fn add_to_palette(&mut self, entry: &OctreeEntry<T>) -> PaletteIndexValues {
        match entry {
            OctreeEntry::Empty => empty_marker::<PaletteIndexValues>(),
            OctreeEntry::Visual(albedo) => {
                if **albedo == Albedo::zero() {
                    return empty_marker();
                }
                let potential_new_albedo_index = self.map_to_color_index_in_palette.keys().len();
                let albedo_index = if let std::collections::hash_map::Entry::Vacant(e) =
                    self.map_to_color_index_in_palette.entry(**albedo)
                {
                    e.insert(potential_new_albedo_index);
                    self.voxel_color_palette.push(**albedo);
                    potential_new_albedo_index
                } else {
                    self.map_to_color_index_in_palette[albedo]
                };
                debug_assert!(
                    albedo_index < u16::MAX as usize,
                    "Albedo color palette overflow!"
                );
                NodeContent::pix_visual(albedo_index as u16)
            }
            OctreeEntry::Informative(data) => {
                if data.is_empty() {
                    return empty_marker();
                }
                let potential_new_data_index = self.map_to_data_index_in_palette.keys().len();
                let data_index = if let std::collections::hash_map::Entry::Vacant(e) =
                    self.map_to_data_index_in_palette.entry((*data).clone())
                {
                    e.insert(potential_new_data_index);
                    self.voxel_data_palette.push((*data).clone());
                    potential_new_data_index
                } else {
                    self.map_to_data_index_in_palette[data]
                };
                debug_assert!(
                    data_index < u16::MAX as usize,
                    "Data color palette overflow!"
                );
                NodeContent::pix_informal(data_index as u16)
            }
            OctreeEntry::Complex(albedo, data) => {
                if **albedo == Albedo::zero() {
                    return self.add_to_palette(&OctreeEntry::Informative(*data));
                } else if data.is_empty() {
                    return self.add_to_palette(&OctreeEntry::Visual(albedo));
                }
                let potential_new_albedo_index = self.map_to_color_index_in_palette.keys().len();
                let albedo_index = if let std::collections::hash_map::Entry::Vacant(e) =
                    self.map_to_color_index_in_palette.entry(**albedo)
                {
                    e.insert(potential_new_albedo_index);
                    self.voxel_color_palette.push(**albedo);
                    potential_new_albedo_index
                } else {
                    self.map_to_color_index_in_palette[albedo]
                };
                let potential_new_data_index = self.map_to_data_index_in_palette.keys().len();
                let data_index = if let std::collections::hash_map::Entry::Vacant(e) =
                    self.map_to_data_index_in_palette.entry((*data).clone())
                {
                    e.insert(potential_new_data_index);
                    self.voxel_data_palette.push((*data).clone());
                    potential_new_data_index
                } else {
                    self.map_to_data_index_in_palette[data]
                };
                debug_assert!(
                    albedo_index < u16::MAX as usize,
                    "Albedo color palette overflow!"
                );
                debug_assert!(
                    data_index < u16::MAX as usize,
                    "Data color palette overflow!"
                );
                NodeContent::pix_complex(albedo_index as u16, data_index as u16)
            }
        }
        // find color in the palette is present, add if not
    }

    //####################################################################################
    //  █████       ██████████   █████████   ███████████
    // ░░███       ░░███░░░░░█  ███░░░░░███ ░░███░░░░░░█
    //  ░███        ░███  █ ░  ░███    ░███  ░███   █ ░
    //  ░███        ░██████    ░███████████  ░███████
    //  ░███        ░███░░█    ░███░░░░░███  ░███░░░█
    //  ░███      █ ░███ ░   █ ░███    ░███  ░███  ░
    //  ███████████ ██████████ █████   █████ █████
    // ░░░░░░░░░░░ ░░░░░░░░░░ ░░░░░   ░░░░░ ░░░░░
    //  █████  █████ ███████████  ██████████     █████████   ███████████ ██████████
    // ░░███  ░░███ ░░███░░░░░███░░███░░░░███   ███░░░░░███ ░█░░░███░░░█░░███░░░░░█
    //  ░███   ░███  ░███    ░███ ░███   ░░███ ░███    ░███ ░   ░███  ░  ░███  █ ░
    //  ░███   ░███  ░██████████  ░███    ░███ ░███████████     ░███     ░██████
    //  ░███   ░███  ░███░░░░░░   ░███    ░███ ░███░░░░░███     ░███     ░███░░█
    //  ░███   ░███  ░███         ░███    ███  ░███    ░███     ░███     ░███ ░   █
    //  ░░████████   █████        ██████████   █████   █████    █████    ██████████
    //   ░░░░░░░░   ░░░░░        ░░░░░░░░░░   ░░░░░   ░░░░░    ░░░░░    ░░░░░░░░░░
    //####################################################################################
    /// Updates the given node to be a Leaf, and inserts the provided data for it.
    /// It will update a whole node, or maximum one brick. Brick update range is starting from the position,
    /// goes up to the extent of the brick. Does not set occupancy bitmap of the given node.
    /// * Returns with the size of the actual update
    pub(crate) fn leaf_update(
        &mut self,
        overwrite_if_empty: bool,
        node_key: usize,
        node_bounds: &Cube,
        target_bounds: &Cube,
        target_child_octant: usize,
        position: &V3c<u32>,
        size: u32,
        target_content: PaletteIndexValues,
    ) -> usize {
        // Update the leaf node, if it is possible as is, and if it's even needed to update
        // and decide if the node content needs to be divided into bricks, and the update function to be called again
        match self.nodes.get_mut(node_key) {
            NodeContent::Leaf(bricks) => {
                // In case brick_dimension == octree size, the root node can not be a leaf...
                debug_assert!(self.brick_dim < self.octree_size);
                match &mut bricks[target_child_octant] {
                    //If there is no brick in the target position of the leaf, create one
                    BrickData::Empty => {
                        // Create a new empty brick at the given octant
                        let mut new_brick = vec![
                            empty_marker::<PaletteIndexValues>();
                            (self.brick_dim * self.brick_dim * self.brick_dim)
                                as usize
                        ];
                        // update the new empty brick at the given position
                        let update_size = Self::update_brick(
                            overwrite_if_empty,
                            &mut new_brick,
                            target_bounds,
                            self.brick_dim,
                            *position,
                            size,
                            &target_content,
                        );
                        bricks[target_child_octant] = BrickData::Parted(new_brick);
                        update_size
                    }
                    BrickData::Solid(voxel) => {
                        // In case the data doesn't match the current contents of the node, it needs to be subdivided
                        let update_size;
                        if (NodeContent::pix_points_to_empty(
                            &target_content,
                            &self.voxel_color_palette,
                            &self.voxel_data_palette,
                        ) && !NodeContent::pix_points_to_empty(
                            voxel,
                            &self.voxel_color_palette,
                            &self.voxel_data_palette,
                        )) || (!NodeContent::pix_points_to_empty(
                            &target_content,
                            &self.voxel_color_palette,
                            &self.voxel_data_palette,
                        ) && *voxel != target_content)
                        {
                            // create new brick and update it at the given position
                            let mut new_brick = vec![
                                *voxel;
                                (self.brick_dim * self.brick_dim * self.brick_dim)
                                    as usize
                            ];
                            update_size = Self::update_brick(
                                overwrite_if_empty,
                                &mut new_brick,
                                target_bounds,
                                self.brick_dim,
                                *position,
                                size,
                                &target_content,
                            );
                            bricks[target_child_octant] = BrickData::Parted(new_brick);
                        } else {
                            // Since the Voxel already equals the data to be set, no need to update anything
                            update_size = 0;
                        }
                        update_size
                    }
                    BrickData::Parted(ref mut brick) => {
                        // Simply update the brick at the given position
                        Self::update_brick(
                            overwrite_if_empty,
                            brick,
                            target_bounds,
                            self.brick_dim,
                            *position,
                            size,
                            &target_content,
                        )
                    }
                }
            }
            NodeContent::UniformLeaf(ref mut mat) => {
                match mat {
                    BrickData::Empty => {
                        debug_assert_eq!(
                            self.node_children[node_key],
                            NodeChildren::OccupancyBitmap(0),
                            "Expected Node OccupancyBitmap(0) for empty leaf node instead of {:?}",
                            self.node_children[node_key]
                        );
                        if !NodeContent::pix_points_to_empty(
                            &target_content,
                            &self.voxel_color_palette,
                            &self.voxel_data_palette,
                        ) {
                            let mut new_leaf_content = [
                                BrickData::Empty,
                                BrickData::Empty,
                                BrickData::Empty,
                                BrickData::Empty,
                                BrickData::Empty,
                                BrickData::Empty,
                                BrickData::Empty,
                                BrickData::Empty,
                            ];

                            // Add a brick to the target octant and update with the given data
                            let mut new_brick = vec![
                                self.add_to_palette(&OctreeEntry::Empty);
                                (self.brick_dim * self.brick_dim * self.brick_dim)
                                    as usize
                            ];
                            let update_size = Self::update_brick(
                                overwrite_if_empty,
                                &mut new_brick,
                                target_bounds,
                                self.brick_dim,
                                *position,
                                size,
                                &target_content,
                            );
                            new_leaf_content[target_child_octant] = BrickData::Parted(new_brick);
                            *self.nodes.get_mut(node_key) = NodeContent::Leaf(new_leaf_content);
                            return update_size;
                        }
                    }
                    BrickData::Solid(voxel) => {
                        debug_assert!(
                            !NodeContent::pix_points_to_empty(voxel, &self.voxel_color_palette, &self.voxel_data_palette)
                                && (self.node_children[node_key]
                                    == NodeChildren::OccupancyBitmap(u64::MAX))
                                || NodeContent::pix_points_to_empty(voxel, &self.voxel_color_palette, &self.voxel_data_palette)
                                    && (self.node_children[node_key]
                                        == NodeChildren::OccupancyBitmap(0)),
                            "Expected Node occupancy bitmap({:?}) to align for Solid Voxel Brick in Uniform Leaf, which is {}",
                            self.node_children[node_key],
                            if NodeContent::pix_points_to_empty(voxel, &self.voxel_color_palette, &self.voxel_data_palette) {
                                "empty"
                            } else {
                                "not empty"
                            }
                        );

                        // In case the data request doesn't match node content, it needs to be subdivided
                        if NodeContent::pix_points_to_empty(
                            &target_content,
                            &self.voxel_color_palette,
                            &self.voxel_data_palette,
                        ) && NodeContent::pix_points_to_empty(
                            voxel,
                            &self.voxel_color_palette,
                            &self.voxel_data_palette,
                        ) {
                            // Data request is to clear, it aligns with the voxel content,
                            // it's enough to update the node content in this case
                            *self.nodes.get_mut(node_key) = NodeContent::Nothing;
                            return 0;
                        }

                        if !NodeContent::pix_points_to_empty(
                            &target_content,
                            &self.voxel_color_palette,
                            &self.voxel_data_palette,
                        ) && *voxel != target_content
                            || (NodeContent::pix_points_to_empty(
                                &target_content,
                                &self.voxel_color_palette,
                                &self.voxel_data_palette,
                            ) && !NodeContent::pix_points_to_empty(
                                voxel,
                                &self.voxel_color_palette,
                                &self.voxel_data_palette,
                            ))
                        {
                            // Data request doesn't align with the voxel data
                            // create a voxel brick and try to update with the given data
                            *mat = BrickData::Parted(vec![
                                *voxel;
                                (self.brick_dim * self.brick_dim * self.brick_dim)
                                    as usize
                            ]);

                            return self.leaf_update(
                                overwrite_if_empty,
                                node_key,
                                node_bounds,
                                target_bounds,
                                target_child_octant,
                                position,
                                size,
                                target_content,
                            );
                        }

                        // data request aligns with node content
                        return 0;
                    }
                    BrickData::Parted(brick) => {
                        // Check if the voxel at the target position matches with the data update request
                        // The target position index is to be calculated from the node bounds,
                        // instead of the target bounds because the position should cover the whole leaf
                        // not just one brick in it
                        let mat_index = matrix_index_for(node_bounds, position, self.brick_dim);
                        let mat_index = flat_projection(
                            mat_index.x,
                            mat_index.y,
                            mat_index.z,
                            self.brick_dim as usize,
                        );
                        if 1 < self.brick_dim // BrickData can only stay parted if brick_dimension is above 1
                            && (
                                (
                                    NodeContent::pix_points_to_empty(
                                        &target_content,
                                        &self.voxel_color_palette,
                                        &self.voxel_data_palette,
                                    )
                                    && NodeContent::pix_points_to_empty(
                                        &brick[mat_index],
                                        &self.voxel_color_palette,
                                        &self.voxel_data_palette
                                    )
                                )||(
                                    !NodeContent::pix_points_to_empty(
                                        &target_content,
                                        &self.voxel_color_palette,
                                        &self.voxel_data_palette,
                                    )
                                    && brick[mat_index] == target_content
                                )
                            )
                        {
                            // Target voxel matches with the data request, there's nothing to do!
                            return 0;
                        }

                        // the data at the position inside the brick doesn't match the given data,
                        // so the leaf needs to be divided into a NodeContent::Leaf(bricks)
                        let mut leaf_data: [BrickData<PaletteIndexValues>; 8] = [
                            BrickData::Empty,
                            BrickData::Empty,
                            BrickData::Empty,
                            BrickData::Empty,
                            BrickData::Empty,
                            BrickData::Empty,
                            BrickData::Empty,
                            BrickData::Empty,
                        ];

                        // Each brick is mapped to take up one subsection of the current data
                        let mut update_size = 0;
                        for octant in 0..8usize {
                            let brick_offset = V3c::<usize>::from(OCTANT_OFFSET_REGION_LUT[octant])
                                * (2.min(self.brick_dim as usize - 1));
                            let flat_brick_offset = flat_projection(
                                brick_offset.x,
                                brick_offset.y,
                                brick_offset.z,
                                self.brick_dim as usize,
                            );
                            let mut new_brick = vec![
                                brick[flat_brick_offset];
                                (self.brick_dim * self.brick_dim * self.brick_dim)
                                    as usize
                            ];
                            for x in 0..self.brick_dim as usize {
                                for y in 0..self.brick_dim as usize {
                                    for z in 0..self.brick_dim as usize {
                                        if x < 2 && y < 2 && z < 2 {
                                            continue;
                                        }
                                        let new_brick_flat_brick_offset = flat_projection(
                                            brick_offset.x,
                                            brick_offset.y,
                                            brick_offset.z,
                                            self.brick_dim as usize,
                                        );
                                        let flat_brick_offset = flat_projection(
                                            brick_offset.x + x / 2,
                                            brick_offset.y + y / 2,
                                            brick_offset.z + z / 2,
                                            self.brick_dim as usize,
                                        );
                                        new_brick[new_brick_flat_brick_offset] =
                                            brick[flat_brick_offset];
                                    }
                                }
                            }

                            // Also update the brick if it is the target
                            if octant == target_child_octant {
                                update_size = Self::update_brick(
                                    overwrite_if_empty,
                                    &mut new_brick,
                                    target_bounds,
                                    self.brick_dim,
                                    *position,
                                    size,
                                    &target_content,
                                );
                            }

                            leaf_data[octant] = BrickData::Parted(new_brick)
                        }

                        *self.nodes.get_mut(node_key) = NodeContent::Leaf(leaf_data);
                        debug_assert_ne!(
                            0, update_size,
                            "Expected Leaf node to be updated in operation"
                        );
                        return update_size;
                    }
                }
                self.leaf_update(
                    overwrite_if_empty,
                    node_key,
                    node_bounds,
                    target_bounds,
                    target_child_octant,
                    position,
                    size,
                    target_content,
                )
            }
            NodeContent::Nothing | NodeContent::Internal(_) => {
                // Warning: Calling leaf update to an internal node might induce data loss - see #69
                *self.nodes.get_mut(node_key) = NodeContent::Leaf([
                    self.try_brick_from_node(self.node_children[node_key].child(0) as usize),
                    self.try_brick_from_node(self.node_children[node_key].child(1) as usize),
                    self.try_brick_from_node(self.node_children[node_key].child(2) as usize),
                    self.try_brick_from_node(self.node_children[node_key].child(3) as usize),
                    self.try_brick_from_node(self.node_children[node_key].child(4) as usize),
                    self.try_brick_from_node(self.node_children[node_key].child(5) as usize),
                    self.try_brick_from_node(self.node_children[node_key].child(6) as usize),
                    self.try_brick_from_node(self.node_children[node_key].child(7) as usize),
                ]);
                self.deallocate_children_of(node_key);
                self.leaf_update(
                    overwrite_if_empty,
                    node_key,
                    node_bounds,
                    target_bounds,
                    target_child_octant,
                    position,
                    size,
                    target_content,
                )
            }
        }
    }

    /// Updates the content of the given brick and its occupancy bitmap. Each components of mat_index must be smaller, than the size of the brick.
    /// mat_index + size however need not be in bounds, the function will cut each component to fit inside the brick.
    /// * `brick` - mutable reference of the brick to update
    /// * `mat_index` - the first position to update with the given data
    /// * `size` - the number of elements in x,y,z to update with the given data
    /// * `data` - the data  to update the brick with. Erases data in case `None`
    /// * Returns with the size of the update
    fn update_brick(
        overwrite_if_empty: bool,
        brick: &mut [PaletteIndexValues],
        brick_bounds: &Cube,
        brick_dim: u32,
        position: V3c<u32>,
        size: u32,
        data: &PaletteIndexValues,
    ) -> usize {
        debug_assert!(
            bound_contains(brick_bounds, &(position.into())),
            "Expected position {:?} to be contained in brick bounds {:?}",
            position,
            brick_bounds
        );

        let mat_index = matrix_index_for(brick_bounds, &position, brick_dim);
        let update_size = (brick_dim as usize - mat_index.x).min(size as usize);
        for x in mat_index.x..(mat_index.x + size as usize).min(brick_dim as usize) {
            for y in mat_index.y..(mat_index.y + size as usize).min(brick_dim as usize) {
                for z in mat_index.z..(mat_index.z + size as usize).min(brick_dim as usize) {
                    let mat_index = flat_projection(x, y, z, brick_dim as usize);
                    if overwrite_if_empty {
                        brick[mat_index] = *data;
                    } else {
                        if NodeContent::pix_color_is_some(data) {
                            brick[mat_index] =
                                NodeContent::pix_overwrite_color(brick[mat_index], data);
                        }
                        if NodeContent::pix_data_is_some(data) {
                            brick[mat_index] =
                                NodeContent::pix_overwrite_data(brick[mat_index], data);
                        }
                    }
                }
            }
        }
        update_size
    }

    //####################################################################################
    //   █████████  █████ ██████   ██████ ███████████  █████       █████ ███████████ █████ █████
    //  ███░░░░░███░░███ ░░██████ ██████ ░░███░░░░░███░░███       ░░███ ░░███░░░░░░█░░███ ░░███
    // ░███    ░░░  ░███  ░███░█████░███  ░███    ░███ ░███        ░███  ░███   █ ░  ░░███ ███
    // ░░█████████  ░███  ░███░░███ ░███  ░██████████  ░███        ░███  ░███████     ░░█████
    //  ░░░░░░░░███ ░███  ░███ ░░░  ░███  ░███░░░░░░   ░███        ░███  ░███░░░█      ░░███
    //  ███    ░███ ░███  ░███      ░███  ░███         ░███      █ ░███  ░███  ░        ░███
    // ░░█████████  █████ █████     █████ █████        ███████████ █████ █████          █████
    //  ░░░░░░░░░  ░░░░░ ░░░░░     ░░░░░ ░░░░░        ░░░░░░░░░░░ ░░░░░ ░░░░░          ░░░░░
    //####################################################################################
    /// Updates the given node recursively to collapse nodes with uniform children into a leaf
    /// Returns with true if the given node was simplified
    pub(crate) fn simplify(&mut self, node_key: usize) -> bool {
        if self.nodes.key_is_valid(node_key) {
            #[cfg(debug_assertions)]
            {
                if let NodeContent::Internal(ocbits) = self.nodes.get(node_key) {
                    for octant in 1..8 {
                        if self
                            .nodes
                            .key_is_valid(self.node_children[node_key].child(octant))
                        {
                            debug_assert_ne!(
                                0,
                                *ocbits & BITMAP_MASK_FOR_OCTANT_LUT[octant as usize],
                                "Expected ocbits({:#10X}) to represent child at octant[{:?}]",
                                ocbits,
                                octant
                            )
                        }
                    }
                }
            }

            match self.nodes.get_mut(node_key) {
                NodeContent::Nothing => true,
                NodeContent::UniformLeaf(brick) => {
                    debug_assert!(
                        matches!(
                            self.node_children[node_key],
                            NodeChildren::OccupancyBitmap(_)
                        ),
                        "Uniform leaf has {:?} instead of an Occupancy_bitmap(_)",
                        self.node_children[node_key]
                    );
                    match brick {
                        BrickData::Empty => true,
                        BrickData::Solid(voxel) => {
                            if NodeContent::pix_points_to_empty(
                                voxel,
                                &self.voxel_color_palette,
                                &self.voxel_data_palette,
                            ) {
                                debug_assert_eq!(
                                    0,
                                    if let NodeChildren::OccupancyBitmap(occupied_bits) =
                                        self.node_children[node_key]
                                    {
                                        occupied_bits
                                    } else {
                                        0xD34D
                                    },
                                    "Solid empty voxel should have its occupied bits set to 0, instead of {:#10X}",
                                    if let NodeChildren::OccupancyBitmap(occupied_bits) =
                                        self.node_children[node_key]
                                    {
                                        occupied_bits
                                    } else {
                                        0xD34D
                                    }
                                );
                                *self.nodes.get_mut(node_key) = NodeContent::Nothing;
                                self.node_children[node_key] = NodeChildren::NoChildren;
                                true
                            } else {
                                debug_assert_eq!(
                                    u64::MAX,
                                    if let NodeChildren::OccupancyBitmap(occupied_bits) =
                                        self.node_children[node_key]
                                    {
                                        occupied_bits
                                    } else {
                                        0xD34D
                                    },
                                    "Solid full voxel should have its occupied bits set to u64::MAX, instead of {:#10X}",
                                    if let NodeChildren::OccupancyBitmap(occupied_bits) =
                                        self.node_children[node_key]
                                    {
                                        occupied_bits
                                    } else {
                                        0xD34D
                                    }
                                );
                                false
                            }
                        }
                        BrickData::Parted(_brick) => {
                            if brick.simplify(&self.voxel_color_palette, &self.voxel_data_palette) {
                                debug_assert!(
                                    self.node_children[node_key]
                                        == NodeChildren::OccupancyBitmap(u64::MAX)
                                        || self.node_children[node_key]
                                            == NodeChildren::OccupancyBitmap(0)
                                );
                                true
                            } else {
                                false
                            }
                        }
                    }
                }
                NodeContent::Leaf(bricks) => {
                    #[cfg(debug_assertions)]
                    {
                        for octant in 0..8 {
                            if let BrickData::Solid(_) | BrickData::Empty = bricks[octant] {
                                // with solid and empty bricks, the relevant occupied bits should either be empty or full
                                if let NodeChildren::OccupancyBitmap(occupied_bits) =
                                    self.node_children[node_key]
                                {
                                    // println!("node[{:?}] octant[{:?}]", node_key, octant);
                                    debug_assert!(
                                        0 == occupied_bits & BITMAP_MASK_FOR_OCTANT_LUT[octant]
                                            || BITMAP_MASK_FOR_OCTANT_LUT[octant]
                                                == occupied_bits
                                                    & BITMAP_MASK_FOR_OCTANT_LUT[octant],
                                        "Brickdata at octant[{:?}] doesn't match occupied bricks: {:?} <> ({:#10X} & {:#10X})",
                                        octant,
                                        bricks[octant],
                                        occupied_bits,
                                        BITMAP_MASK_FOR_OCTANT_LUT[octant]
                                    );
                                }
                            }
                        }
                    }

                    debug_assert!(
                        matches!(
                            self.node_children[node_key],
                            NodeChildren::OccupancyBitmap(_),
                        ),
                        "Expected node child to be OccupancyBitmap(_) instead of {:?}",
                        self.node_children[node_key]
                    );
                    bricks[0].simplify(&self.voxel_color_palette, &self.voxel_data_palette);
                    for octant in 1..8 {
                        bricks[octant]
                            .simplify(&self.voxel_color_palette, &self.voxel_data_palette);
                        if bricks[0] != bricks[octant] {
                            return false;
                        }
                    }

                    // Every matrix is the same! Make leaf uniform
                    *self.nodes.get_mut(node_key) = NodeContent::UniformLeaf(bricks[0].clone());
                    self.node_children[node_key] = NodeChildren::OccupancyBitmap(
                        if let NodeChildren::OccupancyBitmap(bitmaps) = self.node_children[node_key]
                        {
                            bitmaps
                        } else {
                            panic!("Leaf NodeContent should have OccupancyBitmap child assigned to it!");
                        },
                    );
                    self.simplify(node_key); // Try to collapse it to homogeneous node, but
                                             // irrespective of the results of it, return value is true,
                                             // because the node was updated already
                    true
                }
                NodeContent::Internal(ocbits) => {
                    if 0 == *ocbits
                        || matches!(self.node_children[node_key], NodeChildren::NoChildren)
                    {
                        if let NodeContent::Nothing = self.nodes.get(node_key) {
                            return false;
                        }

                        *self.nodes.get_mut(node_key) = NodeContent::Nothing;
                        return true;
                    }

                    debug_assert!(
                        matches!(self.node_children[node_key], NodeChildren::Children(_)),
                        "Expected Internal node to have Children instead of {:?}",
                        self.node_children[node_key]
                    );
                    let child_keys =
                        if let NodeChildren::Children(children) = self.node_children[node_key] {
                            children
                        } else {
                            return false;
                        };

                    // Try to simplify each child of the node
                    self.simplify(child_keys[0] as usize);

                    if !self.nodes.key_is_valid(child_keys[0] as usize) {
                        // At least try to simplify the siblings
                        for child_key in child_keys.iter().skip(1) {
                            self.simplify(*child_key as usize);
                        }
                        return false;
                    }

                    for octant in 1..8 {
                        self.simplify(child_keys[octant] as usize);
                        if !self.nodes.key_is_valid(child_keys[octant] as usize)
                            || !self
                                .nodes
                                .get(child_keys[0] as usize)
                                .compare(self.nodes.get(child_keys[octant] as usize))
                        {
                            return false;
                        }
                    }

                    // All children are the same!
                    // make the current node a leaf, erase the children
                    debug_assert!(matches!(
                        self.nodes.get(child_keys[0] as usize),
                        NodeContent::Leaf(_) | NodeContent::UniformLeaf(_)
                    ));
                    self.nodes.swap(node_key, child_keys[0] as usize);
                    // Deallocate children, and set correct occupancy bitmap
                    let new_node_children = self.node_children[child_keys[0] as usize];
                    self.deallocate_children_of(node_key);
                    self.node_children[node_key] = new_node_children;

                    // At this point there's no need to call simplify on the new leaf node
                    // because it's been attempted already on the data it copied from
                    true
                }
            }
        } else {
            // can't simplify node based on invalid key
            false
        }
    }
}
