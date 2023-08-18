use crate::octree::{NodeContent, Octree};
use crate::object_pool::ObjectPool;
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
                "A NodeContent Object",
                "Something else",
            )),
        }
    }
}


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
