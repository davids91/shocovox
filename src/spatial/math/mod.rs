pub mod vector;

use crate::spatial::{math::vector::V3c, Cube, FLOAT_ERROR_TOLERANCE};

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

/// Queries a bitmask for a single octant position in an 8bit bitmask
pub(crate) fn is_bitmap_occupied_at_octant(bitmap: u8, octant: u8) -> bool {
    0 < bitmap & octant_bitmask(octant)
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

pub fn cube_impact_normal(cube: &Cube, impact_point: &V3c<f32>) -> V3c<f32> {
    let mid_to_impact =
        V3c::from(cube.min_position) + V3c::unit(cube.size as f32 / 2.) - *impact_point;
    let max_component = mid_to_impact
        .x
        .abs()
        .max(mid_to_impact.y.abs())
        .max(mid_to_impact.z.abs());

    let impact_normal = V3c::new(
        if mid_to_impact.x.abs() == max_component {
            mid_to_impact.x
        } else {
            0.
        },
        if mid_to_impact.y.abs() == max_component {
            mid_to_impact.y
        } else {
            0.
        },
        if mid_to_impact.z.abs() == max_component {
            mid_to_impact.z
        } else {
            0.
        },
    );

    debug_assert!(0. < impact_normal.length());
    impact_normal.normalized()
}

#[cfg(test)]
mod bitmask_tests {

    use std::collections::HashSet;

    use super::flat_projection;
    use super::octant_bitmask;
    use super::position_in_bitmap_64bits;

    #[test]
    fn test_lvl2_flat_projection() {
        for octant in 0..8 {
            let bitmask = octant_bitmask(octant);
            for compare_octant in 0..8 {
                assert!(compare_octant == octant || 0 == bitmask & octant_bitmask(compare_octant));
            }
        }
    }

    #[test]
    fn test_flat_projection() {
        const DIMENSION: usize = 10;
        assert!(0 == flat_projection(0, 0, 0, DIMENSION));
        assert!(DIMENSION == flat_projection(10, 0, 0, DIMENSION));
        assert!(DIMENSION == flat_projection(0, 1, 0, DIMENSION));
        assert!(DIMENSION * DIMENSION == flat_projection(0, 0, 1, DIMENSION));
        assert!(DIMENSION * DIMENSION * 4 == flat_projection(0, 0, 4, DIMENSION));
        assert!((DIMENSION * DIMENSION * 4) + 3 == flat_projection(3, 0, 4, DIMENSION));
        assert!(
            (DIMENSION * DIMENSION * 4) + (DIMENSION * 2) + 3
                == flat_projection(3, 2, 4, DIMENSION)
        );

        let mut number_coverage = HashSet::new();
        for x in 0..DIMENSION {
            for y in 0..DIMENSION {
                for z in 0..DIMENSION {
                    let address = flat_projection(x, y, z, DIMENSION);
                    assert!(!number_coverage.contains(&address));
                    number_coverage.insert(address);
                }
            }
        }
    }

    #[test]
    fn test_lvl1_flat_projection_exact_size_match() {
        assert!(0 == position_in_bitmap_64bits(0, 0, 0, 4));
        assert!(32 == position_in_bitmap_64bits(0, 0, 2, 4));
        assert!(63 == position_in_bitmap_64bits(3, 3, 3, 4));
    }

    #[test]
    fn test_lvl1_flat_projection_greater_dimension() {
        assert!(0 == position_in_bitmap_64bits(0, 0, 0, 10));
        assert!(32 == position_in_bitmap_64bits(0, 0, 5, 10));
        assert!(42 == position_in_bitmap_64bits(5, 5, 5, 10));
        assert!(63 == position_in_bitmap_64bits(9, 9, 9, 10));
    }
    #[test]
    fn test_lvl1_flat_projection_smaller_dimension() {
        assert!(0 == position_in_bitmap_64bits(0, 0, 0, 2));
        assert!(2 == position_in_bitmap_64bits(1, 0, 0, 2));
        assert!(8 == position_in_bitmap_64bits(0, 1, 0, 2));
        assert!(10 == position_in_bitmap_64bits(1, 1, 0, 2));
        assert!(32 == position_in_bitmap_64bits(0, 0, 1, 2));
        assert!(34 == position_in_bitmap_64bits(1, 0, 1, 2));
        assert!(40 == position_in_bitmap_64bits(0, 1, 1, 2));
        assert!(42 == position_in_bitmap_64bits(1, 1, 1, 2));
    }
}
