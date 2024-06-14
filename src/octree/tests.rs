#[cfg(test)]
mod octree_serialization_tests {
    use crate::octree::types::Albedo;
    use bendy::decoding::FromBencode;

    use crate::object_pool::empty_marker;
    use crate::octree::types::NodeChildren;
    use crate::octree::Octree;
    use crate::octree::V3c;

    #[test]
    fn test_node_children_serialization() {
        use bendy::encoding::ToBencode;

        let node_children_empty = NodeChildren::new(empty_marker());
        let node_children_filled = NodeChildren::from(empty_marker(), [1, 2, 3, 4, 5, 6, 7, 8]);
        let node_children_bitmap = NodeChildren::bitmasked(empty_marker(), 666);

        let serialized_node_children_empty = node_children_empty.to_bencode();
        let serialized_node_children_filled = node_children_filled.to_bencode();
        let serialized_node_children_bitmap = node_children_bitmap.to_bencode();

        let deserialized_node_children_empty =
            NodeChildren::from_bencode(&serialized_node_children_empty.ok().unwrap())
                .ok()
                .unwrap();
        let deserialized_node_children_filled =
            NodeChildren::from_bencode(&serialized_node_children_filled.ok().unwrap())
                .ok()
                .unwrap();
        let deserialized_node_children_bitmap =
            NodeChildren::from_bencode(&serialized_node_children_bitmap.ok().unwrap())
                .ok()
                .unwrap();

        assert!(deserialized_node_children_empty == node_children_empty);
        assert!(deserialized_node_children_filled == node_children_filled);
        assert!(deserialized_node_children_bitmap == node_children_bitmap);
    }

    #[test]
    fn test_octree_file_io() {
        let red: Albedo = 0xFF0000FF.into();

        let mut tree = Octree::<Albedo>::new(4).ok().unwrap();

        // This will set the area equal to 64 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, red).ok().unwrap();

        // This will clear an area equal to 8 1-sized nodes
        tree.clear_at_lod(&V3c::new(0, 0, 0), 2).ok().unwrap();

        // save andd load into a new tree
        tree.save("test_junk_octree").ok().unwrap();
        let tree_copy = Octree::<Albedo>::load("test_junk_octree").ok().unwrap();

        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    assert!(tree.get(&V3c::new(x, y, z)) == tree_copy.get(&V3c::new(x, y, z)));
                    if let Some(hit) = tree_copy.get(&V3c::new(x, y, z)) {
                        assert_eq!(*hit, red);
                        hits += 1;
                    }
                }
            }
        }

        // number of hits should be the number of nodes set minus the number of nodes cleared
        assert!(hits == (64 - 8));
    }

    #[test]
    fn test_big_octree_serialize() {
        let red: Albedo = 0xFF0000FF.into();

        let mut tree = Octree::<Albedo>::new(512).ok().unwrap();
        for x in 256..300 {
            for y in 256..300 {
                for z in 256..300 {
                    let pos = V3c::new(x, y, z);
                    tree.insert(&pos, red).ok().unwrap();
                }
            }
        }

        let serialized = tree.to_bytes();
        let deserialized = Octree::<Albedo>::from_bytes(serialized);

        for x in 256..300 {
            for y in 256..300 {
                for z in 256..300 {
                    let pos = V3c::new(x, y, z);
                    assert!(deserialized.get(&pos).is_some_and(|v| *v == (red)));
                }
            }
        }
    }
}

#[cfg(test)]
mod octree_tests {
    use crate::octree::types::{Albedo, Octree, VoxelData};
    use crate::spatial::math::vector::V3c;

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
    fn test_insert_at_lod() {
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
    fn test_case_simplified_insert_separated_by_clear() {
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
    fn test_insert_at_lod_with_unaligned_size__() {
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
        assert!(hits == 64);
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
    fn test_simple_clear() {
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

        let mut tree = Octree::<Albedo, 2>::new(2).ok().unwrap();
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
    fn test_clear_at_lod() {
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
    fn test_clear_at_lod_with_unaligned_size() {
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
        // in this case, clear size is taken as 2 as it is the largest smaller number where n == 2^x
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
}
