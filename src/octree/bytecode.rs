use crate::object_pool::ObjectPool;
use crate::octree::{detail::NodeChildrenArray, NodeChildren, NodeContent, Octree};
use bendy::encoding::{Error as BencodeError, SingleItemEncoder, ToBencode};

impl<T> ToBencode for NodeContent<T>
where
    T: Default + ToBencode,
{
    const MAX_DEPTH: usize = 4;
    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), BencodeError> {
        if self.is_leaf() {
            encoder.emit_list(|e| {
                e.emit_str("###")?;
                e.emit(self.leaf_data())
            })
        } else {
            encoder.emit_str("##x##")
        }
    }
}

use bendy::decoding::{FromBencode, Object};
impl<T> FromBencode for NodeContent<T>
where
    T: FromBencode,
{
    fn decode_bencode_object(data: Object) -> Result<Self, bendy::decoding::Error> {
        match data {
            Object::List(mut list) => {
                list.next_object()?; // Shopuld be "###"
                if let Some(o) = list.next_object()? {
                    Ok(NodeContent::Leaf(T::decode_bencode_object(o)?))
                } else {
                    Err(bendy::decoding::Error::missing_field(
                        "Content of Leaf NodeContent object",
                    ))
                }
                // let s = String::from_utf8(list.next_object()?.unwrap().try_into_bytes()?.to_vec())?;
                // if s == "###" {
                //     if let Some(o) = list.next_object()? {
                //         Ok(NodeContent::Leaf(T::decode_bencode_object(o)?))
                //     } else {
                //         Err(bendy::decoding::Error::missing_field(
                //             "Content of Leaf NodeContent object",
                //         ))
                //     }
                // } else {
                //     Err(bendy::decoding::Error::unexpected_token(
                //         "A NodeContent Object marker for 'something'",
                //         s,
                //     ))
                // }
            }
            Object::Bytes(_b) => {
                Ok(NodeContent::Nothing)
                // let s = String::from_utf8(b.to_vec())?;
                // if s == "##x##" {
                //     Ok(NodeContent::Nothing)
                // } else {
                //     Err(bendy::decoding::Error::unexpected_token(
                //         "A NodeContent Object marker for 'nothing'",
                //         s,
                //     ))
                // }
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
impl ToBencode for NodeChildren<usize> {
    const MAX_DEPTH: usize = 4;
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

impl FromBencode for NodeChildren<usize> {
    fn decode_bencode_object(data: Object) -> Result<Self, bendy::decoding::Error> {
        use crate::object_pool::key_none_value;
        match data {
            Object::List(mut list) => {
                // let mut children_array : [usize; 8];
                let mut c = Vec::new();
                for _ in 0..8 {
                    c.push(
                        usize::decode_bencode_object(list.next_object()?.unwrap())
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
    T: Default + ToBencode + FromBencode,
{
    const MAX_DEPTH: usize = 8;
    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), BencodeError> {
        encoder.emit_list(|e| {
            e.emit_int(self.auto_simplify as u8)?;
            e.emit_int(self.root_node as u8)?;
            e.emit_int(self.root_size as u8)?;
            e.emit(&self.nodes)?;
            e.emit(&self.node_children)
        })
    }
}

impl<T> FromBencode for Octree<T>
where
    T: Default + ToBencode + FromBencode,
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

                let root_node = match list.next_object()?.unwrap() {
                    Object::Integer(i) => Ok(i.parse::<usize>().ok().unwrap()),
                    _ => Err(bendy::decoding::Error::unexpected_token(
                        "int field root_node_key",
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
                    root_node,
                    root_size,
                    nodes,
                    node_children,
                })
            }
            _ => Err(bendy::decoding::Error::unexpected_token("List", "not List")),
        }
    }
}
