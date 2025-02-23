use crate::{
    octree::{Albedo, Octree, OctreeEntry},
    spatial::{lut::OCTANT_OFFSET_REGION_LUT, math::vector::V3c},
    voxel_data,
};
use num_traits::Zero;

#[test]
fn test_simplest_insert_and_get() {
    let red: Albedo = 0xFF000001.into();
    let mut tree: Octree = Octree::new(2, 1).ok().unwrap();
    tree.auto_simplify = false;
    tree.insert(&V3c::new(0, 0, 0), &red)
        .expect("octree insert");

    assert!(tree.get(&V3c::new(0, 0, 0)) == (&red).into());
}

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
fn test_insert_empty() {
    let mut tree: Octree = Octree::new(2, 1).ok().unwrap();
    tree.auto_simplify = false;
    tree.insert(&V3c::new(0, 0, 0), OctreeEntry::Empty).ok();
    assert!(tree.get(&V3c::new(0, 0, 0)) == OctreeEntry::Empty);
}

#[test]
fn test_complex_insert_and_get() {
    let red: Albedo = 0xFF0000FF.into();
    let green: Albedo = 0x00FF00FF.into();

    let mut tree: Octree = Octree::new(2, 1).ok().unwrap();
    tree.auto_simplify = false;
    tree.insert(&V3c::new(1, 0, 0), (&red, &3))
        .expect("insert to work");
    tree.insert(&V3c::new(0, 1, 0), (&green, &1))
        .expect("insert to work");
    tree.insert(&V3c::new(0, 0, 1), voxel_data!(&2))
        .expect("insert to work");

    let hit = tree.get(&V3c::new(1, 0, 0));
    assert!(
        hit == (&red, &3).into(),
        "Hit mismatch at {:?}: {:?} <> {:?}",
        (0, 0, 0),
        hit,
        (&red, &3)
    );
    let hit = tree.get(&V3c::new(0, 1, 0));
    assert!(
        hit == (&green, &1).into(),
        "Hit mismatch at {:?}: {:?} <> {:?}",
        (0, 0, 0),
        hit,
        (&green, &1)
    );
    let hit = tree.get(&V3c::new(0, 0, 1));
    assert!(
        hit == voxel_data!(&2),
        "Hit mismatch at {:?}: {:?} <> {:?}",
        (0, 0, 0),
        hit,
        &2
    );
    let hit = tree.get(&V3c::new(1, 1, 1));
    assert!(
        hit == OctreeEntry::Empty,
        "Hit mismatch at {:?}: {:?} <> {:?}",
        (0, 0, 0),
        hit,
        OctreeEntry::<u32>::Empty
    );

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
fn test_insert_at_lod_where_dim_is_1() {
    let red: Albedo = 0xFF0000FF.into();
    let green: Albedo = 0x00FF00FF.into();

    let mut tree: Octree = Octree::new(8, 1).ok().unwrap();
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
    assert_eq!(hits, 64);
}

#[test]
fn test_insert_at_lod_where_dim_is_2() {
    let red: Albedo = 0xFF0000FF.into();
    let green: Albedo = 0x00FF00FF.into();

    let mut tree: Octree = Octree::new(8, 2).ok().unwrap();
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
    assert_eq!(hits, 64);
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
fn test_update_color() {
    let red: Albedo = 0xFF0000FF.into();
    let green: Albedo = 0x00FF00FF.into();
    let mut tree: Octree = Octree::new(2, 1).ok().unwrap();
    tree.auto_simplify = false;

    tree.insert(&V3c::new(0, 0, 0), (&red, &3)).ok();
    assert!(tree.get(&V3c::new(0, 0, 0)) == (&red, &3).into());

    tree.update(&V3c::new(0, 0, 0), &green).ok();
    let hit = tree.get(&V3c::new(0, 0, 0));
    assert!(
        hit == (&green, &3).into(),
        "Hit mismatch at {:?}: {:?} <> {:?}",
        (0, 0, 0),
        hit,
        (&green, &3)
    );
}

#[test]
fn test_update_data() {
    let red: Albedo = 0xFF0000FF.into();
    let mut tree: Octree = Octree::new(2, 1).ok().unwrap();
    tree.auto_simplify = false;

    tree.insert(&V3c::new(0, 0, 0), (&red, &3)).ok();
    assert!(tree.get(&V3c::new(0, 0, 0)) == (&red, &3).into());

    tree.update(&V3c::new(0, 0, 0), OctreeEntry::Informative(&4))
        .ok();
    let hit = tree.get(&V3c::new(0, 0, 0));
    assert!(
        hit == (&red, &4).into(),
        "Hit mismatch at {:?}: {:?} <> {:?}",
        (0, 0, 0),
        hit,
        (&red, &4)
    );
}

#[test]
fn test_update_empty() {
    let red: Albedo = 0xFF0000FF.into();
    let mut tree: Octree = Octree::new(2, 1).ok().unwrap();
    tree.auto_simplify = false;

    tree.insert(&V3c::new(0, 0, 0), (&red, &3)).ok();
    assert!(tree.get(&V3c::new(0, 0, 0)) == (&red, &3).into());

    tree.update(&V3c::new(0, 0, 0), OctreeEntry::Empty).ok();
    assert!(tree.get(&V3c::new(0, 0, 0)) == (&red, &3).into());
}

#[test]
fn test_uniform_solid_leaf_separated_by_clear_where_dim_is_1() {
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
fn test_uniform_solid_leaf_separated_by_insert_where_dim_is_1() {
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
    tree.insert_at_lod(&V3c::new(4, 0, 0), 2, &red)
        .ok()
        .unwrap();

    assert_eq!(tree.get(&V3c::new(4, 0, 0)), (&red).into());
    assert_eq!(tree.get(&V3c::new(4, 0, 1)), (&red).into());
    assert_eq!(tree.get(&V3c::new(4, 1, 0)), (&red).into());
    assert_eq!(tree.get(&V3c::new(4, 1, 1)), (&red).into());
    assert_eq!(tree.get(&V3c::new(5, 0, 0)), (&red).into());
    assert_eq!(tree.get(&V3c::new(5, 0, 1)), (&red).into());
    assert_eq!(tree.get(&V3c::new(5, 1, 0)), (&red).into());
    assert_eq!(tree.get(&V3c::new(5, 1, 1)), (&red).into());

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
                    assert_eq!(hit, (&green).into(), "Hit mismatch at {:?}", (x, y, z));
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
                    assert_eq!(hit, (&red).into(), "Hit mismatch at {:?}", (x, y, z));
                    hits += 1;
                }
            }
        }
    }
    assert_eq!(hits, (64 + 8));
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
fn test_simple_clear_where_dim_is_1() {
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
fn test_clear_small_part_of_large_node_ocbits_resolution_test() {
    const TREE_SIZE: u32 = 64;
    const BRICK_DIMENSION: u32 = 8;
    let red: Albedo = 0xFF0000FF.into();
    let mut tree: Octree = Octree::new(TREE_SIZE, BRICK_DIMENSION).ok().unwrap();

    tree.insert(&V3c::new(0, 1, 1), &red).ok().unwrap();
    tree.insert(&V3c::new(1, 0, 0), &red).ok().unwrap();

    assert_eq!(tree.get(&V3c::new(0, 1, 1)), (&red).into());
    assert_eq!(tree.get(&V3c::new(1, 0, 0)), (&red).into());

    tree.clear(&V3c::new(1, 0, 0)).ok().unwrap();
    assert_eq!(tree.get(&V3c::new(1, 0, 0)), voxel_data!());
}

#[test]
fn test_set_small_part_of_large_node_ocbits_resolution_test_underflow() {
    const TREE_SIZE: u32 = 64;
    const BRICK_DIMENSION: u32 = 8;
    let red: Albedo = 0xFF0000FF.into();
    let mut tree: Octree = Octree::new(TREE_SIZE, BRICK_DIMENSION).ok().unwrap();

    tree.insert_at_lod(&V3c::new(33, 33, 33), 2, &red)
        .ok()
        .unwrap();

    assert_eq!(tree.get(&V3c::new(33, 33, 33)), (&red).into());

    tree.clear(&V3c::new(33, 33, 33)).ok().unwrap();
    assert_eq!(tree.get(&V3c::new(33, 33, 33)), voxel_data!());
}

#[test]
fn test_set_small_part_of_large_node_ocbits_resolution_test_overflow() {
    const TREE_SIZE: u32 = 64;
    const BRICK_DIMENSION: u32 = 8;
    let red: Albedo = 0xFF0000FF.into();
    let mut tree: Octree = Octree::new(TREE_SIZE, BRICK_DIMENSION).ok().unwrap();

    tree.insert(&V3c::new(31, 31, 31), &red).ok().unwrap();

    assert_eq!(tree.get(&V3c::new(31, 31, 31)), (&red).into());

    tree.clear(&V3c::new(31, 31, 31)).ok().unwrap();
    assert_eq!(tree.get(&V3c::new(31, 31, 31)), voxel_data!());
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
    let mut tree: Octree = Octree::new(4, 1).ok().unwrap();

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
fn test_clear_edge_case() {
    const TREE_SIZE: u32 = 64;
    const BRICK_DIMENSION: u32 = 16;
    let red: Albedo = 0xFF0000FF.into();
    let mut tree: Octree = Octree::new(TREE_SIZE, BRICK_DIMENSION).ok().unwrap();

    tree.update(&V3c::new(1, 0, 0), voxel_data!(&0xFACEFEED))
        .ok()
        .unwrap();

    tree.insert_at_lod(&V3c::new(0, 0, 0), 32, &red)
        .ok()
        .unwrap();

    tree.clear_at_lod(&V3c::new(5, 5, 5), 8).ok().unwrap();
    for x in 5..8 {
        for y in 5..8 {
            for z in 5..8 {
                assert_eq!(tree.get(&V3c::new(x, y, z)), voxel_data!());
            }
        }
    }

    for x in 0..5 {
        for y in 0..5 {
            for z in 0..5 {
                assert_eq!(
                    tree.get(&V3c::new(x, y, z)),
                    (&red).into(),
                    "Hit mismatch at {:?}",
                    (x, y, z)
                );
            }
        }
    }

    tree.clear_at_lod(&V3c::new(0, 0, 0), 32).ok().unwrap();
    for x in 0..32 {
        for y in 0..32 {
            for z in 0..32 {
                assert_eq!(
                    tree.get(&V3c::new(x, y, z)),
                    voxel_data!(),
                    "Hit mismatch at {:?}",
                    (x, y, z)
                );
            }
        }
    }
}

#[test]
fn test_clear_at_lod_where_dim_is_1() {
    let albedo: Albedo = 0xFFAAEEFF.into();
    let mut tree: Octree = Octree::new(8, 1).ok().unwrap();

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
    assert_eq!(hits, (64 - 8));
}

#[test]
fn test_clear_at_lod_where_dim_is_2() {
    let albedo: Albedo = 0xFFAAEEFF.into();
    let mut tree: Octree = Octree::new(8, 2).ok().unwrap();

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
    assert_eq!(hits, (64 - 8));
}

#[test]
fn test_clear_at_lod_with_unaligned_position() {
    let albedo: Albedo = 0xFFAAEEFF.into();
    let mut tree: Octree = Octree::new(8, 1).ok().unwrap();

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
    let mut tree: Octree = Octree::new(16, 4).ok().unwrap();

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
    let mut tree: Octree = Octree::new(8, 1).ok().unwrap();
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
    assert_eq!(hits, (64 - 8));
}

#[test]
fn test_clear_at_lod_with_unaligned_size_where_dim_is_4() {
    let albedo: Albedo = 0xFFAAEEFF.into();
    let mut tree: Octree = Octree::new(8, 4).ok().unwrap();
    tree.insert_at_lod(&V3c::new(0, 0, 0), 4, &albedo)
        .ok()
        .unwrap();
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
    assert_eq!(hits, 64);

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
    assert_eq!(hits, (64 - 27));
}

#[test]
fn test_clear_whole_nodes_where_dim_is_4() {
    let albedo: Albedo = 0xFFAAEEFF.into();
    let mut tree: Octree = Octree::new(16, 4).ok().unwrap();
    tree.insert_at_lod(&V3c::new(0, 0, 0), 8, &albedo)
        .ok()
        .unwrap();
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
    assert_eq!(hits, 512);

    tree.clear_at_lod(&V3c::new(0, 0, 0), 5).ok().unwrap();
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
    assert_eq!(hits, (512 - 64));
}

#[test]
fn test_overwrite_whole_nodes_where_dim_is_4() {
    let red: Albedo = 0xFF0000FF.into();
    let blue: Albedo = 0x0000FFFF.into();
    let mut tree: Octree = Octree::new(16, 4).ok().unwrap();
    tree.insert_at_lod(&V3c::new(0, 0, 0), 8, &red)
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
    assert_eq!(hits, 512);

    tree.insert_at_lod(&V3c::new(0, 0, 0), 5, &blue)
        .ok()
        .unwrap();
    let mut hits_red = 0;
    let mut hits_blue = 0;
    for x in 0..8 {
        for y in 0..8 {
            for z in 0..8 {
                let hit = tree.get(&V3c::new(x, y, z));
                assert_ne!(hit, OctreeEntry::Empty);
                match hit {
                    OctreeEntry::Visual(hit) => {
                        if *hit == red {
                            hits_red += 1
                        }
                        if *hit == blue {
                            hits_blue += 1
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    assert_eq!(hits_red, (512 - 64));
    assert_eq!(hits_blue, (64));
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
