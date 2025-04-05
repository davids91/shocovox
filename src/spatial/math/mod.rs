mod tests;
pub mod vector;

use crate::{
    octree::BOX_NODE_DIMENSION,
    spatial::{lut::SECTANT_OFFSET_LUT, math::vector::V3c, Cube},
};
use std::ops::Neg;

pub(crate) const FLOAT_ERROR_TOLERANCE: f32 = 0.00001;

/// Maps 3 dimensional space limited by `size` to 1 dimension
/// This mapping function supposes that the coordinates are bound inside
/// a cube, each dimension `size` long.
/// * `x` - x coordinate of position
/// * `y` - y coordinate of position
/// * `z` - z coordinate of position
/// * `size` - Range of the given coordinate space
pub(crate) fn flat_projection(x: usize, y: usize, z: usize, size: usize) -> usize {
    x + (y * size) + (z * size * size)
}

/// Each Node is separated to 64 sectants based on their relative position inside the Nodes occupying space.
/// The hash function assigns an index for each sectant, so every child cell can be indexed in a well defined manner
/// * `offset` - From range 0..size in each dimensions
/// * `size` - Size of the region to check for child sectants
pub(crate) fn hash_region(offset: &V3c<f32>, size: f32) -> u8 {
    // Scale to 0..BOX_NODE_CHILDREN_COUNT, then project to an unique index
    debug_assert!(
        offset.x <= (size + FLOAT_ERROR_TOLERANCE)
            && offset.y <= (size + FLOAT_ERROR_TOLERANCE)
            && offset.z <= (size + FLOAT_ERROR_TOLERANCE)
            && offset.x >= (-FLOAT_ERROR_TOLERANCE)
            && offset.y >= (-FLOAT_ERROR_TOLERANCE)
            && offset.z >= (-FLOAT_ERROR_TOLERANCE),
        "Expected relative offset {:?} to be inside {size}^3",
        offset
    );
    // let index: V3c<usize> = (*offset * BOX_NODE_DIMENSION as f32 / size).floor().into();
    // // During raytracing, positions on cube boundaries need to be mapped to an index inside @BOX_NODE_DIMENSION
    // let index = index.cut_each_component(BOX_NODE_DIMENSION - 1);
    // BOX_NODE_INDEX_TO_SECTANT_LUT[index.x][index.y][index.z]
    let index = (*offset * BOX_NODE_DIMENSION as f32 / size).floor();
    // During raytracing, positions on cube boundaries need to be mapped to an index inside @BOX_NODE_DIMENSION
    let index = index.cut_each_component((BOX_NODE_DIMENSION - 1) as f32);
    (index.x + (index.y * BOX_NODE_DIMENSION as f32) + (index.z * BOX_NODE_DIMENSION.pow(2) as f32))
        as u8 //flat_projection_f32
}

#[cfg(feature = "raytracing")]
/// Maps direction vector to the octant it points to for indexing within RAY_TO_NODE_OCCUPANCY_BITMASK_LUT
pub(crate) fn hash_direction(direction: &V3c<f32>) -> u8 {
    debug_assert!((1. - direction.length()).abs() < 0.1);
    let offset = V3c::unit(1.) + *direction;
    (offset.x >= 1.) as u8 + (offset.z >= 1.) as u8 * 2 + (offset.y >= 1.) as u8 * 4
}

/// Provides the index value of a given sectant inside a 2x2x2 matrix ( which has octants )
/// Types are not u8 only because this utility is mainly used to index inside bricks
pub(crate) fn octant_in_sectants(sectant: usize) -> usize {
    let offset = SECTANT_OFFSET_LUT[sectant] * 2.;
    (offset.x >= 1.) as usize + (offset.z >= 1.) as usize * 2 + (offset.y >= 1.) as usize * 4
}

/// Provides an index value inside the brick contained in the given bounds
/// Requires that position is larger, than the min_position of the bounds
/// It takes into consideration the size of the bounds as well
pub(crate) fn matrix_index_for(
    bounds: &Cube,
    position: &V3c<u32>,
    matrix_dimension: u32,
) -> V3c<usize> {
    // The position should be inside the bounds
    debug_assert!(
        bounds.min_position.x <= position.x as f32
            && bounds.min_position.y <= position.y as f32
            && bounds.min_position.z <= position.z as f32
            && bounds.min_position.x + bounds.size > position.x as f32
            && bounds.min_position.y + bounds.size > position.y as f32
            && bounds.min_position.z + bounds.size > position.z as f32,
        "Position {:?} not inside bounds {:?}",
        position,
        bounds
    );

    // --> In case the smallest possible node the contained matrix of voxels
    // starts at bounds min_position and ends in min_position + (DIM,DIM,DIM)
    // --> In case of bigger Nodes the below ratio equation is relevant
    // mat[xyz]/DIM = (position - min_position) / bounds.size
    let mat_index = V3c::<usize>::from(
        ((V3c::<f32>::from(*position) - bounds.min_position) * matrix_dimension as f32
            / bounds.size)
            .floor(),
    );
    // The difference between the actual position and min bounds
    // must not be greater, than brick_dimension at each dimension
    debug_assert!(mat_index.x < matrix_dimension as usize);
    debug_assert!(mat_index.y < matrix_dimension as usize);
    debug_assert!(mat_index.z < matrix_dimension as usize);

    mat_index
}

