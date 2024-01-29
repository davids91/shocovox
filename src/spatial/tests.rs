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

#[cfg(test)]
mod octree_tests {
    use crate::octree::Octree;
    use crate::spatial::math::V3c;

    #[test]
    fn test_simple_insert_and_get() {
        let mut tree = Octree::<u32>::new(2).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), 5).ok();
        tree.insert(&V3c::new(0, 1, 0), 6).ok();
        tree.insert(&V3c::new(0, 0, 1), 7).ok();

        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == 6);
        assert!(*tree.get(&V3c::new(0, 0, 1)).unwrap() == 7);
        assert!(tree.get(&V3c::new(1, 1, 1)).is_none());
    }

    #[test]
    fn test_get_mut() {
        let mut tree = Octree::<u32>::new(2).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), 5).ok();
        tree.insert(&V3c::new(0, 1, 0), 6).ok();
        tree.insert(&V3c::new(0, 0, 1), 7).ok();

        assert!(*tree.get_mut(&V3c::new(1, 0, 0)).unwrap() == 5);
        assert!(*tree.get_mut(&V3c::new(0, 1, 0)).unwrap() == 6);
        assert!(*tree.get_mut(&V3c::new(0, 0, 1)).unwrap() == 7);
        assert!(tree.get_mut(&V3c::new(1, 1, 1)).is_none());
    }

    #[test]
    fn test_insert_at_lod() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();
        tree.auto_simplify = false;

        // This will set the area equal to 8 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 2, 5).ok();

        assert!(*tree.get(&V3c::new(0, 0, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 0, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 0, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 1, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 1, 1)).unwrap() == 5);

        // This will set the area equal to 64 1-sized nodes:
        // a size-4 node includes 2 levels,
        // 1-sized nodes at the bottom level doesn't have children,
        // 2-sized nodes above have 8 children each
        // so one 4-sized node has 8*8 = 64 children
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, 1).ok();
        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    if tree.get(&V3c::new(x, y, z)).is_some()
                        && *tree.get(&V3c::new(x, y, z)).unwrap() == 1
                    {
                        hits += 1;
                    }
                }
            }
        }
        assert!(hits == 64);
    }

    #[test]
    fn test_insert_at_lod_with_simplify() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();

        // This will set the area equal to 8 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 2, 5).ok();

        assert!(*tree.get(&V3c::new(0, 0, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 0, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 0, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 1, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 1, 1)).unwrap() == 5);

        // This will set the area equal to 64 1-sized nodes:
        // a size-4 node includes 2 levels,
        // 1-sized nodes at the bottom level doesn't have children,
        // 2-sized nodes above have 8 children each
        // so one 4-sized node has 8*8 = 64 children
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, 1).ok();
        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    if tree.get(&V3c::new(x, y, z)).is_some()
                        && *tree.get(&V3c::new(x, y, z)).unwrap() == 1
                    {
                        hits += 1;
                    }
                }
            }
        }
        assert!(hits == 64);
    }

    #[test]
    fn test_simplifyable_insert_and_get() {
        let mut tree = Octree::<u32>::new(2).ok().unwrap();

        // The below set of values should be simplified to a single node
        tree.insert(&V3c::new(0, 0, 0), 5).ok();
        tree.insert(&V3c::new(0, 0, 1), 5).ok();
        tree.insert(&V3c::new(0, 1, 0), 5).ok();
        tree.insert(&V3c::new(0, 1, 1), 5).ok();
        tree.insert(&V3c::new(1, 0, 0), 5).ok();
        tree.insert(&V3c::new(1, 0, 1), 5).ok();
        tree.insert(&V3c::new(1, 1, 0), 5).ok();
        tree.insert(&V3c::new(1, 1, 1), 5).ok();

        // The below should brake the simplified node back to its party
        tree.insert(&V3c::new(0, 0, 0), 4).ok();

        // Integrity should be kept
        assert!(*tree.get(&V3c::new(0, 0, 0)).unwrap() == 4);
        assert!(*tree.get(&V3c::new(0, 0, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 0, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 1, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 1, 1)).unwrap() == 5);
    }

    #[test]
    fn test_simple_clear() {
        let mut tree = Octree::<u32>::new(2).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), 5).ok();
        tree.insert(&V3c::new(0, 1, 0), 6).ok();
        tree.insert(&V3c::new(0, 0, 1), 7).ok();
        tree.clear(&V3c::new(0, 0, 1)).ok();

        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == 6);
        assert!(tree.get(&V3c::new(0, 0, 1)).is_none());
        assert!(tree.get(&V3c::new(1, 1, 1)).is_none());
    }

    #[test]
    fn test_simplifyable_clear() {
        let mut tree = Octree::<u32>::new(2).ok().unwrap();

        // The below set of values should be simplified to a single node
        tree.insert(&V3c::new(0, 0, 0), 5).ok();
        tree.insert(&V3c::new(0, 0, 1), 5).ok();
        tree.insert(&V3c::new(0, 1, 0), 5).ok();
        tree.insert(&V3c::new(0, 1, 1), 5).ok();
        tree.insert(&V3c::new(1, 0, 0), 5).ok();
        tree.insert(&V3c::new(1, 0, 1), 5).ok();
        tree.insert(&V3c::new(1, 1, 0), 5).ok();
        tree.insert(&V3c::new(1, 1, 1), 5).ok();

        // The below should brake the simplified node back to its party
        tree.clear(&V3c::new(0, 0, 0)).ok();

        // Integrity should be kept
        assert!(tree.get(&V3c::new(0, 0, 0)).is_none());
        assert!(*tree.get(&V3c::new(0, 0, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 0, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 1, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 1, 1)).unwrap() == 5);
    }

    #[test]
    fn test_clear_at_lod() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();

        // This will set the area equal to 64 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, 5).ok();

        // This will clear an area equal to 8 1-sized nodes
        tree.clear_at_lod(&V3c::new(0, 0, 0), 2).ok();

        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    if tree.get(&V3c::new(x, y, z)).is_some()
                        && *tree.get(&V3c::new(x, y, z)).unwrap() == 5
                    {
                        hits += 1;
                    }
                }
            }
        }

        // number of hits should be the number of nodes set minus the number of nodes cleared
        assert!(hits == (64 - 8));
    }
}
