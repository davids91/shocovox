use crate::octree::types::{Albedo, NodeChildrenArray};
use crate::octree::types::{BrickData, NodeContent};
use crate::octree::VoxelData;
use bendy::{decoding::FromBencode, encoding::ToBencode};

use crate::object_pool::empty_marker;
use crate::octree::{types::NodeChildren, Octree, V3c};

#[test]
fn test_node_brickdata_serialization() {
    let brick_data_empty = BrickData::<Albedo, 4>::Empty;
    let brick_data_solid = BrickData::<Albedo, 2>::Solid(Albedo::default().with_red(50));
    let brick_data_parted =
        BrickData::Parted(Box::new([[[Albedo::default().with_blue(33); 4]; 4]; 4]));

    let brick_data_empty_deserialized =
        BrickData::<Albedo, 4>::from_bencode(&brick_data_empty.to_bencode().ok().unwrap())
            .ok()
            .unwrap();
    let brick_data_solid_deserialized =
        BrickData::<Albedo, 2>::from_bencode(&brick_data_solid.to_bencode().ok().unwrap())
            .ok()
            .unwrap();
    let brick_data_parted_deserialized =
        BrickData::<Albedo, 4>::from_bencode(&brick_data_parted.to_bencode().ok().unwrap())
            .ok()
            .unwrap();

    assert!(brick_data_empty_deserialized == brick_data_empty);
    assert!(brick_data_solid_deserialized == brick_data_solid);
    assert!(brick_data_parted_deserialized == brick_data_parted);
}

#[test]
fn test_nodecontent_serialization() {
    let node_content_nothing = NodeContent::<Albedo, 4>::Nothing;
    let node_content_internal = NodeContent::<Albedo, 8>::Internal(0xAB);
    let node_content_leaf = NodeContent::<Albedo, 2>::Leaf([
        BrickData::<Albedo, 2>::Empty,
        BrickData::<Albedo, 2>::Solid(Albedo::default().with_blue(3)),
        BrickData::<Albedo, 2>::Parted(Box::new([[[Albedo::default().with_green(5); 2]; 2]; 2])),
        BrickData::<Albedo, 2>::Empty,
        BrickData::<Albedo, 2>::Empty,
        BrickData::<Albedo, 2>::Empty,
        BrickData::<Albedo, 2>::Empty,
        BrickData::<Albedo, 2>::Empty,
    ]);
    let node_content_uniform_leaf = NodeContent::<Albedo, 16>::UniformLeaf(
        BrickData::<Albedo, 16>::Solid(Albedo::default().with_blue(3)),
    );

    let node_content_nothing_deserialized =
        NodeContent::<Albedo, 4>::from_bencode(&node_content_nothing.to_bencode().ok().unwrap())
            .ok()
            .unwrap();
    let node_content_internal_deserialized =
        NodeContent::<Albedo, 8>::from_bencode(&node_content_internal.to_bencode().ok().unwrap())
            .ok()
            .unwrap();
    let node_content_leaf_deserialized =
        NodeContent::<Albedo, 2>::from_bencode(&node_content_leaf.to_bencode().ok().unwrap())
            .ok()
            .unwrap();
    let node_content_uniform_leaf_deserialized = NodeContent::<Albedo, 16>::from_bencode(
        &node_content_uniform_leaf.to_bencode().ok().unwrap(),
    )
    .ok()
    .unwrap();

    println!(
        "{:?} <> {:?}",
        node_content_internal, node_content_internal_deserialized,
    );

    assert!(node_content_nothing_deserialized == node_content_nothing);
    assert!(node_content_leaf_deserialized == node_content_leaf);
    assert!(node_content_uniform_leaf_deserialized == node_content_uniform_leaf);

    // Node content internal has a special equality implementation, where there is no equality between internal nodes
    match (node_content_internal_deserialized, node_content_internal) {
        (NodeContent::Internal(bits1), NodeContent::Internal(bits2)) => {
            assert_eq!(bits1, bits2);
        }
        _ => {
            assert!(
                false,
                "Deserialized and Original NodeContent enums should match!"
            )
        }
    }
}

