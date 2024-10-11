use crate::object_pool::ObjectPool;
use crate::octree::{
    types::{BrickData, NodeChildren, NodeChildrenArray, NodeContent},
    Albedo, Octree, VoxelData,
};
use bendy::{
    decoding::{FromBencode, ListDecoder, Object},
    encoding::{Encoder, Error as BencodeError, SingleItemEncoder, ToBencode},
};

///####################################################################################
/// BrickData
///####################################################################################
impl<T, const DIM: usize> ToBencode for BrickData<T, DIM>
where
    T: Default + Clone + PartialEq + VoxelData,
{
    const MAX_DEPTH: usize = 3;

    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), BencodeError> {
        match self {
            BrickData::Empty => encoder.emit_str("#b"),
            BrickData::Solid(voxel) => encoder.emit_list(|e| {
                e.emit_str("##b")?;
                Self::encode_single(voxel, e)
            }),
            BrickData::Parted(brick) => encoder.emit_list(|e| {
                e.emit_str("###b")?;
                for z in 0..DIM {
                    for y in 0..DIM {
                        for x in 0..DIM {
                            Self::encode_single(&brick[x][y][z], e)?;
                        }
                    }
                }
                Ok(())
            }),
        }
    }
}

impl<T, const DIM: usize> FromBencode for BrickData<T, DIM>
where
    T: Eq + Default + Clone + Copy + PartialEq + VoxelData,
{
    fn decode_bencode_object(data: Object) -> Result<Self, bendy::decoding::Error> {
        match data {
            Object::Bytes(b) => {
                debug_assert_eq!(
                    String::from_utf8(b.to_vec())
                        .unwrap_or("".to_string())
                        .as_str(),
                    "#b"
                );
                Ok(BrickData::Empty)
            }
            Object::List(mut list) => {
                let is_solid = match list.next_object()?.unwrap() {
                    Object::Bytes(b) => {
                        match String::from_utf8(b.to_vec())
                            .unwrap_or("".to_string())
                            .as_str()
                        {
                            "##b" => Ok(true),   // The content is a single voxel
                            "###b" => Ok(false), // The content is a brick of voxels
                            misc => Err(bendy::decoding::Error::unexpected_token(
                                "A NodeContent Identifier string, which is either # or ##",
                                "The string ".to_owned() + misc,
                            )),
                        }
                    }
                    _ => Err(bendy::decoding::Error::unexpected_token(
                        "BrickData string identifier",
                        "Something else",
                    )),
                }?;
                if is_solid {
                    Ok(BrickData::Solid(Self::decode_single(&mut list)?))
                } else {
                    let mut brick_data = Box::new([[[T::default(); DIM]; DIM]; DIM]);
                    for z in 0..DIM {
                        for y in 0..DIM {
                            for x in 0..DIM {
                                brick_data[x][y][z] = Self::decode_single(&mut list).unwrap();
                            }
                        }
                    }
                    Ok(BrickData::Parted(brick_data))
                }
            }
            _ => Err(bendy::decoding::Error::unexpected_token(
                "A NodeContent Object, either a List or a ByteString",
                "Something else",
            )),
        }
    }
}

