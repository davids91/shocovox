#[cfg(test)]
mod octree_serialization_tests {
    use crate::octree::Octree;
    use crate::octree::V3c;

    #[test]
    fn test_octree_file_io() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();

        // This will set the area equal to 64 1-sized nodes
        tree.insert_at_lod(&V3c::new(0, 0, 0), 4, 5).ok();

        // This will clear an area equal to 8 1-sized nodes
        tree.clear_at_lod(&V3c::new(0, 0, 0), 2).ok();

        // save andd load into a new tree
        tree.save("test_junk_octree").ok();
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
