#[cfg(test)]
mod vector_tests {

    use crate::spatial::V3c;

    #[test]
    fn test_cross_product() {
        let a = V3c::new(3., 0., 2.);
        let b = V3c::new(-1., 4., 2.);
        let cross = a.cross(b);
        assert!(cross.x == -8.);
        assert!(cross.y == -8.);
        assert!(cross.z == 12.);
    }
}

#[cfg(test)]
mod octant_tests {
    use crate::spatial::math::hash_region;
    use crate::spatial::V3c;

    #[test]
    fn test_hash_region() {
        assert!(hash_region(&V3c::new(0.0, 0.0, 0.0), 5.0) == 0);
        assert!(hash_region(&V3c::new(6.0, 0.0, 0.0), 5.0) == 1);
        assert!(hash_region(&V3c::new(0.0, 0.0, 6.0), 5.0) == 2);
        assert!(hash_region(&V3c::new(6.0, 0.0, 6.0), 5.0) == 3);
        assert!(hash_region(&V3c::new(0.0, 6.0, 0.0), 5.0) == 4);
        assert!(hash_region(&V3c::new(6.0, 6.0, 0.0), 5.0) == 5);
        assert!(hash_region(&V3c::new(0.0, 6.0, 6.0), 5.0) == 6);
        assert!(hash_region(&V3c::new(6.0, 6.0, 6.0), 5.0) == 7);
    }
}

#[cfg(test)]
mod bitmask_tests {

    use crate::octree::V3c;
    use crate::spatial::math::{flat_projection, octant_bitmask, position_in_bitmap_64bits};
    use std::collections::HashSet;

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
        assert!(0 == position_in_bitmap_64bits(&V3c::new(0, 0, 0), 4));
        assert!(32 == position_in_bitmap_64bits(&V3c::new(0, 0, 2), 4));
        assert!(63 == position_in_bitmap_64bits(&V3c::new(3, 3, 3), 4));
    }

    #[test]
    fn test_lvl1_flat_projection_greater_dimension() {
        assert!(0 == position_in_bitmap_64bits(&V3c::new(0, 0, 0), 10));
        assert!(32 == position_in_bitmap_64bits(&V3c::new(0, 0, 5), 10));
        assert!(42 == position_in_bitmap_64bits(&V3c::new(5, 5, 5), 10));
        assert!(63 == position_in_bitmap_64bits(&V3c::new(9, 9, 9), 10));
    }
    #[test]
    fn test_lvl1_flat_projection_smaller_dimension() {
        assert!(0 == position_in_bitmap_64bits(&V3c::new(0, 0, 0), 2));
        assert!(2 == position_in_bitmap_64bits(&V3c::new(1, 0, 0), 2));
        assert!(8 == position_in_bitmap_64bits(&V3c::new(0, 1, 0), 2));
        assert!(10 == position_in_bitmap_64bits(&V3c::new(1, 1, 0), 2));
        assert!(32 == position_in_bitmap_64bits(&V3c::new(0, 0, 1), 2));
        assert!(34 == position_in_bitmap_64bits(&V3c::new(1, 0, 1), 2));
        assert!(40 == position_in_bitmap_64bits(&V3c::new(0, 1, 1), 2));
        assert!(42 == position_in_bitmap_64bits(&V3c::new(1, 1, 1), 2));
    }
}
