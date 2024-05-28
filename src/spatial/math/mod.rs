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
