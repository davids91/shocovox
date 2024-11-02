mod octree_tests {
    use crate::octree::types::{Albedo, Octree, VoxelData};
    use crate::spatial::math::{offset_region, vector::V3c};

    #[test]
    fn test_simple_insert_and_get() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();
        let blue: Albedo = 0x0000FFFF.into();

        let mut tree = Octree::<Albedo>::new(2).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), red)
            .expect("insert to work");
        tree.insert(&V3c::new(0, 1, 0), green)
            .expect("insert to work");
        tree.insert(&V3c::new(0, 0, 1), blue)
            .expect("insert to work");

        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == red);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == green);
        assert!(*tree.get(&V3c::new(0, 0, 1)).unwrap() == blue);
        assert!(tree.get(&V3c::new(1, 1, 1)).is_none());
    }

    #[test]
    fn test_simple_insert_and_get_where_dim_is_2() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();
        let blue: Albedo = 0x0000FFFF.into();

        let mut tree = Octree::<Albedo, 2>::new(4).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), red).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 0), green).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 1), blue).ok().unwrap();
        assert!(tree.get(&V3c::new(1, 0, 0)).is_some_and(|v| *v == red));
        assert!(tree.get(&V3c::new(0, 1, 0)).is_some_and(|v| *v == green));
        assert!(tree.get(&V3c::new(0, 0, 1)).is_some_and(|v| *v == blue));

        tree.insert(&V3c::new(3, 0, 0), red).ok().unwrap();
        tree.insert(&V3c::new(0, 3, 0), green).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 3), blue).ok().unwrap();
        assert!(tree.get(&V3c::new(3, 0, 0)).is_some_and(|v| *v == red));
        assert!(tree.get(&V3c::new(0, 3, 0)).is_some_and(|v| *v == green));
        assert!(tree.get(&V3c::new(0, 0, 3)).is_some_and(|v| *v == blue));
    }

    #[test]
    fn test_get_mut() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();
        let blue: Albedo = 0x0000FFFF.into();

        let mut tree = Octree::<Albedo>::new(2).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), red).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 0), green).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 1), blue).ok().unwrap();

        assert!(*tree.get_mut(&V3c::new(1, 0, 0)).unwrap() == red);
        assert!(*tree.get_mut(&V3c::new(0, 1, 0)).unwrap() == green);
        assert!(*tree.get_mut(&V3c::new(0, 0, 1)).unwrap() == blue);
        assert!(tree.get_mut(&V3c::new(1, 1, 1)).is_none());
    }

    #[test]
    fn test_insert_at_lod__() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();

        let mut tree = Octree::<Albedo>::new(4).ok().unwrap();
        tree.auto_simplify = false;

        // This will set the area equal to 8 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 2, red).ok().unwrap();

        assert!(*tree.get(&V3c::new(0, 0, 0)).unwrap() == red);
        assert!(*tree.get(&V3c::new(0, 0, 1)).unwrap() == red);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == red);
        assert!(*tree.get(&V3c::new(0, 1, 1)).unwrap() == red);
        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == red);
        assert!(*tree.get(&V3c::new(1, 0, 1)).unwrap() == red);
        assert!(*tree.get(&V3c::new(1, 1, 0)).unwrap() == red);
        assert!(*tree.get(&V3c::new(1, 1, 1)).unwrap() == red);

        // This will set the area equal to 64 1-sized nodes:
        // a size-4 node includes 2 levels,
        // 1-sized nodes at the bottom level doesn't have children,
        // 2-sized nodes above have 8 children each
        // so one 4-sized node has 8*8 = 64 children
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, green)
            .ok()
            .unwrap();
        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    if let Some(hit) = tree.get(&V3c::new(x, y, z)) {
                        assert!(*hit == green);
                        hits += 1;
                    }
                }
            }
        }
        assert!(hits == 64);
    }

    #[test]
    fn test_insert_at_lod_where_dim_is_2() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();

        let mut tree = Octree::<Albedo, 2>::new(4).ok().unwrap();
        tree.auto_simplify = false;

        // This will set the area equal to 8 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 2, red).ok().unwrap();

        assert!(*tree.get(&V3c::new(0, 0, 0)).unwrap() == red);
        assert!(*tree.get(&V3c::new(0, 0, 1)).unwrap() == red);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == red);
        assert!(*tree.get(&V3c::new(0, 1, 1)).unwrap() == red);
        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == red);
        assert!(*tree.get(&V3c::new(1, 0, 1)).unwrap() == red);
        assert!(*tree.get(&V3c::new(1, 1, 0)).unwrap() == red);
        assert!(*tree.get(&V3c::new(1, 1, 1)).unwrap() == red);

        // This will set the area equal to 64 1-sized nodes:
        // a size-4 node includes 2 levels,
        // 1-sized nodes at the bottom level doesn't have children,
        // 2-sized nodes above have 8 children each
        // so one 4-sized node has 8*8 = 64 children
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, green)
            .ok()
            .unwrap();
        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    if let Some(hit) = tree.get(&V3c::new(x, y, z)) {
                        assert!(*hit == green);
                        hits += 1;
                    }
                }
            }
        }
        assert!(hits == 64);
    }

    #[test]
    fn test_case_simplified_insert_separated_by_clear_with_aligned_dim() {
        let tree_size = 8;
        const MATRIX_DIMENSION: usize = 1;
        let red: Albedo = 0xFF0000FF.into();
        let mut tree = Octree::<Albedo, MATRIX_DIMENSION>::new(tree_size)
            .ok()
            .unwrap();

        for x in 0..tree_size {
            for y in 0..tree_size {
                for z in 0..tree_size {
                    tree.insert(&V3c::new(x, y, z), red).ok().unwrap();
                }
            }
        }

        tree.clear(&V3c::new(3, 3, 3)).ok().unwrap();
        let item_at_333 = tree.get(&V3c::new(3, 3, 3));
        assert!(item_at_333.is_none() || item_at_333.is_some_and(|v| v.is_empty()));

        let mut hits = 0;
        for x in 0..tree_size {
            for y in 0..tree_size {
                for z in 0..tree_size {
                    if let Some(hit) = tree.get(&V3c::new(x, y, z)) {
                        assert!(*hit == red);
                        hits += 1;
                    }
                }
            }
        }

        assert!(hits == 511, "Expected 511 hits instead of {hits}");
    }

    #[test]
    fn test_case_simplified_insert_separated_by_clear_where_dim_is_2() {
        let tree_size = 8;
        const MATRIX_DIMENSION: usize = 2;
        let red: Albedo = 0xFF0000FF.into();
        let mut tree = Octree::<Albedo, MATRIX_DIMENSION>::new(tree_size)
            .ok()
            .unwrap();

        for x in 0..tree_size {
            for y in 0..tree_size {
                for z in 0..tree_size {
                    tree.insert(&V3c::new(x, y, z), red).ok().unwrap();
                }
            }
        }

        tree.clear(&V3c::new(3, 3, 3)).ok().unwrap();
        let item_at_333 = tree.get(&V3c::new(3, 3, 3));
        assert!(item_at_333.is_none() || item_at_333.is_some_and(|v| v.is_empty()));

        let mut hits = 0;
        for x in 0..tree_size {
            for y in 0..tree_size {
                for z in 0..tree_size {
                    if let Some(hit) = tree.get(&V3c::new(x, y, z)) {
                        assert!(*hit == red);
                        hits += 1;
                    }
                }
            }
        }

        assert!(hits == 511);
    }

    #[test]
    fn test_case_simplified_insert_separated_by_clear_where_dim_is_4() {
        let tree_size = 8;
        const MATRIX_DIMENSION: usize = 4;
        let red: Albedo = 0xFF0000FF.into();
        let mut tree = Octree::<Albedo, MATRIX_DIMENSION>::new(tree_size)
            .ok()
            .unwrap();

        for x in 0..tree_size {
            for y in 0..tree_size {
                for z in 0..tree_size {
                    tree.insert(&V3c::new(x, y, z), red).ok().unwrap();
                }
            }
        }

        tree.clear(&V3c::new(3, 3, 3)).ok().unwrap();
        let item_at_000 = tree.get(&V3c::new(3, 3, 3));
        assert!(item_at_000.is_none() || item_at_000.is_some_and(|v| v.is_empty()));

        let mut hits = 0;
        for x in 0..tree_size {
            for y in 0..tree_size {
                for z in 0..tree_size {
                    if let Some(hit) = tree.get(&V3c::new(x, y, z)) {
                        assert!(*hit == red);
                        hits += 1;
                    }
                }
            }
        }

        assert!(hits == 511);
    }

    #[test]
    fn test_uniform_solid_leaf_separated_by_clear__() {
        let tree_size = 2;
        const MATRIX_DIMENSION: usize = 1;
        let mut tree = Octree::<Albedo, MATRIX_DIMENSION>::new(tree_size)
            .ok()
            .unwrap();

        // Fill each octant of the leaf with the same data, it should become a uniform leaf
        let color_base_original = 0xFFFF00FF;

        for octant in 0..8 {
            let start_pos = V3c::<u32>::from(offset_region(octant));
            tree.insert(&start_pos, color_base_original.into())
                .ok()
                .unwrap();
        }

        let item_at_000 = tree.get(&V3c::unit(0)).unwrap();
        assert!(*item_at_000 == color_base_original.into());

        // Separate Uniform leaf by clearing a voxel
        tree.clear(&V3c::unit(0)).ok().unwrap();
        assert!(tree.get(&V3c::unit(0)).is_none());

        // The rest of the voxels should remain intact
        for octant in 1..8 {
            let start_pos = V3c::<u32>::from(offset_region(octant));
            assert!(*tree.get(&start_pos).unwrap() == color_base_original.into());
        }
    }

    #[test]
    fn test_uniform_solid_leaf_separated_by_insert__() {
        let tree_size = 2;
        const MATRIX_DIMENSION: usize = 1;
        let mut tree = Octree::<Albedo, MATRIX_DIMENSION>::new(tree_size)
            .ok()
            .unwrap();

        // Fill each octant of the leaf with the same data, it should become a uniform leaf
        let color_base_original = 0xFFFF00FF;

        for octant in 0..8 {
            let start_pos = V3c::<u32>::from(offset_region(octant));
            tree.insert(&start_pos, color_base_original.into())
                .ok()
                .unwrap();
        }

        let item_at_000 = tree.get(&V3c::unit(0)).unwrap();
        assert!(*item_at_000 == color_base_original.into());

        // Separate Uniform leaf by overwriting a voxel
        tree.insert(&V3c::unit(0), 0x000000FF.into()).ok().unwrap();
        assert!(tree
            .get(&V3c::unit(0))
            .is_some_and(|v| *v == 0x000000FF.into()));

        // The rest of the voxels should remain intact
        for octant in 1..8 {
            let start_pos = V3c::<u32>::from(offset_region(octant));
            assert!(*tree.get(&start_pos).unwrap() == color_base_original.into());
        }
    }

    #[test]
    fn test_uniform_parted_brick_leaf_separated_by_clear_where_dim_is_4() {
        let tree_size = 4;
        const MATRIX_DIMENSION: usize = 2;
        let mut tree = Octree::<Albedo, MATRIX_DIMENSION>::new(tree_size)
            .ok()
            .unwrap();

        // Fill each octant of the leaf with the same data, it should become a uniform leaf
        let color_base_original = 0xFFFF00FF;
        let mut color_base = color_base_original;
        for x in 0..(MATRIX_DIMENSION / 2) as u32 {
            for y in 0..(MATRIX_DIMENSION / 2) as u32 {
                for z in 0..(MATRIX_DIMENSION / 2) as u32 {
                    for octant in 0..8 {
                        let start_pos =
                            V3c::<u32>::from(offset_region(octant)) * (MATRIX_DIMENSION as u32 / 2);
                        tree.insert(&(start_pos + V3c::new(x, y, z)), color_base.into())
                            .ok()
                            .unwrap();
                    }
                    color_base += 0xAA;
                }
            }
        }

        let item_at_000 = tree.get(&V3c::unit(0)).unwrap();
        assert!(*item_at_000 == color_base_original.into());

        // Separate Uniform leaf by clearing a voxel
        tree.clear(&V3c::unit(0)).ok().unwrap();
        assert!(tree.get(&V3c::unit(0)).is_none());

        // The rest of the voxels should remain intact
        color_base = color_base_original;
        for x in 0..(MATRIX_DIMENSION / 2) as u32 {
            for y in 0..(MATRIX_DIMENSION / 2) as u32 {
                for z in 0..(MATRIX_DIMENSION / 2) as u32 {
                    for octant in 0..8 {
                        if x == 0 && y == 0 && z == 0 && octant == 0 {
                            continue;
                        }
                        let start_pos =
                            V3c::<u32>::from(offset_region(octant)) * (MATRIX_DIMENSION as u32 / 2);
                        assert!(
                            *tree.get(&(start_pos + V3c::new(x, y, z))).unwrap()
                                == color_base.into()
                        );
                    }
                    color_base += 0xAA;
                }
            }
        }
    }

    #[test]
    fn test_uniform_solid_leaf_separated_by_clear_where_dim_is_4() {
        let tree_size = 8;
        const MATRIX_DIMENSION: usize = 4;
        let mut tree = Octree::<Albedo, MATRIX_DIMENSION>::new(tree_size)
            .ok()
            .unwrap();

        // Fill each octant with the same data, they should become a solid bricks
        let color_base = 0xFFFF00AA;
        for octant in 0..8 {
            let start_pos = V3c::<u32>::from(offset_region(octant)) * (MATRIX_DIMENSION as u32 / 2);
            for x in 0..(MATRIX_DIMENSION / 2) as u32 {
                for y in 0..(MATRIX_DIMENSION / 2) as u32 {
                    for z in 0..(MATRIX_DIMENSION / 2) as u32 {
                        tree.insert(
                            &(start_pos + V3c::new(x, y, z)),
                            (color_base + octant as u32).into(),
                        )
                        .ok()
                        .unwrap();
                    }
                }
            }
        }

        let item_at_000 = tree.get(&V3c::unit(0)).unwrap();
        assert!(*item_at_000 == color_base.into());

        // Separate Uniform leaf by clearing a voxel
        tree.clear(&V3c::unit(0)).ok().unwrap();
        assert!(tree.get(&V3c::unit(0)).is_none());

        // The rest of the voxels should remain intact
        for octant in 0..8 {
            let start_pos = V3c::<u32>::from(offset_region(octant)) * (MATRIX_DIMENSION as u32 / 2);
            for x in 0..(MATRIX_DIMENSION / 2) as u32 {
                for y in 0..(MATRIX_DIMENSION / 2) as u32 {
                    for z in 0..(MATRIX_DIMENSION / 2) as u32 {
                        if x == 0 && y == 0 && z == 0 && octant == 0 {
                            continue;
                        }
                        assert!(
                            *tree.get(&(start_pos + V3c::new(x, y, z))).unwrap()
                                == (color_base + octant as u32).into(),
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_uniform_solid_leaf_separated_by_insert_where_dim_is_4() {
        let tree_size = 8;
        const MATRIX_DIMENSION: usize = 4;
        let mut tree = Octree::<Albedo, MATRIX_DIMENSION>::new(tree_size)
            .ok()
            .unwrap();

        // Fill each octant with the same data, they should become a solid bricks
        let color_base = 0xFFFF00AA;
        for octant in 0..8 {
            let start_pos = V3c::<u32>::from(offset_region(octant)) * (MATRIX_DIMENSION as u32 / 2);
            for x in 0..(MATRIX_DIMENSION / 2) as u32 {
                for y in 0..(MATRIX_DIMENSION / 2) as u32 {
                    for z in 0..(MATRIX_DIMENSION / 2) as u32 {
                        tree.insert(
                            &(start_pos + V3c::new(x, y, z)),
                            (color_base + octant as u32).into(),
                        )
                        .ok()
                        .unwrap();
                    }
                }
            }
        }

        let item_at_000 = tree.get(&V3c::unit(0)).unwrap();
        assert!(*item_at_000 == color_base.into());

        // Separate Uniform leaf by overwriting a voxel
        tree.insert(&V3c::unit(0), 0x000000FF.into()).ok().unwrap();
        assert!(tree
            .get(&V3c::unit(0))
            .is_some_and(|v| *v == 0x000000FF.into()));

        // The rest of the voxels should remain intact
        for octant in 0..8 {
            let start_pos = V3c::<u32>::from(offset_region(octant)) * (MATRIX_DIMENSION as u32 / 2);
            for x in 0..(MATRIX_DIMENSION / 2) as u32 {
                for y in 0..(MATRIX_DIMENSION / 2) as u32 {
                    for z in 0..(MATRIX_DIMENSION / 2) as u32 {
                        if x == 0 && y == 0 && z == 0 && octant == 0 {
                            continue;
                        }
                        assert!(
                            *tree.get(&(start_pos + V3c::new(x, y, z))).unwrap()
                                == (color_base + octant as u32).into(),
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_uniform_parted_brick_leaf_separated_by_insert() {
        let tree_size = 4;
        const MATRIX_DIMENSION: usize = 2;
        let mut tree = Octree::<Albedo, MATRIX_DIMENSION>::new(tree_size)
            .ok()
            .unwrap();

        // Fill each octant of each brick with the same data, they should become a uniform leaf
        let color_base_original = 0xFFFF00FF;
        let mut color_base = color_base_original;
        for x in 0..(MATRIX_DIMENSION / 2) as u32 {
            for y in 0..(MATRIX_DIMENSION / 2) as u32 {
                for z in 0..(MATRIX_DIMENSION / 2) as u32 {
                    for octant in 0..8 {
                        let start_pos =
                            V3c::<u32>::from(offset_region(octant)) * (MATRIX_DIMENSION as u32 / 2);
                        tree.insert(&(start_pos + V3c::new(x, y, z)), color_base.into())
                            .ok()
                            .unwrap();
                    }
                    color_base += 0xAA;
                }
            }
        }

        let item_at_000 = tree.get(&V3c::unit(0)).unwrap();
        assert!(*item_at_000 == color_base_original.into());

        // Separate Uniform leaf by setting a voxel
        tree.insert(&V3c::unit(0), 0x000000FF.into()).ok().unwrap();
        assert!(tree
            .get(&V3c::unit(0))
            .is_some_and(|v| *v == 0x000000FF.into()));

        // The rest of the voxels should remain intact
        color_base = color_base_original;
        for x in 0..(MATRIX_DIMENSION / 2) as u32 {
            for y in 0..(MATRIX_DIMENSION / 2) as u32 {
                for z in 0..(MATRIX_DIMENSION / 2) as u32 {
                    for octant in 0..8 {
                        if x == 0 && y == 0 && z == 0 && octant == 0 {
                            continue;
                        }
                        let start_pos =
                            V3c::<u32>::from(offset_region(octant)) * (MATRIX_DIMENSION as u32 / 2);
                        assert!(
                            *tree.get(&(start_pos + V3c::new(x, y, z))).unwrap()
                                == color_base.into()
                        );
                    }
                    color_base += 0xAA;
                }
            }
        }
    }

    #[test]
    fn test_insert_at_lod_with_unaligned_position_where_dim_is_4() {
        let red: Albedo = 0xFF0000FF.into();

        let mut tree = Octree::<Albedo, 4>::new(8).ok().unwrap();
        tree.auto_simplify = false;

        tree.insert_at_lod(&V3c::new(3, 3, 3), 4, red).ok().unwrap();

        // This will set the area equal to 64 1-sized nodes:
        // a size-4 node includes 2 levels,
        // 1-sized nodes at the bottom level doesn't have children,
        // 2-sized nodes above have 8 children each
        // so one 4-sized node has 8*8 = 64 children
        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    if let Some(hit) = tree.get(&V3c::new(x, y, z)) {
                        assert!(*hit == red);
                        hits += 1;
                    }
                }
            }
        }
        assert!(hits == 64);
    }

    #[test]
    fn test_insert_at_lod_with_unaligned_size_where_dim_is_1() {
        let red: Albedo = 0xFF0000FF.into();

        let mut tree = Octree::<Albedo>::new(8).ok().unwrap();
        tree.auto_simplify = false;

        tree.insert_at_lod(&V3c::new(3, 3, 3), 3, red).ok().unwrap();
        let mut hits = 0;
        for x in 0..8 {
            for y in 0..8 {
                for z in 0..8 {
                    if let Some(hit) = tree.get(&V3c::new(x, y, z)) {
                        assert!(*hit == red);
                        hits += 1;
                    }
                }
            }
        }
        assert!(hits == 8);
    }

    #[test]
    fn test_insert_at_lod_with_unaligned_size_where_dim_is_4() {
        let red: Albedo = 0xFF0000FF.into();

        let mut tree = Octree::<Albedo, 4>::new(8).ok().unwrap();
        tree.auto_simplify = false;

        tree.insert_at_lod(&V3c::new(3, 3, 3), 3, red).ok().unwrap();

        assert!(tree.get(&V3c::new(1, 1, 1)).is_some_and(|v| *v == red));
        let mut hits = 0;
        for x in 0..8 {
            for y in 0..8 {
                for z in 0..8 {
                    if let Some(hit) = tree.get(&V3c::new(x, y, z)) {
                        assert!(*hit == red);
                        hits += 1;
                    }
                }
            }
        }
        assert!(hits == 27);
    }

    #[test]
    fn test_insert_at_lod_with_simplify() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();

        let mut tree = Octree::<Albedo>::new(8).ok().unwrap();

        // This will set the area equal to 8 1-sized nodes
        tree.insert_at_lod(&V3c::new(5, 0, 0), 2, red).ok().unwrap();

        assert!(*tree.get(&V3c::new(4, 0, 0)).unwrap() == red);
        assert!(*tree.get(&V3c::new(4, 0, 1)).unwrap() == red);
        assert!(*tree.get(&V3c::new(4, 1, 0)).unwrap() == red);
        assert!(*tree.get(&V3c::new(4, 1, 1)).unwrap() == red);
        assert!(*tree.get(&V3c::new(5, 0, 0)).unwrap() == red);
        assert!(*tree.get(&V3c::new(5, 0, 1)).unwrap() == red);
        assert!(*tree.get(&V3c::new(5, 1, 0)).unwrap() == red);
        assert!(*tree.get(&V3c::new(5, 1, 1)).unwrap() == red);

        // This will set the area equal to 64 1-sized nodes:
        // a size-4 node includes 2 levels,
        // 1-sized nodes at the bottom level doesn't have children,
        // 2-sized nodes above have 8 children each
        // so one 4-sized node has 8*8 = 64 children
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, green)
            .ok()
            .unwrap();
        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    if let Some(hit) = tree.get(&V3c::new(x, y, z)) {
                        assert!(*hit == green);
                        hits += 1;
                    }
                }
            }
        }

        for x in 4..6 {
            for y in 0..2 {
                for z in 0..2 {
                    if let Some(hit) = tree.get(&V3c::new(x, y, z)) {
                        assert!(*hit == red);
                        hits += 1;
                    }
                }
            }
        }
        assert!(hits == (64 + 8), "Expected 64 + 8 hits instead of {hits}");
    }

    #[test]
    fn test_simplifyable_insert_and_get() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();

        const SIZE: u32 = 2;
        let mut tree = Octree::<Albedo>::new(SIZE).ok().unwrap();

        // The below set of values should be simplified to a single node
        for x in 0..SIZE {
            for y in 0..SIZE {
                for z in 0..SIZE {
                    tree.insert(&V3c::new(x, y, z), red).ok().unwrap();
                }
            }
        }

        // The below should brake the simplified node back to its parts
        tree.insert(&V3c::new(0, 0, 0), green).ok().unwrap();

        // Integrity should be kept
        assert!(tree.get(&V3c::new(0, 0, 0)).is_some_and(|v| *v == green));
        for x in 1..SIZE {
            for y in 1..SIZE {
                for z in 1..SIZE {
                    assert!(tree.get(&V3c::new(x, y, z)).is_some_and(|v| *v == red));
                }
            }
        }
    }

    #[test]
    fn test_simplifyable_insert_and_get_where_dim_is_2() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();

        const SIZE: u32 = 4;
        let mut tree = Octree::<Albedo, 2>::new(SIZE).ok().unwrap();

        // The below set of values should be simplified to a single node
        for x in 0..SIZE {
            for y in 0..SIZE {
                for z in 0..SIZE {
                    tree.insert(&V3c::new(x, y, z), red).ok().unwrap();
                }
            }
        }

        // The below should brake the simplified node back to its parts
        tree.insert(&V3c::new(0, 0, 0), green).ok().unwrap();

        // Integrity should be kept
        assert!(tree.get(&V3c::new(0, 0, 0)).is_some_and(|v| *v == green));
        for x in 1..SIZE {
            for y in 1..SIZE {
                for z in 1..SIZE {
                    assert!(tree.get(&V3c::new(x, y, z)).is_some_and(|v| *v == red));
                }
            }
        }
    }

    #[test]
    fn test_simple_clear_with_aligned_dim() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();
        let blue: Albedo = 0x0000FFFF.into();

        let mut tree = Octree::<Albedo>::new(2).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), red).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 0), green).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 1), blue).ok().unwrap();
        tree.clear(&V3c::new(0, 0, 1)).ok().unwrap();

        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == red);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == green);
        let item_at_001 = tree.get(&V3c::new(0, 0, 1));
        assert!(item_at_001.is_none() || item_at_001.is_some_and(|v| v.is_empty()));
        let item_at_111 = tree.get(&V3c::new(1, 1, 1));
        assert!(item_at_111.is_none() || item_at_111.is_some_and(|v| v.is_empty()));
    }

    #[test]
    fn test_simple_clear_where_dim_is_2() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();
        let blue: Albedo = 0x0000FFFF.into();

        let mut tree = Octree::<Albedo, 2>::new(4).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), red).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 0), green).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 1), blue).ok().unwrap();
        tree.clear(&V3c::new(0, 0, 1)).ok().unwrap();

        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == red);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == green);
        let item_at_001 = tree.get(&V3c::new(0, 0, 1));
        assert!(item_at_001.is_none() || item_at_001.is_some_and(|v| v.is_empty()));
        let item_at_111 = tree.get(&V3c::new(1, 1, 1));
        assert!(item_at_111.is_none() || item_at_111.is_some_and(|v| v.is_empty()));
    }

    #[test]
    fn test_double_clear() {
        let albedo_black: Albedo = 0x000000FF.into();
        let albedo_white: Albedo = 0xFFFFFFFF.into();
        let mut tree = Octree::<Albedo>::new(2).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), albedo_black).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 0), albedo_white).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 1), albedo_white).ok().unwrap();
        tree.clear(&V3c::new(0, 0, 1)).ok().unwrap();
        tree.clear(&V3c::new(0, 0, 1)).ok().unwrap();

        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == albedo_black);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == albedo_white);
        let item_at_001 = tree.get(&V3c::new(0, 0, 1));
        assert!(item_at_001.is_none() || item_at_001.is_some_and(|v| v.is_empty()));
    }

    #[test]
    fn test_simplifyable_clear() {
        let albedo: Albedo = 0xFFAAEEFF.into();
        const SIZE: u32 = 2;
        let mut tree = Octree::<Albedo>::new(SIZE).ok().unwrap();

        // The below set of values should be simplified to a single node
        for x in 0..SIZE {
            for y in 0..SIZE {
                for z in 0..SIZE {
                    tree.insert(&V3c::new(x, y, z), albedo).ok().unwrap();
                }
            }
        }

        // The below should brake the simplified node back to its party
        tree.clear(&V3c::new(0, 0, 0)).ok().unwrap();

        // Integrity should be kept
        let item_at_000 = tree.get(&V3c::new(0, 0, 0));
        assert!(item_at_000.is_none() || item_at_000.is_some_and(|v| v.is_empty()));
        for x in 1..SIZE {
            for y in 1..SIZE {
                for z in 1..SIZE {
                    assert!(tree.get(&V3c::new(x, y, z)).is_some_and(|v| *v == albedo));
                }
            }
        }
    }

    #[test]
    fn test_simplifyable_clear_where_dim_is_2() {
        let albedo: Albedo = 0xFFAAEEFF.into();
        const SIZE: u32 = 4;
        let mut tree = Octree::<Albedo, 2>::new(SIZE).ok().unwrap();

        // The below set of values should be simplified to a single node
        for x in 0..SIZE {
            for y in 0..SIZE {
                for z in 0..SIZE {
                    tree.insert(&V3c::new(x, y, z), albedo).ok().unwrap();
                }
            }
        }

        // The below should brake the simplified node back to its party
        tree.clear(&V3c::new(0, 0, 0)).ok().unwrap();

        // Integrity should be kept
        let item_at_000 = tree.get(&V3c::new(0, 0, 0));
        assert!(item_at_000.is_none() || item_at_000.is_some_and(|v| v.is_empty()));
        for x in 1..SIZE {
            for y in 1..SIZE {
                for z in 1..SIZE {
                    assert!(tree.get(&V3c::new(x, y, z)).is_some_and(|v| *v == albedo));
                }
            }
        }
    }

    #[test]
    fn test_clear_to_nothing() {
        let albedo: Albedo = 0xFFAAEEFF.into();
        let mut tree = Octree::<Albedo>::new(2).ok().unwrap();

        // The below set of values should be simplified to a single node
        tree.insert(&V3c::new(0, 0, 0), albedo).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 1), albedo).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 0), albedo).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 1), albedo).ok().unwrap();
        tree.insert(&V3c::new(1, 0, 0), albedo).ok().unwrap();
        tree.insert(&V3c::new(1, 0, 1), albedo).ok().unwrap();
        tree.insert(&V3c::new(1, 1, 0), albedo).ok().unwrap();
        tree.insert(&V3c::new(1, 1, 1), albedo).ok().unwrap();

        // The below should brake the simplified node back to its party
        tree.clear_at_lod(&V3c::new(0, 0, 0), 2).ok().unwrap();

        // Nothing should remain in the tree
        assert!(tree.get(&V3c::new(0, 0, 0)).is_none());
        assert!(tree.get(&V3c::new(0, 0, 1)).is_none());
        assert!(tree.get(&V3c::new(0, 1, 0)).is_none());
        assert!(tree.get(&V3c::new(0, 1, 1)).is_none());
        assert!(tree.get(&V3c::new(1, 0, 0)).is_none());
        assert!(tree.get(&V3c::new(1, 0, 1)).is_none());
        assert!(tree.get(&V3c::new(1, 1, 0)).is_none());
        assert!(tree.get(&V3c::new(1, 1, 1)).is_none());
    }

    #[test]
    fn test_clear_at_lod_with_aligned_dim() {
        let albedo: Albedo = 0xFFAAEEFF.into();
        let mut tree = Octree::<Albedo>::new(4).ok().unwrap();

        // This will set the area equal to 64 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, albedo)
            .ok()
            .unwrap();

        // This will clear an area equal to 8 1-sized nodes
        tree.clear_at_lod(&V3c::new(0, 0, 0), 2).ok().unwrap();

        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    if let Some(x) = tree.get(&V3c::new(x, y, z)) {
                        assert_eq!(*x, albedo);
                        hits += 1;
                    }
                }
            }
        }

        // number of hits should be the number of nodes set minus the number of nodes cleared
        assert!(hits == (64 - 8));
    }

    #[test]
    fn test_clear_at_lod_where_dim_is_2() {
        let albedo: Albedo = 0xFFAAEEFF.into();
        let mut tree = Octree::<Albedo, 2>::new(4).ok().unwrap();

        // This will set the area equal to 64 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, albedo)
            .ok()
            .unwrap();

        // This will clear an area equal to 8 1-sized nodes
        tree.clear_at_lod(&V3c::new(0, 0, 0), 2).ok().unwrap();

        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    if let Some(x) = tree.get(&V3c::new(x, y, z)) {
                        assert_eq!(*x, albedo);
                        hits += 1;
                    }
                }
            }
        }

        // number of hits should be the number of nodes set minus the number of nodes cleared
        assert!(hits == (64 - 8));
    }

    #[test]
    fn test_clear_at_lod_with_unaligned_position() {
        let albedo: Albedo = 0xFFAAEEFF.into();
        let mut tree = Octree::<Albedo>::new(4).ok().unwrap();

        // This will set the area equal to 64 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, albedo)
            .ok()
            .unwrap();

        // This will clear an area equal to 8 1-sized nodes
        tree.clear_at_lod(&V3c::new(1, 1, 1), 2).ok().unwrap();

        // unset voxels should not be present
        assert!(tree.get(&V3c::new(0, 0, 0)).is_none());
        assert!(tree.get(&V3c::new(0, 0, 1)).is_none());
        assert!(tree.get(&V3c::new(0, 1, 0)).is_none());
        assert!(tree.get(&V3c::new(0, 1, 1)).is_none());
        assert!(tree.get(&V3c::new(1, 0, 0)).is_none());
        assert!(tree.get(&V3c::new(1, 0, 1)).is_none());
        assert!(tree.get(&V3c::new(1, 1, 0)).is_none());
        assert!(tree.get(&V3c::new(1, 1, 1)).is_none());

        // sampling some voxels who should be present
        assert!(tree.get(&V3c::new(0, 0, 2)).is_some());
        assert!(tree.get(&V3c::new(0, 2, 0)).is_some());
        assert!(tree.get(&V3c::new(0, 2, 2)).is_some());
        assert!(tree.get(&V3c::new(2, 0, 0)).is_some());
        assert!(tree.get(&V3c::new(2, 0, 2)).is_some());
        assert!(tree.get(&V3c::new(2, 2, 0)).is_some());
        assert!(tree.get(&V3c::new(2, 2, 2)).is_some());

        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    if let Some(x) = tree.get(&V3c::new(x, y, z)) {
                        assert_eq!(*x, albedo);
                        hits += 1
                    }
                }
            }
        }

        // number of hits should be the number of nodes set minus the number of nodes cleared
        assert!(hits == (64 - 8));
    }

    #[test]
    fn test_clear_at_lod_with_unaligned_position_where_dim_is_4() {
        let albedo: Albedo = 0xFFAAEEFF.into();
        let mut tree = Octree::<Albedo, 4>::new(8).ok().unwrap();

        tree.insert_at_lod(&V3c::new(0, 0, 0), 8, albedo)
            .ok()
            .unwrap();
        tree.clear_at_lod(&V3c::new(1, 1, 1), 4).ok().unwrap();

        let mut hits = 0;
        for x in 0..8 {
            for y in 0..8 {
                for z in 0..8 {
                    if let Some(hit) = tree.get(&V3c::new(x, y, z)) {
                        assert_eq!(*hit, albedo);
                        hits += 1;
                    }
                }
            }
        }

        // number of hits should be the number of nodes set minus the number of nodes cleared
        assert!(hits == (512 - 64));
    }

    #[test]
    fn test_clear_at_lod_with_unaligned_size_where_dim_is_1() {
        let albedo: Albedo = 0xFFAAEEFF.into();
        let mut tree = Octree::<Albedo>::new(4).ok().unwrap();
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, albedo)
            .ok()
            .unwrap();
        tree.clear_at_lod(&V3c::new(0, 0, 0), 3).ok().unwrap();

        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    if let Some(hit) = tree.get(&V3c::new(x, y, z)) {
                        assert_eq!(*hit, albedo);
                        hits += 1;
                    }
                }
            }
        }

        // number of hits should be the number of nodes set minus the number of nodes cleared
        // in this case, clear size is taken as 2 as it is the smaller number where 2^x < clear_size < 2^(x+1)
        assert!(hits == (64 - 8));
    }

    #[test]
    fn test_clear_at_lod_with_unaligned_size_where_dim_is_4() {
        let albedo: Albedo = 0xFFAAEEFF.into();
        let mut tree = Octree::<Albedo, 4>::new(8).ok().unwrap();
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, albedo)
            .ok()
            .unwrap();
        tree.clear_at_lod(&V3c::new(0, 0, 0), 3).ok().unwrap();

        let mut hits = 0;
        for x in 0..8 {
            for y in 0..8 {
                for z in 0..8 {
                    if let Some(hit) = tree.get(&V3c::new(x, y, z)) {
                        assert_eq!(*hit, albedo);
                        hits += 1;
                    }
                }
            }
        }

        // number of hits should be the number of nodes set minus the number of nodes cleared
        assert!(hits == (64 - 27));
    }

    #[test]
    fn test_edge_case_octree_set() {
        // const TREE_SIZE: u32 = 128;
        // const FILL_RANGE_START: u32 = 100;
        const TREE_SIZE: u32 = 8;
        const FILL_RANGE_START: u32 = 6;
        let mut tree = Octree::<Albedo>::new(TREE_SIZE).ok().unwrap();
        for x in FILL_RANGE_START..TREE_SIZE {
            for y in FILL_RANGE_START..TREE_SIZE {
                for z in FILL_RANGE_START..TREE_SIZE {
                    let pos = V3c::new(x, y, z);
                    tree.insert(&pos, (x + y + z).into()).ok().unwrap();
                    assert!(tree.get(&pos).is_some_and(|v| *v == ((x + y + z).into())));
                }
            }
        }
    }

    #[test]
    fn test_case_inserting_empty() {
        let mut tree = Octree::<Albedo>::new(4).ok().unwrap();
        tree.insert(&V3c::new(3, 0, 0), 0.into()).ok().unwrap();
        let item = tree.get(&V3c::new(3, 0, 0));
        assert!(item.is_none(), "Item shouldn't exist: {:?}", item);
    }
}
