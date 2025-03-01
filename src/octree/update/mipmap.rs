use crate::{
    object_pool::empty_marker,
    octree::{
        types::{BrickData, NodeChildren, NodeContent, OctreeEntry, PaletteIndexValues},
        Albedo, Octree, VoxelData,
    },
    spatial::{
        lut::OCTANT_OFFSET_REGION_LUT,
        math::{flat_projection, hash_region, matrix_index_for, vector::V3c},
        Cube,
    },
};
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
                // determine the sampling range
                let pos_in_bounds = V3c::from(*position) - node_bounds.min_position;
                let child_octant = hash_region(&pos_in_bounds, node_bounds.size / 2.);
                let sample_size = 2;
                let sample_start = pos_in_bounds * 2. * self.brick_dim as f32 / node_bounds.size; // Transform into 2*DIM space
                let sample_start: V3c<u32> = (sample_start
                    - (OCTANT_OFFSET_REGION_LUT[child_octant as usize] * self.brick_dim as f32))
                    .floor()
                    .into();
                let sample_start = sample_start - (sample_start % 2); // sample from grid of 2

                debug_assert!(
                    sample_start.x + sample_size <= 2 * self.brick_dim,
                    "Mipmap sampling out of bounds for x component: ({} + {}) > (2 * {})",
                    sample_start.x,
                    sample_size,
                    self.brick_dim
                );
                debug_assert!(
                    sample_start.y + sample_size <= 2 * self.brick_dim,
                    "Mipmap sampling out of bounds for y component: ({} + {}) > (2 * {})",
                    sample_start.y,
                    sample_size,
                    self.brick_dim
                );
                debug_assert!(
                    sample_start.z + sample_size <= 2 * self.brick_dim,
                    "Mipmap sampling out of bounds for z component: ({} + {}) > (2 * {})",
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
