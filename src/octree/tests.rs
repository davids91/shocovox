mod brick_tests {
    use crate::{
        octree::{flat_projection, Albedo, BrickData, V3c, VoxelContent},
        spatial::lut::OCTANT_OFFSET_REGION_LUT,
    };

    #[test]
    fn test_octant_empty() {
        let color_palette = vec![Albedo::default().with_alpha(100); 1];
        let data_palette = vec![0u32; 1];
        let data = BrickData::<VoxelContent>::Empty;
        assert!(data.is_empty_throughout(0, 1, &color_palette, &data_palette),);
        assert!(data.is_empty_throughout(1, 1, &color_palette, &data_palette),);
        assert!(data.is_empty_throughout(2, 1, &color_palette, &data_palette),);
        assert!(data.is_empty_throughout(3, 1, &color_palette, &data_palette),);
        assert!(data.is_empty_throughout(4, 1, &color_palette, &data_palette),);
        assert!(data.is_empty_throughout(5, 1, &color_palette, &data_palette),);
        assert!(data.is_empty_throughout(6, 1, &color_palette, &data_palette),);
        assert!(data.is_empty_throughout(7, 1, &color_palette, &data_palette),);

        let data = BrickData::<VoxelContent>::Parted(vec![VoxelContent::visual(0); 1]);
        assert!(!data.is_empty_throughout(0, 1, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(1, 1, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(2, 1, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(3, 1, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(4, 1, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(5, 1, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(6, 1, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(7, 1, &color_palette, &data_palette),);
    }

    #[test]
    fn test_octant_empty_where_dim_is_2() {
        // Create a filled parted Brick
        let color_palette = vec![Albedo::default().with_alpha(100); 1];
        let data_palette = vec![0u32; 1];
        let mut data = BrickData::<VoxelContent>::Parted(vec![VoxelContent::visual(0); 2 * 2 * 2]);
        assert!(!data.is_empty_throughout(0, 2, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(1, 2, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(2, 2, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(3, 2, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(4, 2, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(5, 2, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(6, 2, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(7, 2, &color_palette, &data_palette),);

        // Erase an octant worth of data, it should be empty!
        let target_octant = 5;
        if let BrickData::Parted(ref mut brick) = data {
            let octant_offset = V3c::<usize>::from(OCTANT_OFFSET_REGION_LUT[target_octant]);
            let octant_flat_offset =
                flat_projection(octant_offset.x, octant_offset.y, octant_offset.z, 2);
            brick[octant_flat_offset] = VoxelContent::empty();
        }
        assert!(
            data.is_empty_throughout(target_octant, 2, &color_palette, &data_palette,),
            "Data cleared under octant should be empty"
        );
    }

    #[test]
    fn test_octant_empty_where_dim_is_4() {
        // Create a filled parted Brick
        let color_palette = vec![Albedo::default().with_alpha(100); 1];
        let data_palette = vec![0u32; 1];
        let mut data = BrickData::<VoxelContent>::Parted(vec![VoxelContent::visual(0); 4 * 4 * 4]);
        assert!(!data.is_empty_throughout(0, 4, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(1, 4, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(2, 4, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(3, 4, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(4, 4, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(5, 4, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(6, 4, &color_palette, &data_palette),);
        assert!(!data.is_empty_throughout(7, 4, &color_palette, &data_palette),);

        let target_octant = 5;
        // offset by half of the brick dimension, as half of the dim 4x4x4 translates to 2x2x2
        // which is the resolution of OCTANT_OFFSET_REGION_LUT
        let octant_offset = V3c::<usize>::from(OCTANT_OFFSET_REGION_LUT[target_octant] * 2.);

        // Erase part of the octant, it should still not be empty
        if let BrickData::Parted(ref mut brick) = data {
            let octant_flat_offset =
                flat_projection(octant_offset.x, octant_offset.y, octant_offset.z, 4);
            brick[octant_flat_offset] = VoxelContent::empty();
        }
        assert!(
            !data.is_empty_throughout(target_octant, 4, &color_palette, &data_palette,),
            "Data cleared under octant should not be empty"
        );

        // Erase an octant worth of data, it should be empty!
        if let BrickData::Parted(ref mut brick) = data {
            for x in 0..2 {
                for y in 0..2 {
                    for z in 0..2 {
                        let octant_flat_offset = flat_projection(
                            octant_offset.x + x,
                            octant_offset.y + y,
                            octant_offset.z + z,
                            4,
                        );
                        brick[octant_flat_offset] = VoxelContent::empty();
                    }
                }
            }
        }
        assert!(
            data.is_empty_throughout(target_octant, 4, &color_palette, &data_palette),
            "Data cleared under octant should be empty"
        );
    }

    #[test]
    fn test_part_of_octant_empty() {
        // Create a filled parted Brick
        let color_palette = vec![Albedo::default().with_alpha(100); 1];
        let data_palette = vec![0u32; 1];
        let mut data = BrickData::<VoxelContent>::Parted(vec![VoxelContent::visual(0); 4 * 4 * 4]);
        for i in 0..8 {
            for j in 0..8 {
                assert!(!data.is_part_empty_throughout(i, j, 4, &color_palette, &data_palette));
            }
        }

        let target_octant = 5;
        // offset by half of the brick dimension, as half of the dim 4x4x4 translates to 2x2x2
        // which is the resolution of OCTANT_OFFSET_REGION_LUT
        let octant_offset = V3c::<usize>::from(OCTANT_OFFSET_REGION_LUT[target_octant] * 2.);

        // Erase part of the octant, the relevant part should be empty, while others should not be
        if let BrickData::Parted(ref mut brick) = data {
            let octant_flat_offset =
                flat_projection(octant_offset.x, octant_offset.y, octant_offset.z, 4);
            brick[octant_flat_offset] = VoxelContent::empty();
        }
        assert!(
            data.is_part_empty_throughout(target_octant, 0, 4, &color_palette, &data_palette),
            "Data cleared under part of octant should be empty"
        );
        assert!(
            !data.is_part_empty_throughout(target_octant, 1, 4, &color_palette, &data_palette),
            "Data not cleared should not be empty"
        );
    }
}

mod octree_tests {
    use crate::octree::types::{Albedo, Octree, OctreeEntry};
    use crate::spatial::{lut::OCTANT_OFFSET_REGION_LUT, math::vector::V3c};
    use crate::voxel_data;
    use num_traits::Zero;

    #[test]
    fn test_simple_insert_and_get() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();
        let blue: Albedo = 0x0000FFFF.into();

        let mut tree: Octree = Octree::new(2, 1).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), &red)
            .expect("insert to work");
        tree.insert(&V3c::new(0, 1, 0), &green)
            .expect("insert to work");
        tree.insert(&V3c::new(0, 0, 1), &blue)
            .expect("insert to work");

        assert!(tree.get(&V3c::new(1, 0, 0)) == (&red).into());
        assert!(tree.get(&V3c::new(0, 1, 0)) == (&green).into());
        assert!(tree.get(&V3c::new(0, 0, 1)) == (&blue).into());
        assert!(tree.get(&V3c::new(1, 1, 1)) == OctreeEntry::Empty);

        // Overwrite some data as well
        tree.insert(&V3c::new(1, 0, 0), &green)
            .expect("insert to work");
        assert!(tree.get(&V3c::new(1, 0, 0)) == (&green).into());
        assert!(tree.get(&V3c::new(0, 1, 0)) == (&green).into());
        assert!(tree.get(&V3c::new(0, 0, 1)) == (&blue).into());
        assert!(tree.get(&V3c::new(1, 1, 1)) == OctreeEntry::Empty);
    }

    #[test]
    fn test_complex_insert_and_get() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();

        let mut tree: Octree = Octree::new(2, 1).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), (&red, &0))
            .expect("insert to work");
        tree.insert(&V3c::new(0, 1, 0), (&green, &1))
            .expect("insert to work");
        tree.insert(&V3c::new(0, 0, 1), voxel_data!(&2))
            .expect("insert to work");

        assert!(tree.get(&V3c::new(1, 0, 0)) == (&red, &0).into());
        assert!(tree.get(&V3c::new(0, 1, 0)) == (&green, &1).into());
        assert!(tree.get(&V3c::new(0, 0, 1)) == voxel_data!(&2));
        assert!(tree.get(&V3c::new(1, 1, 1)) == OctreeEntry::Empty);

        // Overwrite some data as well
        tree.insert(&V3c::new(1, 0, 0), voxel_data!(&3))
            .expect("insert to work");
        assert!(tree.get(&V3c::new(1, 0, 0)) == voxel_data!(&3));
        assert!(tree.get(&V3c::new(0, 1, 0)) == (&green, &1).into());
        assert!(tree.get(&V3c::new(0, 0, 1)) == voxel_data!(&2));
        assert!(tree.get(&V3c::new(1, 1, 1)) == OctreeEntry::Empty);
    }

    #[test]
    fn test_simple_insert_and_get_where_dim_is_2() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();
        let blue: Albedo = 0x0000FFFF.into();

        let mut tree: Octree = Octree::new(4, 2).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), &red).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 0), &green).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 1), &blue).ok().unwrap();
        assert!(tree.get(&V3c::new(1, 0, 0)) == (&red).into());
        assert!(tree.get(&V3c::new(0, 1, 0)) == (&green).into());
        assert!(tree.get(&V3c::new(0, 0, 1)) == (&blue).into());

        tree.insert(&V3c::new(3, 0, 0), &red).ok().unwrap();
        tree.insert(&V3c::new(0, 3, 0), &green).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 3), &blue).ok().unwrap();
        assert!(tree.get(&V3c::new(3, 0, 0)) == (&red).into());
        assert!(tree.get(&V3c::new(0, 3, 0)) == (&green).into());
        assert!(tree.get(&V3c::new(0, 0, 3)) == (&blue).into());

        // Overwrite some data as well
        tree.insert(&V3c::new(1, 0, 0), &green)
            .expect("insert to work");
        assert!(tree.get(&V3c::new(1, 0, 0)) == (&green).into());
        assert!(tree.get(&V3c::new(0, 1, 0)) == (&green).into());
        assert!(tree.get(&V3c::new(0, 0, 1)) == (&blue).into());
    }

    #[test]
    fn test_insert_at_lod_with_aligned_dim() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();

        let mut tree: Octree = Octree::new(4, 1).ok().unwrap();
        tree.auto_simplify = false;

        // This will set the area equal to 8 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 2, &red)
            .ok()
            .unwrap();

        assert!(tree.get(&V3c::new(0, 0, 0)) == (&red).into());
        assert!(tree.get(&V3c::new(0, 0, 1)) == (&red).into());
        assert!(tree.get(&V3c::new(0, 1, 0)) == (&red).into());
        assert!(tree.get(&V3c::new(0, 1, 1)) == (&red).into());
        assert!(tree.get(&V3c::new(1, 0, 0)) == (&red).into());
        assert!(tree.get(&V3c::new(1, 0, 1)) == (&red).into());
        assert!(tree.get(&V3c::new(1, 1, 0)) == (&red).into());
        assert!(tree.get(&V3c::new(1, 1, 1)) == (&red).into());

        // This will set the area equal to 64 1-sized nodes:
        // a size-4 node includes 2 levels,
        // 1-sized nodes at the bottom level doesn't have children,
        // 2-sized nodes above have 8 children each
        // so one 4-sized node has 8*8 = 64 children
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, &green)
            .ok()
            .unwrap();
        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    let hit = tree.get(&V3c::new(x, y, z));
                    if hit != OctreeEntry::Empty {
                        assert!(
                            hit == (&green).into(),
                            "Hit mismatch at {:?}: {:?} <> {:?}",
                            (x, y, z),
                            hit,
                            green,
                        );
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

        let mut tree: Octree = Octree::new(4, 2).ok().unwrap();
        tree.auto_simplify = false;

        // This will set the area equal to 8 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 2, &red)
            .ok()
            .unwrap();

        assert!(tree.get(&V3c::new(0, 0, 0)) == (&red).into());
        assert!(tree.get(&V3c::new(0, 0, 1)) == (&red).into());
        assert!(tree.get(&V3c::new(0, 1, 0)) == (&red).into());
        assert!(tree.get(&V3c::new(0, 1, 1)) == (&red).into());
        assert!(tree.get(&V3c::new(1, 0, 0)) == (&red).into());
        assert!(tree.get(&V3c::new(1, 0, 1)) == (&red).into());
        assert!(tree.get(&V3c::new(1, 1, 0)) == (&red).into());
        assert!(tree.get(&V3c::new(1, 1, 1)) == (&red).into());

        // This will set the area equal to 64 1-sized nodes:
        // a size-4 node includes 2 levels,
        // 1-sized nodes at the bottom level doesn't have children,
        // 2-sized nodes above have 8 children each
        // so one 4-sized node has 8*8 = 64 children
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, &green)
            .ok()
            .unwrap();
        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    let hit = tree.get(&V3c::new(x, y, z));
                    if hit != OctreeEntry::Empty {
                        assert!(
                            hit == (&green).into(),
                            "Hit mismatch at {:?}: {:?} <> {:?}",
                            (x, y, z),
                            hit,
                            green,
                        );
                        hits += 1;
                    }
                }
            }
        }
        assert!(hits == 64);
    }

    #[test]
    fn test_case_simplified_insert_separated_by_clear_where_dim_is_1() {
        let tree_size = 8;
        const MATRIX_DIMENSION: u32 = 1;
        let red: Albedo = 0xFF0000FF.into();
        let mut tree: Octree = Octree::new(tree_size, MATRIX_DIMENSION).ok().unwrap();

        for x in 0..tree_size {
            for y in 0..tree_size {
                for z in 0..tree_size {
                    tree.insert(&V3c::new(x, y, z), &red).ok().unwrap();
                }
            }
        }

        tree.clear(&V3c::new(3, 3, 3)).ok().unwrap();
        let item_at_333 = tree.get(&V3c::new(3, 3, 3));
        assert!(item_at_333 == OctreeEntry::Empty);

        let mut hits = 0;
        for x in 0..tree_size {
            for y in 0..tree_size {
                for z in 0..tree_size {
                    let hit = tree.get(&V3c::new(x, y, z));
                    if hit != OctreeEntry::Empty {
                        assert!(
                            hit == (&red).into(),
                            "Hit mismatch at {:?}: {:?} <> {:?}",
                            (x, y, z),
                            hit,
                            red,
                        );
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
        const MATRIX_DIMENSION: u32 = 2;
        let red: Albedo = 0xFF0000FF.into();
        let mut tree: Octree = Octree::new(tree_size, MATRIX_DIMENSION).ok().unwrap();

        for x in 0..tree_size {
            for y in 0..tree_size {
                for z in 0..tree_size {
                    tree.insert(&V3c::new(x, y, z), &red).ok().unwrap();
                }
            }
        }

        let item_at_333 = tree.get(&V3c::new(3, 3, 3));
        assert!(
            item_at_333 == (&red).into(),
            "Hit mismatch at {:?}: {:?} <> {:?}",
            (3, 3, 3),
            item_at_333,
            red
        );
        tree.clear(&V3c::new(3, 3, 3)).ok().unwrap();
        let item_at_333 = tree.get(&V3c::new(3, 3, 3));
        assert!(
            item_at_333 == OctreeEntry::Empty,
            "Hit mismatch at {:?}: {:?} <> {:?}",
            (3, 3, 3),
            item_at_333,
            OctreeEntry::<u32>::Empty
        );

        let mut hits = 0;
        for x in 0..tree_size {
            for y in 0..tree_size {
                for z in 0..tree_size {
                    let hit = tree.get(&V3c::new(x, y, z));
                    if hit != OctreeEntry::Empty {
                        assert!(
                            hit == (&red).into(),
                            "Hit mismatch at {:?}: {:?} <> {:?}",
                            (x, y, z),
                            hit,
                            red
                        );
                        hits += 1;
                    }
                }
            }
        }

        assert!(hits == 511, "Expected 511 hits instead of {hits}");
    }

    #[test]
    fn test_case_simplified_insert_separated_by_clear_where_dim_is_4() {
        let tree_size = 8;
        const MATRIX_DIMENSION: u32 = 4;
        let red: Albedo = 0xFF0000FF.into();
        let mut tree: Octree = Octree::new(tree_size, MATRIX_DIMENSION).ok().unwrap();

        for x in 0..tree_size {
            for y in 0..tree_size {
                for z in 0..tree_size {
                    tree.insert(&V3c::new(x, y, z), &red).ok().unwrap();
                }
            }
        }

        tree.clear(&V3c::new(3, 3, 3)).ok().unwrap();
        let item_at_000 = tree.get(&V3c::new(3, 3, 3));
        assert!(item_at_000 == OctreeEntry::Empty);

        let mut hits = 0;
        for x in 0..tree_size {
            for y in 0..tree_size {
                for z in 0..tree_size {
                    let hit = tree.get(&V3c::new(x, y, z));
                    if hit != OctreeEntry::Empty {
                        assert!(
                            hit == (&red).into(),
                            "Hit mismatch at {:?}: {:?} <> {:?}",
                            (x, y, z),
                            hit,
                            red,
                        );
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
        const MATRIX_DIMENSION: u32 = 1;
        let mut tree: Octree = Octree::new(tree_size, MATRIX_DIMENSION).ok().unwrap();

        // Fill each octant of the leaf with the same data, it should become a uniform leaf
        let color_base_original: Albedo = 0xFFFF00FF.into();

        for octant in 0..8 {
            let start_pos = V3c::<u32>::from(OCTANT_OFFSET_REGION_LUT[octant]);
            tree.insert(&start_pos, &color_base_original).ok().unwrap();
        }

        let item_at_000 = tree.get(&V3c::unit(0));
        assert!(item_at_000 == (&color_base_original).into());

        // Separate Uniform leaf by clearing a voxel
        tree.clear(&V3c::unit(0)).ok().unwrap();
        assert!(tree.get(&V3c::unit(0)) == OctreeEntry::Empty);

        // The rest of the voxels should remain intact
        for octant in 1..8 {
            let start_pos = V3c::<u32>::from(OCTANT_OFFSET_REGION_LUT[octant]);
            assert!(tree.get(&start_pos) == (&color_base_original).into());
        }
    }

    #[test]
    fn test_uniform_solid_leaf_separated_by_insert__() {
        let tree_size = 2;
        const MATRIX_DIMENSION: u32 = 1;
        let mut tree: Octree = Octree::new(tree_size, MATRIX_DIMENSION).ok().unwrap();

        // Fill each octant of the leaf with the same data, it should become a uniform leaf
        let color_base_original: Albedo = 0xFFFF00FF.into();

        for octant in 0..8 {
            let start_pos = V3c::<u32>::from(OCTANT_OFFSET_REGION_LUT[octant]);
            tree.insert(&start_pos, &color_base_original).ok().unwrap();
        }

        let item_at_000 = tree.get(&V3c::unit(0));
        assert!(item_at_000 == (&color_base_original).into());

        // Separate Uniform leaf by overwriting a voxel
        let color_base_modified: Albedo = 0xFFFF00FF.into();
        tree.insert(&V3c::unit(0), &color_base_modified)
            .ok()
            .unwrap();
        assert!(tree.get(&V3c::unit(0)) == (&color_base_modified).into());

        // The rest of the voxels should remain intact
        for octant in 1..8 {
            let start_pos = V3c::<u32>::from(OCTANT_OFFSET_REGION_LUT[octant]);
            assert!(tree.get(&start_pos) == (&color_base_original).into());
        }
    }

    #[test]
    fn test_uniform_parted_brick_leaf_separated_by_clear_where_dim_is_4() {
        let tree_size = 4;
        const MATRIX_DIMENSION: u32 = 2;
        let mut tree: Octree = Octree::new(tree_size, MATRIX_DIMENSION).ok().unwrap();

        // Fill each octant of the leaf with the same data, it should become a uniform leaf
        let color_base_original = 0xFFFF00FF;
        let mut color_base = color_base_original;
        for x in 0..(MATRIX_DIMENSION / 2) as u32 {
            for y in 0..(MATRIX_DIMENSION / 2) as u32 {
                for z in 0..(MATRIX_DIMENSION / 2) as u32 {
                    for octant in 0..8 {
                        let start_pos = V3c::<u32>::from(OCTANT_OFFSET_REGION_LUT[octant])
                            * (MATRIX_DIMENSION as u32 / 2);
                        tree.insert(&(start_pos + V3c::new(x, y, z)), &Albedo::from(color_base))
                            .ok()
                            .unwrap();
                    }
                    color_base += 0xAA;
                }
            }
        }

        let item_at_000 = tree.get(&V3c::unit(0));
        assert!(item_at_000 == (&Albedo::from(color_base_original)).into());

        // Separate Uniform leaf by clearing a voxel
        tree.clear(&V3c::unit(0)).ok().unwrap();
        assert!(tree.get(&V3c::unit(0)) == OctreeEntry::Empty);

        // The rest of the voxels should remain intact
        color_base = color_base_original;
        for x in 0..(MATRIX_DIMENSION / 2) as u32 {
            for y in 0..(MATRIX_DIMENSION / 2) as u32 {
                for z in 0..(MATRIX_DIMENSION / 2) as u32 {
                    for octant in 0..8 {
                        if x == 0 && y == 0 && z == 0 && octant == 0 {
                            continue;
                        }
                        let start_pos = V3c::<u32>::from(OCTANT_OFFSET_REGION_LUT[octant])
                            * (MATRIX_DIMENSION as u32 / 2);
                        assert!(
                            tree.get(&(start_pos + V3c::new(x, y, z)))
                                == (&Albedo::from(color_base)).into()
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
        const MATRIX_DIMENSION: u32 = 4;
        let mut tree: Octree = Octree::new(tree_size, MATRIX_DIMENSION).ok().unwrap();

        // Fill each octant with the same data, they should become a solid bricks
        let color_base = 0xFFFF00AA;
        for octant in 0..8 {
            let start_pos =
                V3c::<u32>::from(OCTANT_OFFSET_REGION_LUT[octant]) * (MATRIX_DIMENSION as u32 / 2);
            for x in 0..(MATRIX_DIMENSION / 2) as u32 {
                for y in 0..(MATRIX_DIMENSION / 2) as u32 {
                    for z in 0..(MATRIX_DIMENSION / 2) as u32 {
                        tree.insert(
                            &(start_pos + V3c::new(x, y, z)),
                            &Albedo::from(color_base + octant as u32),
                        )
                        .ok()
                        .unwrap();
                    }
                }
            }
        }

        let item_at_000 = tree.get(&V3c::unit(0));
        assert!(item_at_000 == (&Albedo::from(color_base)).into());

        // Separate Uniform leaf by clearing a voxel
        tree.clear(&V3c::unit(0)).ok().unwrap();
        assert!(tree.get(&V3c::unit(0)) == OctreeEntry::Empty);

        // The rest of the voxels should remain intact
        for octant in 0..8 {
            let start_pos =
                V3c::<u32>::from(OCTANT_OFFSET_REGION_LUT[octant]) * (MATRIX_DIMENSION as u32 / 2);
            for x in 0..(MATRIX_DIMENSION / 2) as u32 {
                for y in 0..(MATRIX_DIMENSION / 2) as u32 {
                    for z in 0..(MATRIX_DIMENSION / 2) as u32 {
                        if x == 0 && y == 0 && z == 0 && octant == 0 {
                            continue;
                        }
                        assert!(
                            tree.get(&(start_pos + V3c::new(x, y, z)))
                                == (&Albedo::from(color_base + octant as u32)).into(),
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_uniform_solid_leaf_separated_by_insert_where_dim_is_4() {
        let tree_size = 8;
        const MATRIX_DIMENSION: u32 = 4;
        let mut tree: Octree = Octree::new(tree_size, MATRIX_DIMENSION).ok().unwrap();

        // Fill each octant with the same data, they should become a solid bricks
        let color_base = 0xFFFF00AA;
        for octant in 0..8 {
            let start_pos =
                V3c::<u32>::from(OCTANT_OFFSET_REGION_LUT[octant]) * (MATRIX_DIMENSION as u32 / 2);
            for x in 0..(MATRIX_DIMENSION / 2) as u32 {
                for y in 0..(MATRIX_DIMENSION / 2) as u32 {
                    for z in 0..(MATRIX_DIMENSION / 2) as u32 {
                        tree.insert(
                            &(start_pos + V3c::new(x, y, z)),
                            &Albedo::from(color_base + octant as u32),
                        )
                        .ok()
                        .unwrap();
                    }
                }
            }
        }

        let item_at_000 = tree.get(&V3c::unit(0));
        assert!(item_at_000 == (&Albedo::from(color_base)).into());

        // Separate Uniform leaf by overwriting a voxel
        let color_base_modified: Albedo = 0x000000FF.into();
        tree.insert(&V3c::unit(0), &color_base_modified)
            .ok()
            .unwrap();
        assert!(tree.get(&V3c::unit(0)) == (&color_base_modified).into());

        // The rest of the voxels should remain intact
        for octant in 0..8 {
            let start_pos =
                V3c::<u32>::from(OCTANT_OFFSET_REGION_LUT[octant]) * (MATRIX_DIMENSION as u32 / 2);
            for x in 0..(MATRIX_DIMENSION / 2) as u32 {
                for y in 0..(MATRIX_DIMENSION / 2) as u32 {
                    for z in 0..(MATRIX_DIMENSION / 2) as u32 {
                        if x == 0 && y == 0 && z == 0 && octant == 0 {
                            continue;
                        }
                        assert!(
                            tree.get(&(start_pos + V3c::new(x, y, z)))
                                == (&Albedo::from(color_base + octant as u32)).into(),
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_uniform_parted_brick_leaf_separated_by_insert() {
        std::env::set_var("RUST_BACKTRACE", "1");
        let tree_size = 4;
        const MATRIX_DIMENSION: u32 = 2;
        let mut tree: Octree = Octree::new(tree_size, MATRIX_DIMENSION).ok().unwrap();

        // Fill each octant of each brick with the same data, they should become a uniform leaf
        let color_base_original = 0xFFFF00FF;
        let mut color_base = color_base_original;
        for x in 0..(MATRIX_DIMENSION / 2) as u32 {
            for y in 0..(MATRIX_DIMENSION / 2) as u32 {
                for z in 0..(MATRIX_DIMENSION / 2) as u32 {
                    for octant in 0..8 {
                        let start_pos = V3c::<u32>::from(OCTANT_OFFSET_REGION_LUT[octant])
                            * (MATRIX_DIMENSION as u32 / 2);
                        tree.insert(&(start_pos + V3c::new(x, y, z)), &Albedo::from(color_base))
                            .ok()
                            .unwrap();
                    }
                    color_base += 0xAA;
                }
            }
        }

        let item_at_000 = tree.get(&V3c::unit(0));
        assert!(item_at_000 == (&Albedo::from(color_base_original)).into());

        // Separate Uniform leaf by setting a voxel
        let color_base_modified: Albedo = 0x000000FF.into();
        tree.insert(&V3c::unit(0), &color_base_modified)
            .ok()
            .unwrap();
        assert!(tree.get(&V3c::unit(0)) == (&color_base_modified).into());

        // The rest of the voxels should remain intact
        color_base = color_base_original;
        for x in 0..(MATRIX_DIMENSION / 2) as u32 {
            for y in 0..(MATRIX_DIMENSION / 2) as u32 {
                for z in 0..(MATRIX_DIMENSION / 2) as u32 {
                    for octant in 0..8 {
                        if x == 0 && y == 0 && z == 0 && octant == 0 {
                            continue;
                        }
                        let start_pos = V3c::<u32>::from(OCTANT_OFFSET_REGION_LUT[octant])
                            * (MATRIX_DIMENSION as u32 / 2);
                        assert!(
                            tree.get(&(start_pos + V3c::new(x, y, z)))
                                == (&Albedo::from(color_base)).into()
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

        let mut tree: Octree = Octree::new(8, 4).ok().unwrap();
        tree.auto_simplify = false;

        tree.insert_at_lod(&V3c::new(1, 1, 1), 4, &red)
            .ok()
            .unwrap();

        // This will set the area equal to 64 1-sized nodes:
        // a size-4 node includes 2 levels,
        // 1-sized nodes at the bottom level doesn't have children,
        // 2-sized nodes above have 8 children each
        // so one 4-sized node has 8*8 = 64 children
        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    let hit = tree.get(&V3c::new(x, y, z));
                    if hit != OctreeEntry::Empty {
                        assert!(
                            hit == (&red).into(),
                            "Hit mismatch at {:?}: {:?} <> {:?}",
                            (x, y, z),
                            hit,
                            red,
                        );
                        hits += 1;
                    }
                }
            }
        }

        // At most one brick can be updated; Starting from 1,1,1 the updated area spans 3x3x3,
        // thus the number of voxels updated are 27
        assert!(hits == 27, "Expected 27 hits instead of {hits}");
    }

    #[test]
    fn test_insert_at_lod_with_unaligned_size_where_dim_is_1() {
        let red: Albedo = 0xFF0000FF.into();

        let mut tree: Octree = Octree::new(8, 1).ok().unwrap();
        tree.auto_simplify = false;

        tree.insert_at_lod(&V3c::new(3, 3, 3), 3, &red)
            .ok()
            .unwrap();
        let mut hits = 0;
        for x in 0..8 {
            for y in 0..8 {
                for z in 0..8 {
                    let hit = tree.get(&V3c::new(x, y, z));
                    if hit != OctreeEntry::Empty {
                        assert!(
                            hit == (&red).into(),
                            "Hit mismatch at {:?}: {:?} <> {:?}",
                            (x, y, z),
                            hit,
                            red,
                        );
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

        let mut tree: Octree = Octree::new(8, 4).ok().unwrap();
        tree.auto_simplify = false;

        tree.insert_at_lod(&V3c::new(1, 1, 1), 3, &red)
            .ok()
            .unwrap();

        assert!(tree.get(&V3c::new(1, 1, 1)) == (&red).into());
        let mut hits = 0;
        for x in 0..8 {
            for y in 0..8 {
                for z in 0..8 {
                    let hit = tree.get(&V3c::new(x, y, z));
                    if hit != OctreeEntry::Empty {
                        assert!(
                            hit == (&red).into(),
                            "Hit mismatch at {:?}: {:?} <> {:?}",
                            (x, y, z),
                            hit,
                            red,
                        );
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

        let mut tree: Octree = Octree::new(8, 1).ok().unwrap();

        // This will set the area equal to 8 1-sized nodes
        tree.insert_at_lod(&V3c::new(5, 0, 0), 2, &red)
            .ok()
            .unwrap();

        assert!(tree.get(&V3c::new(4, 0, 0)) == (&red).into());
        assert!(tree.get(&V3c::new(4, 0, 1)) == (&red).into());
        assert!(tree.get(&V3c::new(4, 1, 0)) == (&red).into());
        assert!(tree.get(&V3c::new(4, 1, 1)) == (&red).into());
        assert!(tree.get(&V3c::new(5, 0, 0)) == (&red).into());
        assert!(tree.get(&V3c::new(5, 0, 1)) == (&red).into());
        assert!(tree.get(&V3c::new(5, 1, 0)) == (&red).into());
        assert!(tree.get(&V3c::new(5, 1, 1)) == (&red).into());

        // This will set the area equal to 64 1-sized nodes:
        // a size-4 node includes 2 levels,
        // 1-sized nodes at the bottom level doesn't have children,
        // 2-sized nodes above have 8 children each
        // so one 4-sized node has 8*8 = 64 children
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, &green)
            .ok()
            .unwrap();
        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    let hit = tree.get(&V3c::new(x, y, z));
                    if hit != OctreeEntry::Empty {
                        assert!(
                            hit == (&green).into(),
                            "Hit mismatch at {:?}: {:?} <> {:?}",
                            (x, y, z),
                            hit,
                            green,
                        );
                        hits += 1;
                    }
                }
            }
        }

        for x in 4..6 {
            for y in 0..2 {
                for z in 0..2 {
                    let hit = tree.get(&V3c::new(x, y, z));
                    if hit != OctreeEntry::Empty {
                        assert!(
                            hit == (&red).into(),
                            "Hit mismatch at {:?}: {:?} <> {:?}",
                            (x, y, z),
                            hit,
                            red,
                        );
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
        let mut tree: Octree = Octree::new(SIZE, 1).ok().unwrap();

        // The below set of values should be simplified to a single node
        for x in 0..SIZE {
            for y in 0..SIZE {
                for z in 0..SIZE {
                    tree.insert(&V3c::new(x, y, z), &red).ok().unwrap();
                }
            }
        }

        // The below should brake the simplified node back to its parts
        tree.insert(&V3c::new(0, 0, 0), &green).ok().unwrap();

        // Integrity should be kept
        assert!(tree.get(&V3c::new(0, 0, 0)) == (&green).into());
        for x in 1..SIZE {
            for y in 1..SIZE {
                for z in 1..SIZE {
                    assert!(tree.get(&V3c::new(x, y, z)) == (&red).into());
                }
            }
        }
    }

    #[test]
    fn test_simplifyable_insert_and_get_where_dim_is_2() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();

        const SIZE: u32 = 4;
        let mut tree: Octree = Octree::new(SIZE, 2).ok().unwrap();

        // The below set of values should be simplified to a single node
        for x in 0..SIZE {
            for y in 0..SIZE {
                for z in 0..SIZE {
                    tree.insert(&V3c::new(x, y, z), &red).ok().unwrap();
                }
            }
        }

        // The below should brake the simplified node back to its parts
        tree.insert(&V3c::new(0, 0, 0), &green).ok().unwrap();

        // Integrity should be kept
        assert!(tree.get(&V3c::new(0, 0, 0)) == (&green).into());
        for x in 1..SIZE {
            for y in 1..SIZE {
                for z in 1..SIZE {
                    assert!(tree.get(&V3c::new(x, y, z)) == (&red).into());
                }
            }
        }
    }

    #[test]
    fn test_simple_clear_with_aligned_dim() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();
        let blue: Albedo = 0x0000FFFF.into();

        let mut tree: Octree = Octree::new(2, 1).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), &red).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 0), &green).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 1), &blue).ok().unwrap();
        tree.clear(&V3c::new(0, 0, 1)).ok().unwrap();

        assert!(tree.get(&V3c::new(1, 0, 0)) == (&red).into());
        assert!(tree.get(&V3c::new(0, 1, 0)) == (&green).into());
        let item_at_001 = tree.get(&V3c::new(0, 0, 1));
        assert!(item_at_001 == OctreeEntry::Empty);
        let item_at_111 = tree.get(&V3c::new(1, 1, 1));
        assert!(item_at_111 == OctreeEntry::Empty);
    }

    #[test]
    fn test_simple_clear_where_dim_is_2() {
        let red: Albedo = 0xFF0000FF.into();
        let green: Albedo = 0x00FF00FF.into();
        let blue: Albedo = 0x0000FFFF.into();

        let mut tree: Octree = Octree::new(4, 2).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), &red).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 0), &green).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 1), &blue).ok().unwrap();
        tree.clear(&V3c::new(0, 0, 1)).ok().unwrap();

        assert!(tree.get(&V3c::new(1, 0, 0)) == (&red).into());
        assert!(tree.get(&V3c::new(0, 1, 0)) == (&green).into());
        let item_at_001 = tree.get(&V3c::new(0, 0, 1));
        assert!(item_at_001 == OctreeEntry::Empty);
        let item_at_111 = tree.get(&V3c::new(1, 1, 1));
        assert!(item_at_111 == OctreeEntry::Empty);
    }

    #[test]
    fn test_double_clear() {
        let albedo_black: Albedo = 0x000000FF.into();
        let albedo_white: Albedo = 0xFFFFFFFF.into();
        let mut tree: Octree = Octree::new(2, 1).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), &albedo_black).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 0), &albedo_white).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 1), &albedo_white).ok().unwrap();
        tree.clear(&V3c::new(0, 0, 1)).ok().unwrap();
        tree.clear(&V3c::new(0, 0, 1)).ok().unwrap();

        assert!(tree.get(&V3c::new(1, 0, 0)) == (&albedo_black).into());
        assert!(tree.get(&V3c::new(0, 1, 0)) == (&albedo_white).into());
        let item_at_001 = tree.get(&V3c::new(0, 0, 1));
        assert!(item_at_001 == OctreeEntry::Empty);
    }

    #[test]
    fn test_simplifyable_clear() {
        let albedo: Albedo = 0xFFAAEEFF.into();
        const SIZE: u32 = 2;
        let mut tree: Octree = Octree::new(SIZE, 1).ok().unwrap();

        // The below set of values should be simplified to a single node
        for x in 0..SIZE {
            for y in 0..SIZE {
                for z in 0..SIZE {
                    tree.insert(&V3c::new(x, y, z), &albedo).ok().unwrap();
                }
            }
        }

        // The below should brake the simplified node back to its party
        tree.clear(&V3c::new(0, 0, 0)).ok().unwrap();

        // Integrity should be kept
        let item_at_000 = tree.get(&V3c::new(0, 0, 0));
        assert!(item_at_000 == OctreeEntry::Empty);
        for x in 1..SIZE {
            for y in 1..SIZE {
                for z in 1..SIZE {
                    assert!(tree.get(&V3c::new(x, y, z)) == (&albedo).into());
                }
            }
        }
    }

    #[test]
    fn test_simplifyable_clear_where_dim_is_2() {
        let albedo: Albedo = 0xFFAAEEFF.into();
        const SIZE: u32 = 4;
        let mut tree: Octree = Octree::new(SIZE, 2).ok().unwrap();

        // The below set of values should be simplified to a single node
        for x in 0..SIZE {
            for y in 0..SIZE {
                for z in 0..SIZE {
                    tree.insert(&V3c::new(x, y, z), &albedo).ok().unwrap();
                }
            }
        }

        // The below should brake the simplified node back to its party
        tree.clear(&V3c::new(0, 0, 0)).ok().unwrap();

        // Integrity should be kept
        let item_at_000 = tree.get(&V3c::new(0, 0, 0));
        assert!(item_at_000 == OctreeEntry::Empty);
        for x in 1..SIZE {
            for y in 1..SIZE {
                for z in 1..SIZE {
                    assert!(tree.get(&V3c::new(x, y, z)) == (&albedo).into());
                }
            }
        }
    }

    #[test]
    fn test_clear_to_nothing() {
        let albedo: Albedo = 0xFFAAEEFF.into();
        let entry = OctreeEntry::Visual(&albedo);
        let mut tree: Octree = Octree::new(2, 1).ok().unwrap();

        // The below set of values should be simplified to a single node
        tree.insert(&V3c::new(0, 0, 0), entry).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 1), entry).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 0), entry).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 1), entry).ok().unwrap();
        tree.insert(&V3c::new(1, 0, 0), entry).ok().unwrap();
        tree.insert(&V3c::new(1, 0, 1), entry).ok().unwrap();
        tree.insert(&V3c::new(1, 1, 0), entry).ok().unwrap();
        tree.insert(&V3c::new(1, 1, 1), entry).ok().unwrap();

        // The below should brake the simplified node back to its party
        tree.clear_at_lod(&V3c::new(0, 0, 0), 2).ok().unwrap();

        // Nothing should remain in the tree
        assert!(tree.get(&V3c::new(0, 0, 0)) == OctreeEntry::Empty);
        assert!(tree.get(&V3c::new(0, 0, 1)) == OctreeEntry::Empty);
        assert!(tree.get(&V3c::new(0, 1, 0)) == OctreeEntry::Empty);
        assert!(tree.get(&V3c::new(0, 1, 1)) == OctreeEntry::Empty);
        assert!(tree.get(&V3c::new(1, 0, 0)) == OctreeEntry::Empty);
        assert!(tree.get(&V3c::new(1, 0, 1)) == OctreeEntry::Empty);
        assert!(tree.get(&V3c::new(1, 1, 0)) == OctreeEntry::Empty);
        assert!(tree.get(&V3c::new(1, 1, 1)) == OctreeEntry::Empty);
    }

    #[test]
    fn test_clear_at_lod_with_aligned_dim() {
        let albedo: Albedo = 0xFFAAEEFF.into();
        let mut tree: Octree = Octree::new(4, 1).ok().unwrap();

        // This will set the area equal to 64 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, &albedo)
            .ok()
            .unwrap();

        // This will clear an area equal to 8 1-sized nodes
        tree.clear_at_lod(&V3c::new(0, 0, 0), 2).ok().unwrap();

        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    let hit = tree.get(&V3c::new(x, y, z));
                    if hit != OctreeEntry::Empty {
                        assert!(
                            hit == (&albedo).into(),
                            "Hit mismatch at {:?}: {:?} <> {:?}",
                            (x, y, z),
                            hit,
                            albedo
                        );
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
        let mut tree: Octree = Octree::new(4, 2).ok().unwrap();

        // This will set the area equal to 64 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, &albedo)
            .ok()
            .unwrap();

        // This will clear an area equal to 8 1-sized nodes
        tree.clear_at_lod(&V3c::new(0, 0, 0), 2).ok().unwrap();

        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    let hit = tree.get(&V3c::new(x, y, z));
                    if hit != OctreeEntry::Empty {
                        assert!(
                            hit == (&albedo).into(),
                            "Hit mismatch at {:?}: {:?} <> {:?}",
                            (x, y, z),
                            hit,
                            albedo,
                        );
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
        let mut tree: Octree = Octree::new(4, 1).ok().unwrap();

        // This will set the area equal to 64 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, &albedo)
            .ok()
            .unwrap();

        // This will clear an area equal to 8 1-sized nodes
        tree.clear_at_lod(&V3c::new(1, 1, 1), 2).ok().unwrap();

        // unset voxels should not be present
        assert!(tree.get(&V3c::new(0, 0, 0)) == OctreeEntry::Empty);
        assert!(tree.get(&V3c::new(0, 0, 1)) == OctreeEntry::Empty);
        assert!(tree.get(&V3c::new(0, 1, 0)) == OctreeEntry::Empty);
        assert!(tree.get(&V3c::new(0, 1, 1)) == OctreeEntry::Empty);
        assert!(tree.get(&V3c::new(1, 0, 0)) == OctreeEntry::Empty);
        assert!(tree.get(&V3c::new(1, 0, 1)) == OctreeEntry::Empty);
        assert!(tree.get(&V3c::new(1, 1, 0)) == OctreeEntry::Empty);
        assert!(tree.get(&V3c::new(1, 1, 1)) == OctreeEntry::Empty);

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
                    let hit = tree.get(&V3c::new(x, y, z));
                    if hit != OctreeEntry::Empty {
                        assert!(
                            hit == (&albedo).into(),
                            "Hit mismatch at {:?}: {:?} <> {:?}",
                            (x, y, z),
                            hit,
                            albedo,
                        );
                        hits += 1;
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
        let mut tree: Octree = Octree::new(8, 4).ok().unwrap();

        tree.insert_at_lod(&V3c::new(0, 0, 0), 8, &albedo)
            .ok()
            .unwrap();

        assert!(tree.get(&V3c::unit(0)).is_some());

        let mut hits = 0;
        for x in 0..8 {
            for y in 0..8 {
                for z in 0..8 {
                    let hit = tree.get(&V3c::new(x, y, z));
                    assert!(hit == (&albedo).into());
                    if hit != OctreeEntry::Empty {
                        hits += 1;
                    }
                }
            }
        }

        // number of hits should be the number of nodes set
        assert!(hits == 512, "Expected 512 hits instead of {hits}",);

        tree.clear_at_lod(&V3c::new(1, 1, 1), 4).ok().unwrap();
        let mut hits = 0;
        for x in 0..8 {
            for y in 0..8 {
                for z in 0..8 {
                    let hit = tree.get(&V3c::new(x, y, z));
                    if hit != OctreeEntry::Empty {
                        assert!(
                            hit == (&albedo).into(),
                            "Hit mismatch at {:?}: {:?} <> {:?}",
                            (x, y, z),
                            hit,
                            albedo,
                        );
                        hits += 1;
                    }
                }
            }
        }

        // number of hits should be the number of nodes set minus the number of nodes cleared
        // Note: Only at most one brick is updated with each update call
        // --> In this case the relevant brick is updated from 1,1,1 ---> 3,3,3 ( inclusive )
        // So 3^3 voxels are cleared == 27
        assert!(
            hits == (512 - 27),
            "Expected {} hits instead of {hits}",
            512 - 27
        );
    }

    #[test]
    fn test_clear_at_lod_with_unaligned_size_where_dim_is_1() {
        let albedo: Albedo = 0xFFAAEEFF.into();
        let mut tree: Octree = Octree::new(4, 1).ok().unwrap();
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, &albedo)
            .ok()
            .unwrap();
        tree.clear_at_lod(&V3c::new(0, 0, 0), 3).ok().unwrap();

        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    let hit = tree.get(&V3c::new(x, y, z));
                    if hit != OctreeEntry::Empty {
                        assert!(
                            hit == (&albedo).into(),
                            "Hit mismatch at {:?}: {:?} <> {:?}",
                            (x, y, z),
                            hit,
                            albedo,
                        );
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
        let mut tree: Octree = Octree::new(8, 4).ok().unwrap();
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, &albedo)
            .ok()
            .unwrap();
        tree.clear_at_lod(&V3c::new(0, 0, 0), 3).ok().unwrap();

        let mut hits = 0;
        for x in 0..8 {
            for y in 0..8 {
                for z in 0..8 {
                    let hit = tree.get(&V3c::new(x, y, z));
                    if hit != OctreeEntry::Empty {
                        assert!(
                            hit == (&albedo).into(),
                            "Hit mismatch at {:?}: {:?} <> {:?}",
                            (x, y, z),
                            hit,
                            albedo,
                        );
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
        let mut tree: Octree = Octree::new(TREE_SIZE, 1).ok().unwrap();
        for x in FILL_RANGE_START..TREE_SIZE {
            for y in FILL_RANGE_START..TREE_SIZE {
                for z in FILL_RANGE_START..TREE_SIZE {
                    let pos = V3c::new(x, y, z);
                    tree.insert(&pos, &Albedo::from(x + y + z)).ok().unwrap();
                    assert!(tree.get(&pos) == (&Albedo::from(x + y + z)).into());
                }
            }
        }
    }

    #[test]
    fn test_case_inserting_empty() {
        let mut tree: Octree = Octree::new(4, 1).ok().unwrap();
        tree.insert(&V3c::new(3, 0, 0), &Albedo::zero())
            .ok()
            .unwrap();
        let item = tree.get(&V3c::new(3, 0, 0));
        assert!(
            item == OctreeEntry::Empty,
            "Item shouldn't exist: {:?}",
            item
        );
    }
}
