use crate::object_pool::ObjectPool;
use crate::octree::{detail::NodeChildrenArray, NodeChildren, NodeContent, Octree, VoxelData};
use bendy::encoding::{Error as BencodeError, SingleItemEncoder, ToBencode};

impl<T> ToBencode for NodeContent<T>
where
    T: Clone + Default + VoxelData,
{
    const MAX_DEPTH: usize = 8;
    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), BencodeError> {
        if self.is_leaf() {
            encoder.emit_list(|e| {
                e.emit_str("###")?;
                let color = self.leaf_data().albedo();
                e.emit(color[0])?;
                e.emit(color[1])?;
                e.emit(color[2])?;
                if let Some(d) = self.leaf_data().user_data() {
                    e.emit(d)
                } else {
                    e.emit_str("##x##")
                }
            })
        } else {
            encoder.emit_str("##x##")
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
                list.next_object()?; // Should be "###"
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
            Object::Bytes(_b) => { // should be "##x##"
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
            e.emit_int(self.root_node as u32)?;
            e.emit_int(self.root_size as u32)?;
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

                let root_node = match list.next_object()?.unwrap() {
                    Object::Integer(i) => Ok(i.parse::<u32>().ok().unwrap()),
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

///####################################################################################
/// Tests
///####################################################################################
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
