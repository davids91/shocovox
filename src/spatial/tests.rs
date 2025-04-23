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
mod detail_tests {
    use crate::octree::V3c;
    use crate::spatial::{update_size_within, Cube};

    #[test]
    fn test_update_size() {
        let bounds = Cube {
            min_position: V3c::unit(5.),
            size: 5.,
        };
        assert_eq!(update_size_within(&bounds, &V3c::new(5, 5, 5), 5), 5);
        assert_eq!(update_size_within(&bounds, &V3c::new(5, 5, 6), 5), 4);
        assert_eq!(update_size_within(&bounds, &V3c::new(8, 8, 8), 5), 2);
        assert_eq!(update_size_within(&bounds, &V3c::new(8, 8, 8), 2), 2);
        assert_eq!(update_size_within(&bounds, &V3c::new(6, 5, 6), 3), 3);
        assert_eq!(update_size_within(&bounds, &V3c::new(5, 5, 5), 2), 2);
    }
}

#[cfg(test)]
mod sectant_tests {
    use crate::spatial::math::hash_region;
    use crate::spatial::V3c;

    #[test]
    fn test_hash_region() {
        assert_eq!(hash_region(&V3c::new(0.0, 0.0, 0.0), 12.0), 0);
        assert_eq!(hash_region(&V3c::new(3.0, 0.0, 0.0), 12.0), 1);
        assert_eq!(hash_region(&V3c::new(0.0, 3.0, 0.0), 12.0), 4);
        assert_eq!(hash_region(&V3c::new(0.0, 0.0, 3.0), 12.0), 16);
        assert_eq!(hash_region(&V3c::new(10.0, 10.0, 10.0), 12.0), 63);
    }
}

#[cfg(test)]
mod bitmask_tests {

    use crate::octree::V3c;
    use crate::spatial::math::{flat_projection, hash_region};
    use std::collections::HashSet;

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
    fn test_bitmap_flat_projection_exact_size_match() {
        assert_eq!(0, hash_region(&V3c::new(0., 0., 0.), 4.));
        assert_eq!(32, hash_region(&V3c::new(0., 0., 2.), 4.));
        assert_eq!(63, hash_region(&V3c::new(3., 3., 3.), 4.));
    }

    #[test]
    fn test_bitmap_flat_projection_greater_dimension() {
        assert_eq!(0, hash_region(&V3c::new(0., 0., 0.), 10.));
        assert_eq!(32, hash_region(&V3c::new(0., 0., 5.), 10.));
        assert_eq!(42, hash_region(&V3c::new(5., 5., 5.), 10.));
        assert_eq!(63, hash_region(&V3c::new(9., 9., 9.), 10.));
    }

    #[test]
    fn test_bitmap_flat_projection_smaller_dimension() {
        assert_eq!(0, hash_region(&V3c::new(0., 0., 0.), 2.));
        assert_eq!(2, hash_region(&V3c::new(1., 0., 0.), 2.));
        assert_eq!(8, hash_region(&V3c::new(0., 1., 0.), 2.));
        assert_eq!(10, hash_region(&V3c::new(1., 1., 0.), 2.));
        assert_eq!(32, hash_region(&V3c::new(0., 0., 1.), 2.));
        assert_eq!(34, hash_region(&V3c::new(1., 0., 1.), 2.));
        assert_eq!(40, hash_region(&V3c::new(0., 1., 1.), 2.));
        assert_eq!(42, hash_region(&V3c::new(1., 1., 1.), 2.));
    }
}