#[test]
fn test_node_children_serialization() {
    let node_children_empty = NodeChildren::new(empty_marker());
    let node_children_filled = NodeChildren {
        empty_marker: empty_marker(),
        content: NodeChildrenArray::Children([1, 2, 3, 4, 5, 6, 7, 8]),
    };
    let node_children_bitmap = NodeChildren {
        empty_marker: empty_marker(),
        content: NodeChildrenArray::OccupancyBitmap(666),
    };
    let node_children_bitmaps = NodeChildren {
        empty_marker: empty_marker(),
        content: NodeChildrenArray::OccupancyBitmaps([1, 2, 3, 4, 5, 6, 7, 8]),
    };

    let serialized_node_children_empty = node_children_empty.to_bencode();
    let serialized_node_children_filled = node_children_filled.to_bencode();
    let serialized_node_children_bitmap = node_children_bitmap.to_bencode();
    let serialized_node_children_bitmaps = node_children_bitmaps.to_bencode();

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
    let deserialized_node_children_bitmaps =
        NodeChildren::from_bencode(&serialized_node_children_bitmaps.ok().unwrap())
            .ok()
            .unwrap();

    assert!(deserialized_node_children_empty == node_children_empty);
    assert!(deserialized_node_children_filled == node_children_filled);
    assert!(deserialized_node_children_bitmap == node_children_bitmap);
    assert!(deserialized_node_children_bitmaps == node_children_bitmaps);
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
    let mut tree = Octree::<Albedo>::new(128).ok().unwrap();
    for x in 100..128 {
        for y in 100..128 {
            for z in 100..128 {
                let pos = V3c::new(x, y, z);
                tree.insert(&pos, (x + y + z).into()).ok().unwrap();
            }
        }
    }

    let serialized = tree.to_bytes();
    let deserialized = Octree::<Albedo>::from_bytes(serialized);

    for x in 100..128 {
        for y in 100..128 {
            for z in 100..128 {
                let pos = V3c::new(x, y, z);
                assert!(deserialized
                    .get(&pos)
                    .is_some_and(|v| *v == ((x + y + z).into())));
            }
        }
    }
}

#[test]
fn test_octree_serialize_where_dim_is_2() {
    let mut tree = Octree::<Albedo, 2>::new(4).ok().unwrap();
    for x in 0..4 {
        for y in 0..4 {
            for z in 0..4 {
                let pos = V3c::new(x, y, z);
                let albedo: Albedo = ((x << 24) + (y << 16) + (z << 8) + 0xFF).into();
                tree.insert(&pos, albedo).ok().unwrap();
            }
        }
    }

    let serialized = tree.to_bytes();
    let deserialized = Octree::<Albedo, 2>::from_bytes(serialized);

    for x in 0..4 {
        for y in 0..4 {
            for z in 0..4 {
                let pos = V3c::new(x, y, z);
                assert!(deserialized
                    .get(&pos)
                    .is_some_and(|v| { *v == ((x << 24) + (y << 16) + (z << 8) + 0xFF).into() }));
            }
        }
    }
}

#[test]
fn test_big_octree_serialize_where_dim_is_2() {
    let mut tree = Octree::<Albedo, 2>::new(128).ok().unwrap();
    for x in 100..128 {
        for y in 100..128 {
            for z in 100..128 {
                let pos = V3c::new(x, y, z);
                tree.insert(&pos, ((x << 24) + (y << 16) + (z << 8) + 0xFF).into())
                    .ok()
                    .unwrap();
            }
        }
    }

    let serialized = tree.to_bytes();
    let deserialized = Octree::<Albedo, 2>::from_bytes(serialized);

    for x in 100..128 {
        for y in 100..128 {
            for z in 100..128 {
                let pos = V3c::new(x, y, z);
                assert!(deserialized
                    .get(&pos)
                    .is_some_and(|v| *v == (((x << 24) + (y << 16) + (z << 8) + 0xFF).into())));
            }
        }
    }
}
