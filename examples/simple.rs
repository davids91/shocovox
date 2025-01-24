use shocovox_rs::{
    octree::{Albedo, Octree, OctreeEntry, V3c, VoxelData},
    voxel_data,
};

fn main() {
    // To create an empty octree the size and brick dimension needs to be set
    const TREE_SIZE: u32 = 64; // The length of the edges of the cube the octree covers ( number of voxels )
    const BRICK_DIMENSION: u32 = 8; // How big should one "group of voxels" should be refer to docs @Octree::new
                                    // If you have no idea what size it should be, 32 is a good reference
    let mut tree: Octree = Octree::new(TREE_SIZE, BRICK_DIMENSION).ok().unwrap();

    // The visual data the octree contains are provided through the ALbedo type
    let voxel_color_red: Albedo = 0xFF0000FF.into(); // RGBA hex codes can be used like this
    let voxel_color_green: Albedo = 0x00FF00FF.into();
    let voxel_color_blue: Albedo = 0x0000FFFF.into();

    // Data can be inserted through a reference to a position inside bounds of the octree
    tree.insert(&V3c::new(0, 0, 0), &voxel_color_red)
        .ok()
        .unwrap();
    tree.insert(&V3c::new(0, 0, 1), &voxel_color_green)
        .ok()
        .unwrap();
    assert_eq!(tree.get(&V3c::new(0, 0, 0)), (&voxel_color_red).into());
    assert_eq!(tree.get(&V3c::new(0, 0, 1)), (&voxel_color_green).into());

    // To overwrite data, just insert it to the same position
    tree.insert(&V3c::new(0, 0, 0), &voxel_color_green)
        .ok()
        .unwrap();
    assert_eq!(tree.get(&V3c::new(0, 0, 0)), (&voxel_color_green).into());
    assert_eq!(tree.get(&V3c::new(0, 0, 1)), (&voxel_color_green).into());

    // custom data can also be stored inside the octree, e.g. u32 ( most number types by default )
    tree.insert(&V3c::new(0, 1, 1), voxel_data!(&0xBEEF))
        .ok()
        .unwrap();
    assert_eq!(tree.get(&V3c::new(0, 1, 1)), voxel_data!(&0xBEEF).into());

    // it can also be stored next to visual information
    tree.insert(
        &V3c::new(1, 0, 0),
        (&voxel_color_green, &0xBEEF), //BEWARE: not fresh, do not eat
    )
    .ok()
    .unwrap();
    assert_eq!(
        tree.get(&V3c::new(1, 0, 0)),
        (&voxel_color_green, &0xBEEF).into()
    );

    // updating only one component of the voxel is also possible
    tree.update(&V3c::new(1, 0, 0), &voxel_color_red)
        .ok()
        .unwrap();
    assert_eq!(
        tree.get(&V3c::new(1, 0, 0)),
        (&voxel_color_red, &0xBEEF).into()
    );
    tree.update(&V3c::new(1, 0, 0), voxel_data!(&0xFACEFEED))
        .ok()
        .unwrap();
    assert_eq!(
        tree.get(&V3c::new(1, 0, 0)),
        (&voxel_color_red, &0xFACEFEED).into()
    );

    // The below will do nothing though..
    tree.insert(&V3c::new(1, 0, 0), voxel_data!()).ok().unwrap();
    tree.insert(&V3c::new(1, 0, 0), &Albedo::default())
        .ok()
        .unwrap();

    // use clear instead!
    // There is no way to only clear one component of a voxel,
    // both color and data information will be erased through clear
    tree.clear(&V3c::new(1, 0, 0)).ok().unwrap();
    assert_eq!(tree.get(&V3c::new(1, 0, 0)), voxel_data!());

    // data can also be inserted in bulk..
    tree.insert_at_lod(&V3c::new(0, 0, 0), 16, &voxel_color_blue)
        .ok()
        .unwrap();
    for x in 0..16 {
        for y in 0..16 {
            for z in 0..16 {
                assert_eq!(tree.get(&V3c::new(x, y, z)), (&voxel_color_blue).into());
            }
        }
    }

    // ..or cleared in bulk!
    // Both insert and clear bulk operations update the data until the end of one target brick
    // In the below example, voxel from 5,5,5 until 8,8,8 will be cleared
    // instead of 5,5,5 -/-> 13,13,13.
    tree.clear_at_lod(&V3c::new(5, 5, 5), 8).ok().unwrap();
    for x in 5..8 {
        for y in 5..8 {
            for z in 5..8 {
                assert_eq!(tree.get(&V3c::new(x, y, z)), voxel_data!());
            }
        }
    }

    // The maximum update size in a bulk operation depends on the brick dimension,
    // the maximum it can overwrite is (2 * brick_dimension)
    // If the target position is a multiple of (2 * brick_dimension)
    tree.clear_at_lod(&V3c::new(0, 0, 0), 32).ok().unwrap();
    for x in 0..32 {
        for y in 0..32 {
            for z in 0..32 {
                assert_eq!(tree.get(&V3c::new(x, y, z)), voxel_data!());
            }
        }
    }

    // It sounds a bit tricky at first, but totally managable:
    // One leaf node (the biggest updatable area) is the size of 8 voxel bricks, packed together into a cube
    // The leaf node at the lowest position starts at (0,0,0)
    // The leaf node at the lowest position ends at (brick_dimension * 2, brick_dimension * 2, brick_dimension * 2)
    // Each leaf starts at a multiple of (2 * brick_dimension), which is the smallest corner of a leaf node

    // You can also use your own data types to be stored in an octree
    // You have to implement some traits(e.g. VoxelData) for it though. See below!
    let _custom_octree: Octree<MyAwesomeData> = Octree::new(8, 2).ok().unwrap();
}

#[derive(Default, Clone, Eq, PartialEq, Hash)]
struct MyAwesomeData {
    data_field: i64,
}

// The trait VoxelData is required in order to differentiate between empty and non-empty contents of a voxel
impl VoxelData for MyAwesomeData {
    fn is_empty(&self) -> bool {
        self.data_field == 69420
    }
}

// To be able to save and load the data in the octree the crate bendy is used for now
// So the traits below need to be implementd for bytecode serialization
use bendy::{
    decoding::{FromBencode, Object},
    encoding::{SingleItemEncoder, ToBencode},
};
impl ToBencode for MyAwesomeData {
    const MAX_DEPTH: usize = 1;
    fn encode(&self, encoder: SingleItemEncoder<'_>) -> Result<(), bendy::encoding::Error> {
        encoder.emit_int(self.data_field)
    }
}

impl FromBencode for MyAwesomeData {
    fn decode_bencode_object(object: Object<'_, '_>) -> Result<Self, bendy::decoding::Error> {
        match object {
            Object::Integer(i) => Ok(MyAwesomeData {
                data_field: i.parse()?,
            }),
            _ => Err(bendy::decoding::Error::unexpected_field(
                "Expected a single integer from bytestream",
            )),
        }
    }
}
