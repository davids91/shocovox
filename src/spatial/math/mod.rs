pub mod vector;

use crate::spatial::math::vector::V3c;

///####################################################################################
/// Octant
///####################################################################################
pub(crate) fn offset_region(octant: u8) -> V3c<u32> {
    match octant {
        0 => V3c::new(0, 0, 0),
        1 => V3c::new(1, 0, 0),
        2 => V3c::new(0, 0, 1),
        3 => V3c::new(1, 0, 1),
        4 => V3c::new(0, 1, 0),
        5 => V3c::new(1, 1, 0),
        6 => V3c::new(0, 1, 1),
        7 => V3c::new(1, 1, 1),
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

fn bitmask_mapping_64bits(x: usize, y: usize, z: usize, size: usize) -> usize {
    x + (y * size) + (z * size * size)
}

/// Returns with a bitmask to select the relevant octant based on the relative position
/// and size of the covered area
fn position_in_bitmask_64bits(x: usize, y: usize, z: usize, size: usize) -> u8 {
    let pos_inside_bitmask = (V3c::new(x, y, z) * 4) / size;
    debug_assert!(pos_inside_bitmask.x < 4);
    debug_assert!(pos_inside_bitmask.y < 4);
    debug_assert!(pos_inside_bitmask.z < 4);
    let pos_inside_bitmask = bitmask_mapping_64bits(
        pos_inside_bitmask.x,
        pos_inside_bitmask.y,
        pos_inside_bitmask.z,
        size,
    );
    debug_assert!(pos_inside_bitmask < 64);
    pos_inside_bitmask as u8
}

/// Updates the given bitmask based on the position and whether or not it's occupied
/// * `x` - x coordinate of position
/// * `y` - y coordinate of position
/// * `z` - z coordinate of position
/// * `size` - x coordinate of position
/// * `occupied` - the value to set the bitmask at the given position
/// * `mask` - The bitmask to query
pub(crate) fn set_occupancy_in_bitmap_64bits(
    x: usize,
    y: usize,
    z: usize,
    size: usize,
    occupied: bool,
    bitmask: &mut u64,
) {
    let pos_mask = 0x01 << position_in_bitmask_64bits(x, y, z, size);
    if occupied {
        *bitmask |= pos_mask;
    } else {
        *bitmask &= !pos_mask
    }
}

/// Queries the given bitmask based on the position for whether or not it's occupied
/// * `x` - x coordinate of position
/// * `y` - y coordinate of position
/// * `z` - z coordinate of position
/// * `size` - x coordinate of position
/// * `mask` - The bitmask to query
/// returns with true if the bitmask stores the position to be occupied
pub(crate) fn get_occupancy_in_bitmap_64bits(
    x: usize,
    y: usize,
    z: usize,
    size: usize,
    bitmask: u64,
) -> bool {
    let pos_mask = 0x01 << position_in_bitmask_64bits(x, y, z, size);
    0 < (bitmask & pos_mask)
}

/// Creates a bitmask for a single octant position in an 8bit bitmask
pub(crate) fn octant_bitmask(octant: u8) -> u8 {
    0x01 << octant
}

/// Queries a bitmask for a single octant position in an 8bit bitmask
pub(crate) fn is_bitmask_occupied_at_octant(bitmask: u8, octant: u8) -> bool {
    0 < bitmask & octant_bitmask(octant)
}

#[allow(dead_code)] // Could be useful either for debugging or new implementations
#[cfg(feature = "raytracing")]
/// calculates the distance between the line, and the plane both described by a ray
/// plane: normal, and a point on plane, line: origin and direction
/// return the distance from the line origin to the direction of it, if they have an intersection
pub fn plane_line_intersection(
    plane_point: &V3c<f32>,
    plane_normal: &V3c<f32>,
    line_origin: &V3c<f32>,
    line_direction: &V3c<f32>,
) -> Option<f32> {
    let origins_diff = *plane_point - *line_origin;
    let plane_line_dot_to_plane = origins_diff.dot(plane_normal);
    let directions_dot = line_direction.dot(plane_normal);
    if 0. == directions_dot {
        // line and plane is paralell
        if 0. == origins_diff.dot(plane_normal) {
            // The distance is zero because the origin is already on the plane
            return Some(0.);
        }
        return None;
    }
    Some(plane_line_dot_to_plane / directions_dot)
}
