use crate::object_pool::ObjectPool;
use crate::octree::types::{NodeChildren, NodeChildrenArray, NodeContent, Octree, VoxelData};
use bendy::encoding::{Error as BencodeError, SingleItemEncoder, ToBencode};

impl<T> ToBencode for NodeContent<T>
where
    T: Clone + Default + VoxelData,
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
                let color = data.albedo();
                e.emit(color[0])?;
                e.emit(color[1])?;
                e.emit(color[2])?;
                if let Some(d) = data.user_data() {
                    e.emit(d)
                } else {
                    e.emit_str("#")
                }
            }),
        }
    }
}

use bendy::decoding::{FromBencode, Object};
impl<T> FromBencode for NodeContent<T>
where
    T: Clone + VoxelData,
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
                    Ok(NodeContent::Internal(count))
                } else {
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
                            "int field red color component",
                            "Something else",
                        )),
                    }?;
                    let b = match list.next_object()?.unwrap() {
                        Object::Integer(i) => Ok(i.parse::<u8>().ok().unwrap()),
                        _ => Err(bendy::decoding::Error::unexpected_token(
                            "int field red color component",
                            "Something else",
                        )),
                    }?;
                    let user_data = match list.next_object()?.unwrap() {
                        Object::Integer(i) => Some(i.parse::<u32>().ok().unwrap()),
                        _ => None,
                    };
                    Ok(NodeContent::Leaf(T::new(r, g, b, user_data)))
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
        }
    }
}

impl FromBencode for NodeChildren<u32> {
    fn decode_bencode_object(data: Object) -> Result<Self, bendy::decoding::Error> {
        use crate::object_pool::key_none_value;
        match data {
            Object::List(mut list) => {
                let mut c = Vec::new();
                for _ in 0..8 {
                    c.push(
                        u32::decode_bencode_object(list.next_object()?.unwrap())
                            .ok()
                            .unwrap(),
                    );
                }
                Ok(NodeChildren::from(
                    key_none_value(),
                    c.try_into().ok().unwrap(),
                ))
            }
            Object::Bytes(_b) =>
            // Should be "##x##"
            {
                Ok(NodeChildren::new(key_none_value()))
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
impl<T> ToBencode for Octree<T>
where
    T: Default + Clone + VoxelData,
{
    const MAX_DEPTH: usize = 10;
    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), BencodeError> {
        encoder.emit_list(|e| {
            e.emit_int(self.auto_simplify as u8)?;
            e.emit_int(self.root_size)?;
            e.emit(&self.nodes)?;
            e.emit(&self.node_children)
        })
    }
}

impl<T> FromBencode for Octree<T>
where
    T: Default + Clone + VoxelData,
{
    fn decode_bencode_object(data: Object) -> Result<Self, bendy::decoding::Error> {
        match data {
            Object::List(mut list) => {
                let auto_simplify = match list.next_object()?.unwrap() {
                    Object::Integer(i) if i == "0" => Ok(false),
                    Object::Integer(i) if i == "1" => Ok(true),
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
                let nodes = ObjectPool::<NodeContent<T>>::decode_bencode_object(
                    list.next_object()?.unwrap(),
                )?;
                let node_children = Vec::decode_bencode_object(list.next_object()?.unwrap())?;
                Ok(Self {
                    auto_simplify,
                    root_size,
                    nodes,
                    node_children,
                })
            }
            _ => Err(bendy::decoding::Error::unexpected_token("List", "not List")),
        }
    }
}
