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
    use crate::spatial::math::offset_region;
    use crate::spatial::V3c;

    #[test]
    fn test_hash_region() {
        assert!(hash_region(&V3c::new(0.0, 0.0, 0.0), 10.0) == 0);
        assert!(hash_region(&V3c::new(6.0, 0.0, 0.0), 10.0) == 1);
        assert!(hash_region(&V3c::new(0.0, 0.0, 6.0), 10.0) == 2);
        assert!(hash_region(&V3c::new(6.0, 0.0, 6.0), 10.0) == 3);
        assert!(hash_region(&V3c::new(0.0, 6.0, 0.0), 10.0) == 4);
        assert!(hash_region(&V3c::new(6.0, 6.0, 0.0), 10.0) == 5);
        assert!(hash_region(&V3c::new(0.0, 6.0, 6.0), 10.0) == 6);
        assert!(hash_region(&V3c::new(6.0, 6.0, 6.0), 10.0) == 7);
    }

    #[test]
    fn test_offset_region() {
        assert!(V3c::new(0, 0, 0) == offset_region(0));
        assert!(V3c::new(1, 0, 0) == offset_region(1));
        assert!(V3c::new(0, 0, 1) == offset_region(2));
        assert!(V3c::new(1, 0, 1) == offset_region(3));
        assert!(V3c::new(0, 1, 0) == offset_region(4));
        assert!(V3c::new(1, 1, 0) == offset_region(5));
        assert!(V3c::new(0, 1, 1) == offset_region(6));
        assert!(V3c::new(1, 1, 1) == offset_region(7));
    }
}

#[cfg(test)]
mod intersection_tests {

    use crate::spatial::{math::plane_line_intersection_distance, V3c};

    #[test]
    fn test_negative_intersection() {
        let plane_point = V3c::new(0., 0., 0.);
        let plane_normal = V3c::new(0., 1., 0.);
        let line_origin = V3c::new(0., 1., 0.);
        let line_direction = V3c::new(0., 1., 0.);
        assert!(plane_line_intersection_distance(
            &plane_point,
            &plane_normal,
            &line_origin,
            &line_direction
        )
        .is_some_and(|v| v == -1.));
    }
}