impl<'obj, 'ser, T, const DIM: usize> BrickData<T, DIM>
where
    T: Clone + VoxelData + PartialEq,
{
    fn encode_single(data: &T, encoder: &mut Encoder) -> Result<(), BencodeError> {
        let color = data.albedo();
        encoder.emit(color.r)?;
        encoder.emit(color.g)?;
        encoder.emit(color.b)?;
        encoder.emit(color.a)?;
        encoder.emit(data.user_data())
    }

    fn decode_single(list: &mut ListDecoder<'obj, 'ser>) -> Result<T, bendy::decoding::Error> {
        let r = match list.next_object()?.unwrap() {
            Object::Integer(i) => Ok(i.parse::<u8>().ok().unwrap()),
            _ => Err(bendy::decoding::Error::unexpected_token(
                "int field red color component",
                "Something else",
            )),
        }?;
        let g = match list.next_object()?.unwrap() {
            Object::Integer(i) => Ok(i.parse::<u8>().ok().unwrap()),
            _ => Err(bendy::decoding::Error::unexpected_token(
                "int field green color component",
                "Something else",
            )),
        }?;
        let b = match list.next_object()?.unwrap() {
            Object::Integer(i) => Ok(i.parse::<u8>().ok().unwrap()),
            _ => Err(bendy::decoding::Error::unexpected_token(
                "int field blue color component",
                "Something else",
            )),
        }?;
        let a = match list.next_object()?.unwrap() {
            Object::Integer(i) => Ok(i.parse::<u8>().ok().unwrap()),
            _ => Err(bendy::decoding::Error::unexpected_token(
                "int field alpha color component",
                "Something else",
            )),
        }?;
        let user_data = match list.next_object()?.unwrap() {
            Object::Integer(i) => i.parse::<u32>().ok().unwrap(),
            _ => 0,
        };
        let albedo = Albedo::default()
            .with_red(r)
            .with_green(g)
            .with_blue(b)
            .with_alpha(a);
        Ok(VoxelData::new(albedo, user_data))
    }
}

///####################################################################################
/// NodeContent
///####################################################################################
impl<T, const DIM: usize> ToBencode for NodeContent<T, DIM>
where
    T: Default + Clone + PartialEq + VoxelData,
{
    const MAX_DEPTH: usize = 8;
    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), BencodeError> {
        match self {
            NodeContent::Nothing => encoder.emit_str("#"),
            NodeContent::Internal(occupied_bits) => encoder.emit_list(|e| {
                e.emit_str("##")?;
                e.emit_int(*occupied_bits)
            }),
            NodeContent::Leaf(bricks) => encoder.emit_list(|e| {
                e.emit_str("###")?;
                e.emit(bricks[0].clone())?;
                e.emit(bricks[1].clone())?;
                e.emit(bricks[2].clone())?;
                e.emit(bricks[3].clone())?;
                e.emit(bricks[4].clone())?;
                e.emit(bricks[5].clone())?;
                e.emit(bricks[6].clone())?;
                e.emit(bricks[7].clone())
            }),
            NodeContent::UniformLeaf(brick) => encoder.emit_list(|e| {
                e.emit_str("##u#")?;
                e.emit(brick.clone())
            }),
        }
    }
}

