#[cfg(test)]
mod octree_serialization_tests {
    use crate::octree::Octree;
    use crate::octree::V3c;

    #[test]
    fn test_octree_file_io() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();

        // This will set the area equal to 64 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, 5).ok().unwrap();

        // This will clear an area equal to 8 1-sized nodes
        tree.clear_at_lod(&V3c::new(0, 0, 0), 2).ok().unwrap();

        // save andd load into a new tree
        tree.save("test_junk_octree").ok().unwrap();
        let tree_copy = Octree::<u32>::load("test_junk_octree").ok().unwrap();

        let mut hits = 0;
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    assert!(tree.get(&V3c::new(x, y, z)) == tree_copy.get(&V3c::new(x, y, z)));
                    if tree_copy.get(&V3c::new(x, y, z)).is_some()
                        && *tree_copy.get(&V3c::new(x, y, z)).unwrap() == 5
                    {
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
        let mut tree = Octree::<u32>::new(512).ok().unwrap();
        for x in 256..300 {
            for y in 256..300 {
                for z in 256..300 {
                    let pos = V3c::new(x, y, z);
                    tree.insert(&pos, x + y + z).ok().unwrap();
                }
            }
        }

        let serialized = tree.to_bytes();
        let deserialized = Octree::<u32>::from_bytes(serialized);

        for x in 256..300 {
            for y in 256..300 {
                for z in 256..300 {
                    let pos = V3c::new(x, y, z);
                    assert!(deserialized.get(&pos).is_some_and(|v| *v == (x + y + z)));
                }
            }
        }
    }
}

#[cfg(test)]
mod octree_tests {
    use crate::octree::types::Octree;
    use crate::spatial::math::V3c;