/// Updates occupancy data in parts of the given bitmap defined by the given position and size range
/// * `position` - start coordinate of position to update
/// * `size` - size to set inside the bitmap
/// * `brick_size` - range of the given coordinate space
/// * `occupied` - the value to set the bitmask at the given position
/// * `bitmap` - The bitmap to update
pub(crate) fn set_occupied_bitmap_value(
    position: &V3c<usize>,
    size: usize,
    brick_dim: usize,
    occupied: bool,
    bitmap: &mut u64,
) {
    // In case the brick size is smaller than 4, one position sets multiple bits
    debug_assert!(brick_dim >= 4 || (brick_dim == 2 || brick_dim == 1));
    debug_assert!(
        position.x < brick_dim,
        "Expected coordinate {:?} < brick size({brick_dim})",
        position.x
    );
    debug_assert!(
        position.y < brick_dim,
        "Expected coordinate {:?} < brick size({brick_dim})",
        position.y
    );
    debug_assert!(
        position.z < brick_dim,
        "Expected coordinate {:?} < brick size({brick_dim})",
        position.z
    );

    if brick_dim == 1 {
        *bitmap = if occupied { u64::MAX } else { 0 };
        return;
    }

    let update_count = (size as f32 * BOX_NODE_DIMENSION as f32 / brick_dim as f32).ceil() as usize;
    let update_start: V3c<usize> = (V3c::<f32>::from(*position * BOX_NODE_DIMENSION)
        / brick_dim as f32)
        .floor()
        .into();
    for x in update_start.x..(update_start.x + update_count).min(BOX_NODE_DIMENSION) {
        for y in update_start.y..(update_start.y + update_count).min(BOX_NODE_DIMENSION) {
            for z in update_start.z..(update_start.z + update_count).min(BOX_NODE_DIMENSION) {
                let pos_mask = 0x01
                    << hash_region(
                        &V3c::new(x as f32, y as f32, z as f32),
                        BOX_NODE_DIMENSION as f32,
                    );
                if occupied {
                    *bitmap |= pos_mask;
                } else {
                    *bitmap &= !pos_mask
                }
            }
        }
    }
}

#[cfg(feature = "dot_vox_support")]
#[allow(dead_code)]
pub(crate) enum CoordinateSystemType {
    Lzup, // Left handed Z Up
    Lyup, // Left handed Y Up
    Rzup, // Right handed Z Up
    Ryup, // Right handed Y Up
}

#[cfg(feature = "dot_vox_support")]
pub(crate) fn convert_coordinate<T: Copy + Neg<Output = T>>(
    c: V3c<T>,
    src_type: CoordinateSystemType,
    dst_type: CoordinateSystemType,
) -> V3c<T> {
    match (src_type, dst_type) {
        (CoordinateSystemType::Lzup, CoordinateSystemType::Lzup) => c,
        (CoordinateSystemType::Lyup, CoordinateSystemType::Lyup) => c,
        (CoordinateSystemType::Rzup, CoordinateSystemType::Rzup) => c,
        (CoordinateSystemType::Ryup, CoordinateSystemType::Ryup) => c,

        (CoordinateSystemType::Lyup, CoordinateSystemType::Ryup)
        | (CoordinateSystemType::Ryup, CoordinateSystemType::Lyup) => V3c::new(c.x, c.y, -c.z),

        (CoordinateSystemType::Lzup, CoordinateSystemType::Rzup)
        | (CoordinateSystemType::Rzup, CoordinateSystemType::Lzup) => V3c::new(c.x, -c.y, c.z),

        (CoordinateSystemType::Lyup, CoordinateSystemType::Lzup)
        | (CoordinateSystemType::Ryup, CoordinateSystemType::Rzup) => V3c::new(c.x, -c.z, c.y),
        (CoordinateSystemType::Lzup, CoordinateSystemType::Lyup)
        | (CoordinateSystemType::Rzup, CoordinateSystemType::Ryup) => V3c::new(c.x, c.z, -c.y),

        (CoordinateSystemType::Lyup, CoordinateSystemType::Rzup)
        | (CoordinateSystemType::Rzup, CoordinateSystemType::Lyup)
        | (CoordinateSystemType::Ryup, CoordinateSystemType::Lzup)
        | (CoordinateSystemType::Lzup, CoordinateSystemType::Ryup) => V3c::new(c.x, c.z, c.y),
    }
}