impl<T, const DIM: usize> FromBencode for NodeContent<T, DIM>
where
    T: Eq + Default + Clone + Copy + PartialEq + VoxelData,
{
    fn decode_bencode_object(data: Object) -> Result<Self, bendy::decoding::Error> {
        match data {
            Object::List(mut list) => {
                let (is_leaf, is_uniform) = match list.next_object()?.unwrap() {
                    Object::Bytes(b) => {
                        match String::from_utf8(b.to_vec())
                            .unwrap_or("".to_string())
                            .as_str()
                        {
                            "##" => {
                                // The content is an internal Node
                                Ok((false, false))
                            }
                            "###" => {
                                // The content is a leaf
                                Ok((true, false))
                            }
                            "##u#" => {
                                // The content is a uniform leaf
                                Ok((true, true))
                            }
                            misc => Err(bendy::decoding::Error::unexpected_token(
                                "A NodeContent Identifier string, which is either # or ##",
                                "The string ".to_owned() + misc,
                            )),
                        }
                    }
                    _ => Err(bendy::decoding::Error::unexpected_token(
                        "A NodeContent Identifier, which is a string",
                        "Something else",
                    )),
                }?;

                if !is_leaf && !is_uniform {
                    let occupied_bits;
                    match list.next_object()?.unwrap() {
                        Object::Integer(i) => occupied_bits = i.parse::<u32>().ok().unwrap(),
                        _ => {
                            return Err(bendy::decoding::Error::unexpected_token(
                                "int field for Internal Node Occupancy bitmap",
                                "Something else",
                            ))
                        }
                    };
                    return Ok(NodeContent::Internal(occupied_bits as u8));
                }

                if is_leaf && !is_uniform {
                    let mut leaf_data = [
                        BrickData::Empty,
                        BrickData::Empty,
                        BrickData::Empty,
                        BrickData::Empty,
                        BrickData::Empty,
                        BrickData::Empty,
                        BrickData::Empty,
                        BrickData::Empty,
                    ];
                    leaf_data[0] = BrickData::decode_bencode_object(list.next_object()?.unwrap())?;
                    leaf_data[1] = BrickData::decode_bencode_object(list.next_object()?.unwrap())?;
                    leaf_data[2] = BrickData::decode_bencode_object(list.next_object()?.unwrap())?;
                    leaf_data[3] = BrickData::decode_bencode_object(list.next_object()?.unwrap())?;
                    leaf_data[4] = BrickData::decode_bencode_object(list.next_object()?.unwrap())?;
                    leaf_data[5] = BrickData::decode_bencode_object(list.next_object()?.unwrap())?;
                    leaf_data[6] = BrickData::decode_bencode_object(list.next_object()?.unwrap())?;
                    leaf_data[7] = BrickData::decode_bencode_object(list.next_object()?.unwrap())?;
                    return Ok(NodeContent::Leaf(leaf_data));
                }

                if is_leaf && is_uniform {
                    return Ok(NodeContent::UniformLeaf(BrickData::decode_bencode_object(
                        list.next_object()?.unwrap(),
                    )?));
                }
                panic!(
                    "The logical combination of !is_leaf and is_uniform should never be reached"
                );
            }
            Object::Bytes(b) => {
                assert!(String::from_utf8(b.to_vec()).unwrap_or("".to_string()) == "#");
                Ok(NodeContent::Nothing)
            }
            _ => Err(bendy::decoding::Error::unexpected_token(
                "A NodeContent Object, either a List or a ByteString",
                "Something else",
            )),
        }
    }
}

///####################################################################################
/// NodeChildren
///####################################################################################
// using generic arguments means the default key needs to be serialzied along with the data, which means a lot of wasted space..
// so serialization for the current ObjectPool key is adequate; The engineering hour cost of implementing new serialization logic
// every time the ObjectPool::Itemkey type changes is acepted.
impl ToBencode for NodeChildren<u32> {
    const MAX_DEPTH: usize = 2;
    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), BencodeError> {
        match &self.content {
            NodeChildrenArray::Children(c) => encoder.emit_list(|e| {
                e.emit_str("##c##")?;
                e.emit(c[0])?;
                e.emit(c[1])?;
                e.emit(c[2])?;
                e.emit(c[3])?;
                e.emit(c[4])?;
                e.emit(c[5])?;
                e.emit(c[6])?;
                e.emit(c[7])
            }),
            NodeChildrenArray::NoChildren => encoder.emit_str("##x##"),
            NodeChildrenArray::OccupancyBitmap(map) => encoder.emit_list(|e| {
                e.emit_str("##b##")?;
                e.emit(map)
            }),
            NodeChildrenArray::OccupancyBitmaps(maps) => encoder.emit_list(|e| {
                e.emit_str("##bs##")?;
                e.emit(maps[0])?;
                e.emit(maps[1])?;
                e.emit(maps[2])?;
                e.emit(maps[3])?;
                e.emit(maps[4])?;
                e.emit(maps[5])?;
                e.emit(maps[6])?;
                e.emit(maps[7])
            }),
        }
    }
}

