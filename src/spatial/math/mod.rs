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
pub fn hash_region(offset: &V3c<f32>, size_half: f32) -> u8 {
    // The below is rewritten to be branchless
    // (if offset.x < half_size { 0 } else { 1 })
    //     + if offset.z < half_size{ 0 } else { 2 }
    //     + if offset.y < half_size { 0 } else { 4 }
    (offset.x >= size_half) as u8
        + (offset.z >= size_half) as u8 * 2
        + (offset.y >= size_half) as u8 * 4
}

/// Generates a bitmask based on teh given octant corresponding to the coverage of the octant in an occupancy bitmap
pub(crate) fn mask_for_octant_64_bits(octant: u8) -> u64 {
    match octant {
        0 => 0x0000000000330033,
        1 => 0x0000000000cc00cc,
        2 => 0x0033003300000000,
        3 => 0x00cc00cc00000000,
        4 => 0x0000000033003300,
        5 => 0x00000000cc00cc00,
        6 => 0x3300330000000000,
        7 => 0xcc00cc0000000000,
        _ => panic!("Invalid region hash provided for spatial reference!"),
    }
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

/// Returns with a bitmask to select the relevant octant based on the relative position
/// and size of the covered area
pub(crate) fn position_in_bitmap_64bits(x: usize, y: usize, z: usize, brick_size: usize) -> usize {
    const BITMAP_SPACE_DIMENSION: usize = 4;
    debug_assert!((x * BITMAP_SPACE_DIMENSION / brick_size) < BITMAP_SPACE_DIMENSION);
    debug_assert!((y * BITMAP_SPACE_DIMENSION / brick_size) < BITMAP_SPACE_DIMENSION);
    debug_assert!((z * BITMAP_SPACE_DIMENSION / brick_size) < BITMAP_SPACE_DIMENSION);
    let pos_inside_bitmap = flat_projection(
        x * BITMAP_SPACE_DIMENSION / brick_size,
        y * BITMAP_SPACE_DIMENSION / brick_size,
        z * BITMAP_SPACE_DIMENSION / brick_size,
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
    brick_size: usize,
    occupied: bool,
    bitmap: &mut u64,
) {
    // In case the brick size is smaller than 4, one position sets multiple bits
    debug_assert!(brick_size >= 4 || (brick_size == 2 || brick_size == 1));
    debug_assert!(
        x < brick_size,
        "Expected coordinate {x} < brick size({brick_size})"
    );
    debug_assert!(
        y < brick_size,
        "Expected coordinate {x} < brick size({brick_size})"
    );
    debug_assert!(
        z < brick_size,
        "Expected coordinate {x} < brick size({brick_size})"
    );

    if brick_size == 1 {
        *bitmap = if occupied { u64::MAX } else { 0 };
        return;
    }

    if brick_size == 2 {
        // One position will set 4 bits
        for x_ in (x * 2)..(((x * 2) + 2).min(4)) {
            for y_ in (y * 2)..(((y * 2) + 2).min(4)) {
                for z_ in (z * 2)..(((z * 2) + 2).min(4)) {
                    let pos_mask = 0x01 << position_in_bitmap_64bits(x_, y_, z_, 4);
                    if occupied {
                        *bitmap |= pos_mask;
                    } else {
                        *bitmap &= !pos_mask
                    }
                }
            }
        }
    }

    let pos_mask = 0x01 << position_in_bitmap_64bits(x, y, z, brick_size);
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
