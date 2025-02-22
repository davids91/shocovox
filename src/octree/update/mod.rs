pub mod clear;
pub mod insert;

#[cfg(test)]
mod tests;

use crate::{
    object_pool::empty_marker,
    octree::{
        detail::bound_contains,
        types::{BrickData, NodeChildren, NodeContent, OctreeEntry, PaletteIndexValues},
        Albedo, Octree, VoxelData,
    },
    spatial::{
        lut::{BITMAP_MASK_FOR_OCTANT_LUT, OCTANT_OFFSET_REGION_LUT},
        math::{flat_projection, hash_region, matrix_index_for, vector::V3c},
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
                // In case brick_dimension == octree size, the 0 can not be a leaf...
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
                    self.try_brick_from_node(self.node_children[node_key].child(0)),
                    self.try_brick_from_node(self.node_children[node_key].child(1)),
                    self.try_brick_from_node(self.node_children[node_key].child(2)),
                    self.try_brick_from_node(self.node_children[node_key].child(3)),
                    self.try_brick_from_node(self.node_children[node_key].child(4)),
                    self.try_brick_from_node(self.node_children[node_key].child(5)),
                    self.try_brick_from_node(self.node_children[node_key].child(6)),
                    self.try_brick_from_node(self.node_children[node_key].child(7)),
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

    //####################################################################################
    //  ██████   ██████ █████ ███████████     ██████   ██████   █████████   ███████████   █████████
    // ░░██████ ██████ ░░███ ░░███░░░░░███   ░░██████ ██████   ███░░░░░███ ░░███░░░░░███ ███░░░░░███
    //  ░███░█████░███  ░███  ░███    ░███    ░███░█████░███  ░███    ░███  ░███    ░███░███    ░░░
    //  ░███░░███ ░███  ░███  ░██████████     ░███░░███ ░███  ░███████████  ░██████████ ░░█████████
    //  ░███ ░░░  ░███  ░███  ░███░░░░░░      ░███ ░░░  ░███  ░███░░░░░███  ░███░░░░░░   ░░░░░░░░███
    //  ░███      ░███  ░███  ░███            ░███      ░███  ░███    ░███  ░███         ███    ░███
    //  █████     █████ █████ █████           █████     █████ █████   █████ █████       ░░█████████
    // ░░░░░     ░░░░░ ░░░░░ ░░░░░           ░░░░░     ░░░░░ ░░░░░   ░░░░░ ░░░░░         ░░░░░░░░░
    //####################################################################################
    /// Provides an average color value for the given range calculated from the sampling function
    /// * `sample_start` - The start position of the range to sample from
    /// * `sample_size` - The size of the range to sample from
    /// * `sample_fn` - The function providing the samples. It will be called on each position given by the range
    fn sample_from<F: Fn(&V3c<u32>) -> Option<Albedo>>(
        sample_start: &V3c<u32>,
        sample_size: u32,
        sample_fn: F,
    ) -> Option<Albedo> {
        // Calculate average albedo in the sampling range
        let mut avg_albedo = None;
        let mut entry_count = 0;
        for x in sample_start.x..(sample_start.x + sample_size) {
            for y in sample_start.y..(sample_start.y + sample_size) {
                for z in sample_start.z..(sample_start.z + sample_size) {
                    match (&mut avg_albedo, sample_fn(&V3c::new(x, y, z))) {
                        (None, Some(new_albedo)) => {
                            entry_count += 1;
                            avg_albedo = Some((
                                new_albedo.r as f32,
                                new_albedo.g as f32,
                                new_albedo.b as f32,
                                new_albedo.a as f32,
                            ));
                        }
                        (Some(ref mut current_avg_albedo), Some(new_albedo)) => {
                            entry_count += 1;
                            current_avg_albedo.0 += new_albedo.r as f32;
                            current_avg_albedo.1 += new_albedo.g as f32;
                            current_avg_albedo.2 += new_albedo.b as f32;
                            current_avg_albedo.3 += new_albedo.a as f32;
                        }
                        (None, None) | (Some(_), None) => {}
                    }
                }
            }
        }

        if let Some(albedo) = avg_albedo {
            debug_assert_ne!(0, entry_count, "Expected to have non-zero entries in MIP");
            let r = (albedo.0 / entry_count as f32).min(255.) as u8;
            let g = (albedo.1 / entry_count as f32).min(255.) as u8;
            let b = (albedo.2 / entry_count as f32).min(255.) as u8;
            let a = (albedo.3 / entry_count as f32).min(255.) as u8;
            Some(Albedo { r, g, b, a })
        } else {
            None
        }
    }

    /// Updates the MIP for the given node at the given position. It expects that MIPS of child nodes are up-to-date.
    /// * `node_key` - The node to update teh MIP for
    /// * `node_bounds` - The bounds of the target node
    /// * `position` - The global position in alignment with node bounds
    pub(crate) fn update_mip(&mut self, node_key: usize, node_bounds: &Cube, position: &V3c<u32>) {
        if !self.albedo_mip_maps {
            return;
        }

        debug_assert_eq!(
            0,
            node_bounds.size as u32 % self.brick_dim,
            "Expected node bounds to be the multiple of DIM"
        );

        let mip_entry = match self.nodes.get(node_key) {
            NodeContent::Nothing => {
                debug_assert_eq!(
                    NodeChildren::NoChildren,
                    self.node_children[node_key],
                    "Expected empty node to not have children!"
                );
                None
            }
            NodeContent::UniformLeaf(_brick) => {
                if !matches!(self.node_mips[node_key], BrickData::Empty) {
                    //Uniform Leaf nodes need not a MIP, because their content is equivalent with it
                    self.node_mips[node_key] = BrickData::Empty;
                }
                None
            }
            NodeContent::Leaf(_bricks) => {
                // determine the sampling range
                let sample_size =
                    (node_bounds.size as u32 / self.brick_dim).min(self.brick_dim * 2);
                let sample_start =
                    V3c::from((*position - (*position % sample_size)) * 2 * self.brick_dim)
                        / node_bounds.size;
                let sample_start: V3c<u32> = sample_start.floor().into();
                debug_assert!(
                    sample_start.x + sample_size
                        <= (node_bounds.min_position.x + node_bounds.size) as u32,
                    "Mipmap sampling out of bounds for x component: ({} + {}) > ({} + {})",
                    sample_start.x,
                    sample_size,
                    node_bounds.min_position.x,
                    node_bounds.size
                );
                debug_assert!(
                    sample_start.y + sample_size
                        <= (node_bounds.min_position.y + node_bounds.size) as u32,
                    "Mipmap sampling out of bounds for y component: ({} + {}) > ({} + {})",
                    sample_start.y,
                    sample_size,
                    node_bounds.min_position.y,
                    node_bounds.size
                );
                debug_assert!(
                    sample_start.z + sample_size
                        <= (node_bounds.min_position.z + node_bounds.size) as u32,
                    "Mipmap sampling out of bounds for z component: ({} + {}) > ({} + {})",
                    sample_start.z,
                    sample_size,
                    node_bounds.min_position.z,
                    node_bounds.size
                );

                let sampled_color =
                    Self::sample_from(&sample_start, sample_size, |pos| -> Option<Albedo> {
                        self.get_internal(node_key, *node_bounds, pos)
                            .albedo()
                            .copied()
                    });

                // Assemble MIP entry
                Some(if let Some(ref color) = sampled_color {
                    self.add_to_palette(&OctreeEntry::Visual(color))
                } else {
                    empty_marker::<PaletteIndexValues>()
                })
            }
            NodeContent::Internal(_occupied_bits) => {
                //TODO: get the largest MIP level below the current one, fall back to LEAF logic if there isn't a MIP level active below
                // determine the sampling range
                let sample_size = 2;
                let sample_start = ((V3c::from(*position) - node_bounds.min_position)
                    * 2.
                    * self.brick_dim as f32)
                    / node_bounds.size; // Transform into 2*DIM space
                let child_octant = hash_region(&((*position).into()), self.brick_dim as f32);
                let sample_start: V3c<u32> = (sample_start
                    - (OCTANT_OFFSET_REGION_LUT[child_octant as usize] * self.brick_dim as f32))
                    .floor()
                    .into();
                let sample_start = sample_start - (sample_start % self.brick_dim); // start from octant start

                debug_assert!(
                    sample_start.x + sample_size < 2 * self.brick_dim,
                    "Mipmap sampling out of bounds for x component: ({} + {}) >= (2 * {})",
                    sample_start.x,
                    sample_size,
                    self.brick_dim
                );
                debug_assert!(
                    sample_start.y + sample_size < 2 * self.brick_dim,
                    "Mipmap sampling out of bounds for y component: ({} + {}) >= (2 * {})",
                    sample_start.y,
                    sample_size,
                    self.brick_dim
                );
                debug_assert!(
                    sample_start.z + sample_size < 2 * self.brick_dim,
                    "Mipmap sampling out of bounds for z component: ({} + {}) >= (2 * {})",
                    sample_start.z,
                    sample_size,
                    self.brick_dim
                );
                let sampled_color = if empty_marker::<u32>() as usize
                    == self.node_children[node_key].child(child_octant)
                {
                    None
                } else {
                    Self::sample_from(&sample_start, sample_size, |pos| -> Option<Albedo> {
                        match &self.node_mips[self.node_children[node_key].child(child_octant)] {
                            BrickData::Empty => None,
                            BrickData::Solid(voxel) => NodeContent::pix_get_ref(
                                voxel,
                                &self.voxel_color_palette,
                                &self.voxel_data_palette,
                            )
                            .albedo()
                            .copied(),
                            BrickData::Parted(brick) => {
                                let mip_index = flat_projection(
                                    pos.x as usize,
                                    pos.y as usize,
                                    pos.z as usize,
                                    self.brick_dim as usize,
                                );
                                NodeContent::pix_get_ref(
                                    &brick[mip_index],
                                    &self.voxel_color_palette,
                                    &self.voxel_data_palette,
                                )
                                .albedo()
                                .copied()
                            }
                        }
                    })
                };

                // Assemble MIP entry
                Some(if let Some(ref color) = sampled_color {
                    self.add_to_palette(&OctreeEntry::Visual(color))
                } else {
                    empty_marker::<PaletteIndexValues>()
                })
            }
        };

        if let Some(mip_entry) = mip_entry {
            // Set MIP entry
            let pos_in_mip = matrix_index_for(node_bounds, position, self.brick_dim);
            let flat_pos_in_mip = flat_projection(
                pos_in_mip.x,
                pos_in_mip.y,
                pos_in_mip.z,
                self.brick_dim as usize,
            );
            match &mut self.node_mips[node_key] {
                BrickData::Empty => {
                    let mut new_brick_data =
                        vec![empty_marker::<PaletteIndexValues>(); self.brick_dim.pow(3) as usize];
                    new_brick_data[flat_pos_in_mip] = mip_entry;
                    self.node_mips[node_key] = BrickData::Parted(new_brick_data);
                }
                BrickData::Solid(voxel) => {
                    let mut new_brick_data = vec![*voxel; self.brick_dim.pow(3) as usize];
                    new_brick_data[flat_pos_in_mip] = mip_entry;
                    self.node_mips[node_key] = BrickData::Parted(new_brick_data);
                }
                BrickData::Parted(brick) => {
                    brick[flat_pos_in_mip] = mip_entry;
                }
            }
        }
    }

    pub(crate) fn recalculate_mip(&mut self, node_key: usize, node_bounds: &Cube) {
        if !self.albedo_mip_maps {
            return;
        }

        for x in 0..self.brick_dim {
            for y in 0..self.brick_dim {
                for z in 0..self.brick_dim {
                    let pos: V3c<f32> = node_bounds.min_position
                        + (V3c::<f32>::new(x as f32, y as f32, z as f32) * node_bounds.size
                            / self.brick_dim as f32)
                            .round();
                    self.update_mip(node_key, node_bounds, &V3c::from(pos));
                }
            }
        }
    }
}