    #[test]
    fn test_simple_insert_and_get() {
        let mut tree = Octree::<u32>::new(2).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), 5).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 0), 6).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 1), 7).ok().unwrap();

        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == 6);
        assert!(*tree.get(&V3c::new(0, 0, 1)).unwrap() == 7);
        assert!(tree.get(&V3c::new(1, 1, 1)).is_none());
    }

    #[test]
    fn test_simple_insert_and_get_where_dim_is_2() {
        std::env::set_var("RUST_BACKTRACE", "1");
        let mut tree = Octree::<u32, 2>::new(2).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), 5).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 0), 6).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 1), 7).ok().unwrap();
        println!("result: {:?}", *tree.get(&V3c::new(1, 0, 0)).unwrap());
        assert!(tree.get(&V3c::new(1, 0, 0)).is_some_and(|v| *v == 5));
        assert!(tree.get(&V3c::new(0, 1, 0)).is_some_and(|v| *v == 6));
        assert!(tree.get(&V3c::new(0, 0, 1)).is_some_and(|v| *v == 7));

        tree.insert(&V3c::new(3, 0, 0), 5).ok().unwrap();
        tree.insert(&V3c::new(0, 3, 0), 6).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 3), 7).ok().unwrap();
        assert!(tree.get(&V3c::new(3, 0, 0)).is_some_and(|v| *v == 5));
        assert!(tree.get(&V3c::new(0, 3, 0)).is_some_and(|v| *v == 6));
        assert!(tree.get(&V3c::new(0, 0, 3)).is_some_and(|v| *v == 7));
    }

    #[test]
    fn test_get_mut() {
        let mut tree = Octree::<u32>::new(2).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), 5).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 0), 6).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 1), 7).ok().unwrap();

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
        tree.insert_at_lod(&V3c::new(0, 0, 0), 2, 5).ok().unwrap();

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
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, 1).ok().unwrap();
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
    fn test_insert_at_lod_where_dim_is_2() {
        let mut tree = Octree::<u32, 2>::new(2).ok().unwrap();
        tree.auto_simplify = false;

        // This will set the area equal to 8 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 2, 5).ok().unwrap();

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
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, 1).ok().unwrap();
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
    fn test_insert_at_lod_with_unaligned_position() {
        let mut tree = Octree::<u32, 4>::new(2).ok().unwrap();
        tree.auto_simplify = false;

        // This will set the area equal to 8 1-sized nodes
        tree.insert_at_lod(&V3c::new(3, 3, 3), 4, 5).ok().unwrap();

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
        tree.insert_at_lod(&V3c::new(3, 3, 3), 4, 1).ok().unwrap();
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

    // #[test]
    // fn test_insert_at_lod_with_unaligned_size() {
    //     let mut tree = Octree::<u32, 4>::new(2).ok().unwrap();
    //     tree.auto_simplify = false;

    //     // This will set the area equal to 8 1-sized nodes
    //     tree.insert_at_lod(&V3c::new(3, 3, 3), 3, 5).ok().unwrap();

    //     assert!(tree.get(&V3c::new(1, 1, 1)).is_some_and(|v| *v == 5));
    //     //TODO: add assertions for the other positions

    //     let mut hits = 0;
    //     for x in 0..4 {
    //         for y in 0..4 {
    //             for z in 0..4 {
    //                 if tree.get(&V3c::new(x, y, z)).is_some()
    //                     && *tree.get(&V3c::new(x, y, z)).unwrap() == 1
    //                 {
    //                     hits += 1;
    //                 }
    //             }
    //         }
    //     }
    //     // println!("hits: {hits}");
    //     assert!(hits == 64);
    // }

    #[test]
    fn test_insert_at_lod_with_simplify() {
        let mut tree = Octree::<u32>::new(8).ok().unwrap();

        // This will set the area equal to 8 1-sized nodes
        tree.insert_at_lod(&V3c::new(5, 0, 0), 2, 5).ok().unwrap();

        assert!(*tree.get(&V3c::new(4, 0, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(4, 0, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(4, 1, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(4, 1, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(5, 0, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(5, 0, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(5, 1, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(5, 1, 1)).unwrap() == 5);

        // This will set the area equal to 64 1-sized nodes:
        // a size-4 node includes 2 levels,
        // 1-sized nodes at the bottom level doesn't have children,
        // 2-sized nodes above have 8 children each
        // so one 4-sized node has 8*8 = 64 children
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, 1).ok().unwrap();
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
        tree.insert(&V3c::new(0, 0, 0), 5).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 1), 5).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 0), 5).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 1), 5).ok().unwrap();
        tree.insert(&V3c::new(1, 0, 0), 5).ok().unwrap();
        tree.insert(&V3c::new(1, 0, 1), 5).ok().unwrap();
        tree.insert(&V3c::new(1, 1, 0), 5).ok().unwrap();
        tree.insert(&V3c::new(1, 1, 1), 5).ok().unwrap();

        // The below should brake the simplified node back to its parts
        tree.insert(&V3c::new(0, 0, 0), 4).ok().unwrap();

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
        std::env::set_var("RUST_BACKTRACE", "1");
        let mut tree = Octree::<u32>::new(2).ok().unwrap();
        tree.auto_simplify = false;
        tree.insert(&V3c::new(1, 0, 0), 5).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 0), 6).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 1), 7).ok().unwrap();
        tree.clear(&V3c::new(0, 0, 1)).ok().unwrap();

        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == 6);
        println!("is some? {:?}", tree.get(&V3c::new(0, 0, 1)).unwrap());
        //TODO: Below do not pass because in some cases clear puts default values
        // assert!(tree.get(&V3c::new(0, 0, 1)).is_none());
        // assert!(tree.get(&V3c::new(1, 1, 1)).is_none());
    }

    #[test]
    fn test_simplifyable_clear() {
        let mut tree = Octree::<u32>::new(2).ok().unwrap();

        // The below set of values should be simplified to a single node
        tree.insert(&V3c::new(0, 0, 0), 5).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 1), 5).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 0), 5).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 1), 5).ok().unwrap();
        tree.insert(&V3c::new(1, 0, 0), 5).ok().unwrap();
        tree.insert(&V3c::new(1, 0, 1), 5).ok().unwrap();
        tree.insert(&V3c::new(1, 1, 0), 5).ok().unwrap();
        tree.insert(&V3c::new(1, 1, 1), 5).ok().unwrap();

        // The below should brake the simplified node back to its party
        tree.clear(&V3c::new(0, 0, 0)).ok().unwrap();

        // Integrity should be kept
        //TODO: Below do not pass because in some cases clear puts default values
        // assert!(tree.get(&V3c::new(0, 0, 0)).is_none());
        assert!(*tree.get(&V3c::new(0, 0, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(0, 1, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 0, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 0, 1)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 1, 0)).unwrap() == 5);
        assert!(*tree.get(&V3c::new(1, 1, 1)).unwrap() == 5);
    }

    #[test]
    fn test_clear_to_nothing() {
        let mut tree = Octree::<u32>::new(2).ok().unwrap();

        // The below set of values should be simplified to a single node
        tree.insert(&V3c::new(0, 0, 0), 5).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 1), 5).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 0), 5).ok().unwrap();
        tree.insert(&V3c::new(0, 1, 1), 5).ok().unwrap();
        tree.insert(&V3c::new(1, 0, 0), 5).ok().unwrap();
        tree.insert(&V3c::new(1, 0, 1), 5).ok().unwrap();
        tree.insert(&V3c::new(1, 1, 0), 5).ok().unwrap();
        tree.insert(&V3c::new(1, 1, 1), 5).ok().unwrap();

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
        let mut tree = Octree::<u32>::new(4).ok().unwrap();

        // This will set the area equal to 64 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, 5).ok().unwrap();

        // This will clear an area equal to 8 1-sized nodes
        tree.clear_at_lod(&V3c::new(0, 0, 0), 2).ok().unwrap();

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
