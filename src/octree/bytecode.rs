use crate::object_pool::ObjectPool;
use crate::octree::types::{NodeChildren, NodeChildrenArray, NodeContent, Octree, VoxelData};
use bendy::{
    decoding::ListDecoder,
    encoding::{Encoder, Error as BencodeError, SingleItemEncoder, ToBencode},
};

impl<'obj, 'ser, T: Clone + VoxelData, const DIM: usize> NodeContent<T, DIM> {
    fn encode_single(data: &T, encoder: &mut Encoder) -> Result<(), BencodeError> {
        let color = data.albedo();
        encoder.emit((color.r * 1000.) as i32)?;
        encoder.emit((color.g * 1000.) as i32)?;
        encoder.emit((color.b * 1000.) as i32)?;
        encoder.emit((color.a * 1000.) as i32)?;
        encoder.emit(data.user_data())
    }

    fn decode_single(list: &mut ListDecoder<'obj, 'ser>) -> Result<T, bendy::decoding::Error> {
        let r = match list.next_object()?.unwrap() {
            Object::Integer(i) => Ok(i.parse::<i32>().ok().unwrap()),
            _ => Err(bendy::decoding::Error::unexpected_token(
                "int field red color component",
                "Something else",
            )),
        }?;
        let g = match list.next_object()?.unwrap() {
            Object::Integer(i) => Ok(i.parse::<i32>().ok().unwrap()),
            _ => Err(bendy::decoding::Error::unexpected_token(
                "int field green color component",
                "Something else",
            )),
        }?;
        let b = match list.next_object()?.unwrap() {
            Object::Integer(i) => Ok(i.parse::<i32>().ok().unwrap()),
            _ => Err(bendy::decoding::Error::unexpected_token(
                "int field blue color component",
                "Something else",
            )),
        }?;
        let a = match list.next_object()?.unwrap() {
            Object::Integer(i) => Ok(i.parse::<i32>().ok().unwrap()),
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
            .with_red(r as f32 / 1000.)
            .with_green(g as f32 / 1000.)
            .with_blue(b as f32 / 1000.)
            .with_alpha(a as f32 / 1000.);
        Ok(VoxelData::new(albedo, user_data))
    }
}

impl<T, const DIM: usize> ToBencode for NodeContent<T, DIM>
where
    T: Default + Clone + VoxelData,
{
    const MAX_DEPTH: usize = 8;
    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), BencodeError> {
        match self {
            NodeContent::Nothing => encoder.emit_str("#"),
            NodeContent::Internal(count) => encoder.emit_list(|e| {
                e.emit_str("##")?;
                e.emit_int(*count)
            }),
            NodeContent::Leaf(data) => encoder.emit_list(|e| {
                e.emit_str("###")?;
                for x in data.iter().take(DIM) {
                    for y in x.iter().take(DIM) {
                        for z in y.iter().take(DIM) {
                            NodeContent::<T, DIM>::encode_single(z, e)?;
                        }
                    }
                }
                Ok(())
            }),
        }
    }
}

use bendy::decoding::{FromBencode, Object};

use super::types::Albedo;
impl<T, const DIM: usize> FromBencode for NodeContent<T, DIM>
where
    T: PartialEq + Default + Clone + Copy + VoxelData,
{
    fn decode_bencode_object(data: Object) -> Result<Self, bendy::decoding::Error> {
        match data {
            Object::List(mut list) => {
                let is_leaf = match list.next_object()?.unwrap() {
                    Object::Bytes(b) => {
                        match String::from_utf8(b.to_vec())
                            .unwrap_or("".to_string())
                            .as_str()
                        {
                            "##" => {
                                // The content is an internal Node
                                Ok(false)
                            }
                            "###" => {
                                // The content is a leaf
                                Ok(true)
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
                if !is_leaf {
                    let count;
                    match list.next_object()?.unwrap() {
                        Object::Integer(i) => count = i.parse::<u32>().ok().unwrap(),
                        _ => {
                            return Err(bendy::decoding::Error::unexpected_token(
                                "int field for Internal Node count",
                                "Something else",
                            ))
                        }
                    };
                    Ok(NodeContent::Internal(count as u8))
                } else {
                    Ok(NodeContent::<T, DIM>::Leaf(
                        [[[NodeContent::<T, DIM>::decode_single(&mut list).unwrap(); DIM]; DIM];
                            DIM],
                    ))
                }
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
            NodeChildrenArray::OccupancyBitmap(mask) => encoder.emit_list(|e| {
                e.emit_str("##b##")?;
                e.emit(mask)
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
                        Ok(NodeChildren::from(
                            empty_marker(),
                            c.try_into().ok().unwrap(),
                        ))
                    }
                    "##b##" => Ok(NodeChildren::bitmasked(
                        empty_marker(),
                        u64::decode_bencode_object(list.next_object()?.unwrap())?,
                    )),
                    s => Err(bendy::decoding::Error::unexpected_token(
                        "A NodeChildren marker, either ##b## or ##c##",
                        s,
                    )),
                }
            }
            Object::Bytes(_b) =>
            // Should be "##x##"
            {
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
    T: Default + Clone + VoxelData,
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
    T: PartialEq + Default + Clone + Copy + VoxelData,
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
