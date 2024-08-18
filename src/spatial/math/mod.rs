mod tests;
pub mod vector;

use crate::spatial::math::vector::V3c;
use std::ops::Neg;

///####################################################################################
/// Octant
///####################################################################################
pub(crate) fn offset_region(octant: u8) -> V3c<f32> {
    match octant {
        0 => V3c::new(0., 0., 0.),
        1 => V3c::new(1., 0., 0.),
        2 => V3c::new(0., 0., 1.),
        3 => V3c::new(1., 0., 1.),
        4 => V3c::new(0., 1., 0.),
        5 => V3c::new(1., 1., 0.),
        6 => V3c::new(0., 1., 1.),
        7 => V3c::new(1., 1., 1.),
        _ => panic!("Invalid region hash provided for spatial reference!"),
    }
}

/// Each Node is separated to 8 Octants based on their relative position inside the Nodes occupying space.
/// The hash function assigns an index for each octant, so every child Node can be indexed in a well defined manner
/// * `offset` - From range 0..size in each dimensions
/// * `size` - Size of the region to check for child octants
pub fn hash_region(offset: &V3c<f32>, size: f32) -> u8 {
    // The below is rewritten to be branchless
    let half_size = size / 2.0;
    // (if offset.x < half_size { 0 } else { 1 })
    //     + if offset.z < half_size{ 0 } else { 2 }
    //     + if offset.y < half_size { 0 } else { 4 }
    (offset.x >= half_size) as u8
        + (offset.z >= half_size) as u8 * 2
        + (offset.y >= half_size) as u8 * 4
}

/// Maps direction vector to the octant it points to
pub(crate) fn hash_direction(direction: &V3c<f32>) -> u8 {
    debug_assert!((1.0 - direction.length()).abs() < 0.1);
    let offset = V3c::unit(1.) + *direction;
    hash_region(&offset, 2.)
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

/// Returns with a bitmask to select the relevant octant based on the relative position
/// and size of the covered area
pub(crate) fn position_in_bitmap_64bits(x: usize, y: usize, z: usize, size: usize) -> usize {
    const BITMAP_SPACE_DIMENSION: usize = 4;
    debug_assert!((x * BITMAP_SPACE_DIMENSION / size) < BITMAP_SPACE_DIMENSION);
    debug_assert!((y * BITMAP_SPACE_DIMENSION / size) < BITMAP_SPACE_DIMENSION);
    debug_assert!((z * BITMAP_SPACE_DIMENSION / size) < BITMAP_SPACE_DIMENSION);
    let pos_inside_bitmap = flat_projection(
        x * BITMAP_SPACE_DIMENSION / size,
        y * BITMAP_SPACE_DIMENSION / size,
        z * BITMAP_SPACE_DIMENSION / size,
        BITMAP_SPACE_DIMENSION,
    );
    debug_assert!(
        pos_inside_bitmap
            < (BITMAP_SPACE_DIMENSION * BITMAP_SPACE_DIMENSION * BITMAP_SPACE_DIMENSION)
    );
    pos_inside_bitmap
}

/// Updates the given bitmap based on the position and whether or not it's occupied
/// * `x` - x coordinate of position
/// * `y` - y coordinate of position
/// * `z` - z coordinate of position
/// * `size` - range of the given coordinate space
/// * `occupied` - the value to set the bitmask at the given position
/// * `bitmap` - The bitmap to update
pub(crate) fn set_occupancy_in_bitmap_64bits(
    x: usize,
    y: usize,
    z: usize,
    size: usize,
    occupied: bool,
    bitmap: &mut u64,
) {
    let pos_mask = 0x01 << position_in_bitmap_64bits(x, y, z, size);
    if occupied {
        *bitmap |= pos_mask;
    } else {
        *bitmap &= !pos_mask
    }
}

/// Creates a bitmask for a single octant position in an 8bit bitmask
pub(crate) fn octant_bitmask(octant: u8) -> u8 {
    0x01 << octant
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