impl FromBencode for NodeChildren<u32> {
    fn decode_bencode_object(data: Object) -> Result<Self, bendy::decoding::Error> {
        use crate::object_pool::empty_marker;
        match data {
            Object::List(mut list) => {
                let marker = String::decode_bencode_object(list.next_object()?.unwrap())?;
                match marker.as_str() {
                    "##c##" => {
                        let mut c = Vec::new();
                        for _ in 0..8 {
                            c.push(
                                u32::decode_bencode_object(list.next_object()?.unwrap())
                                    .ok()
                                    .unwrap(),
                            );
                        }
                        Ok(NodeChildren {
                            empty_marker: empty_marker(),
                            content: NodeChildrenArray::Children(c.try_into().ok().unwrap()),
                        })
                    }
                    "##b##" => Ok(NodeChildren {
                        empty_marker: empty_marker(),
                        content: NodeChildrenArray::OccupancyBitmap(u64::decode_bencode_object(
                            list.next_object()?.unwrap(),
                        )?),
                    }),
                    "##bs##" => {
                        let mut c = Vec::new();
                        for _ in 0..8 {
                            c.push(
                                u64::decode_bencode_object(list.next_object()?.unwrap())
                                    .ok()
                                    .unwrap(),
                            );
                        }
                        Ok(NodeChildren {
                            empty_marker: empty_marker(),
                            content: NodeChildrenArray::OccupancyBitmaps(
                                c.try_into().ok().unwrap(),
                            ),
                        })
                    }
                    s => Err(bendy::decoding::Error::unexpected_token(
                        "A NodeChildren marker, either ##b##, ##bs## or ##c##",
                        s,
                    )),
                }
            }
            Object::Bytes(b) => {
                debug_assert_eq!(
                    String::from_utf8(b.to_vec())
                        .unwrap_or("".to_string())
                        .as_str(),
                    "##x##"
                );
                Ok(NodeChildren::new(empty_marker()))
            }
            _ => Err(bendy::decoding::Error::unexpected_token(
                "A NodeChildren Object, Either a List or a ByteString",
                "Something else",
            )),
        }
    }
}

///####################################################################################
/// Octree
///####################################################################################
impl<T, const DIM: usize> ToBencode for Octree<T, DIM>
where
    T: Default + Clone + PartialEq + VoxelData,
{
    const MAX_DEPTH: usize = 10;
    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), BencodeError> {
        encoder.emit_list(|e| {
            e.emit_int(self.auto_simplify as u8)?;
            e.emit_int(self.octree_size)?;
            e.emit(&self.nodes)?;
            e.emit(&self.node_children)
        })
    }
}

impl<T, const DIM: usize> FromBencode for Octree<T, DIM>
where
    T: Eq + Default + Clone + Copy + VoxelData,
{
    fn decode_bencode_object(data: Object) -> Result<Self, bendy::decoding::Error> {
        match data {
            Object::List(mut list) => {
                let auto_simplify = match list.next_object()?.unwrap() {
                    Object::Integer("0") => Ok(false),
                    Object::Integer("1") => Ok(true),
                    Object::Integer(i) => Err(bendy::decoding::Error::unexpected_token(
                        "boolean field auto_simplify",
                        format!("the number: {}", i),
                    )),
                    _ => Err(bendy::decoding::Error::unexpected_token(
                        "boolean field auto_simplify",
                        "Something else",
                    )),
                }?;

                let root_size = match list.next_object()?.unwrap() {
                    Object::Integer(i) => Ok(i.parse::<u32>().ok().unwrap()),
                    _ => Err(bendy::decoding::Error::unexpected_token(
                        "int field root_size",
                        "Something else",
                    )),
                }?;
                let nodes = ObjectPool::<NodeContent<T, DIM>>::decode_bencode_object(
                    list.next_object()?.unwrap(),
                )?;
                let node_children = Vec::decode_bencode_object(list.next_object()?.unwrap())?;
                Ok(Self {
                    auto_simplify,
                    octree_size: root_size,
                    nodes,
                    node_children,
                })
            }
            _ => Err(bendy::decoding::Error::unexpected_token("List", "not List")),
        }
    }
}
