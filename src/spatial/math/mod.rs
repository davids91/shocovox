mod tests;
pub mod vector;

use crate::spatial::{math::vector::V3c, Cube};
use std::ops::Neg;

/// Each Node is separated to 8 Octants based on their relative position inside the Nodes occupying space.
/// The hash function assigns an index for each octant, so every child Node can be indexed in a well defined manner
/// * `offset` - From range 0..size in each dimensions
/// * `size` - Size of the region to check for child octants
pub fn hash_region(offset: &V3c<f32>, size_half: f32) -> u8 {
    // The below is rewritten to be branchless
    // (if offset.x < half_size { 0 } else { 1 })
    //     + if offset.z < half_size{ 0 } else { 2 }
    //     + if offset.y < half_size { 0 } else { 4 }
    (offset.x >= size_half) as u8
        + (offset.z >= size_half) as u8 * 2
        + (offset.y >= size_half) as u8 * 4
}

/// Maps direction vector to the octant it points to
pub(crate) fn hash_direction(direction: &V3c<f32>) -> u8 {
    debug_assert!((1.0 - direction.length()).abs() < 0.1);
    let offset = V3c::unit(1.) + *direction;
    hash_region(&offset, 1.)
}

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

pub(crate) const BITMAP_DIMENSION: usize = 4;

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
    let mat_index =
        (V3c::<usize>::from((*position - bounds.min_position.into()) * matrix_dimension))
            / bounds.size as usize;
    // The difference between the actual position and min bounds
    // must not be greater, than self.octree_dim at each dimension
    debug_assert!(mat_index.x < matrix_dimension as usize);
    debug_assert!(mat_index.y < matrix_dimension as usize);
    debug_assert!(mat_index.z < matrix_dimension as usize);

    mat_index
}

/// Returns with a bitmask to select the relevant octant based on the relative position
/// and size of the covered area
pub(crate) fn position_in_bitmap_64bits(index_in_brick: &V3c<usize>, brick_size: usize) -> usize {
    debug_assert!(
        (index_in_brick.x * BITMAP_DIMENSION / brick_size) < BITMAP_DIMENSION,
        "Expected coordinate {:?} == ({:?} * {BITMAP_DIMENSION} / {:?})  to be < bitmap dimension({BITMAP_DIMENSION})",
         (index_in_brick.x * BITMAP_DIMENSION / brick_size), index_in_brick.x, brick_size
    );
    debug_assert!(
        (index_in_brick.y * BITMAP_DIMENSION / brick_size) < BITMAP_DIMENSION,
        "Expected coordinate {:?} == ({:?} * {BITMAP_DIMENSION} / {:?})  to be < bitmap dimension({BITMAP_DIMENSION})",
         (index_in_brick.y * BITMAP_DIMENSION / brick_size), index_in_brick.y, brick_size
    );
    debug_assert!(
        (index_in_brick.z * BITMAP_DIMENSION / brick_size) < BITMAP_DIMENSION,
        "Expected coordinate {:?} == ({:?} * {BITMAP_DIMENSION} / {:?})  to be < bitmap dimension({BITMAP_DIMENSION})",
         (index_in_brick.z * BITMAP_DIMENSION / brick_size), index_in_brick.z, brick_size
    );
    let pos_inside_bitmap = flat_projection(
        index_in_brick.x * BITMAP_DIMENSION / brick_size,
        index_in_brick.y * BITMAP_DIMENSION / brick_size,
        index_in_brick.z * BITMAP_DIMENSION / brick_size,
        BITMAP_DIMENSION,
    );
    debug_assert!(pos_inside_bitmap < (BITMAP_DIMENSION * BITMAP_DIMENSION * BITMAP_DIMENSION));
    pos_inside_bitmap
}

/// Updates occupancy data in parts of the given bitmap defined by the given position and size range
/// * `position` - start coordinate of position to update
/// * `size` - size to set inside the bitmap
/// * `brick_size` - range of the given coordinate space
/// * `occupied` - the value to set the bitmask at the given position
/// * `bitmap` - The bitmap to update
pub(crate) fn set_occupancy_in_bitmap_64bits(
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

    let update_count = (size as f32 * BITMAP_DIMENSION as f32 / brick_dim as f32).ceil() as usize;
    let update_start = (*position * BITMAP_DIMENSION) / brick_dim as usize;
    for x in update_start.x..(update_start.x + update_count).min(BITMAP_DIMENSION) {
        for y in update_start.y..(update_start.y + update_count).min(BITMAP_DIMENSION) {
            for z in update_start.z..(update_start.z + update_count).min(BITMAP_DIMENSION) {
                let pos_mask =
                    0x01 << position_in_bitmap_64bits(&V3c::new(x, y, z), BITMAP_DIMENSION);
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
pub(crate) enum CoordinateSystemType {
    LZUP, // Left handed Z Up
    LYUP, // Left handed Y Up
    RZUP, // Right handed Z Up
    RYUP, // Right handed Y Up
}

#[cfg(feature = "dot_vox_support")]
pub(crate) fn convert_coordinate<T: Copy + Neg<Output = T>>(
    c: V3c<T>,
    src_type: CoordinateSystemType,
    dst_type: CoordinateSystemType,
) -> V3c<T> {
    match (src_type, dst_type) {
        (CoordinateSystemType::LZUP, CoordinateSystemType::LZUP) => c,
        (CoordinateSystemType::LYUP, CoordinateSystemType::LYUP) => c,
        (CoordinateSystemType::RZUP, CoordinateSystemType::RZUP) => c,
        (CoordinateSystemType::RYUP, CoordinateSystemType::RYUP) => c,

        (CoordinateSystemType::LYUP, CoordinateSystemType::RYUP)
        | (CoordinateSystemType::RYUP, CoordinateSystemType::LYUP) => V3c::new(c.x, c.y, -c.z),

        (CoordinateSystemType::LZUP, CoordinateSystemType::RZUP)
        | (CoordinateSystemType::RZUP, CoordinateSystemType::LZUP) => V3c::new(c.x, -c.y, c.z),

        (CoordinateSystemType::LYUP, CoordinateSystemType::LZUP)
        | (CoordinateSystemType::RYUP, CoordinateSystemType::RZUP) => V3c::new(c.x, -c.z, c.y),
        (CoordinateSystemType::LZUP, CoordinateSystemType::LYUP)
        | (CoordinateSystemType::RZUP, CoordinateSystemType::RYUP) => V3c::new(c.x, c.z, -c.y),

        (CoordinateSystemType::LYUP, CoordinateSystemType::RZUP)
        | (CoordinateSystemType::RZUP, CoordinateSystemType::LYUP)
        | (CoordinateSystemType::RYUP, CoordinateSystemType::LZUP)
        | (CoordinateSystemType::LZUP, CoordinateSystemType::RYUP) => V3c::new(c.x, c.z, c.y),
    }
}
