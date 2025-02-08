use crate::octree::{
    types::{NodeConnection, NodeData},
    {Albedo, ObjectPool, Octree},
};
use std::{collections::HashMap, hash::Hash};

impl<'de, T> serde::Deserialize<'de> for Octree<T>
where
    T: Default + Clone + Eq + Hash,
    T: serde::Deserialize<'de>,
    T: serde::__private::Default,
{
    fn deserialize<__D>(__deserializer: __D) -> serde::__private::Result<Self, __D::Error>
    where
        __D: serde::Deserializer<'de>,
    {
        #[allow(non_camel_case_types)]
        #[doc(hidden)]
        enum __Field {
            __field0,
            __field1,
            __field2,
            __field3,
            __field4,
            __field5,
            __field6,
            __ignore,
        }
        #[doc(hidden)]
        struct __FieldVisitor;
        impl<'de> serde::de::Visitor<'de> for __FieldVisitor {
            type Value = __Field;
            fn expecting(
                &self,
                __formatter: &mut serde::__private::Formatter,
            ) -> serde::__private::fmt::Result {
                serde::__private::Formatter::write_str(__formatter, "field identifier")
            }
            fn visit_u64<__E>(self, __value: u64) -> serde::__private::Result<Self::Value, __E>
            where
                __E: serde::de::Error,
            {
                match __value {
                    0u64 => serde::__private::Ok(__Field::__field0),
                    1u64 => serde::__private::Ok(__Field::__field1),
                    2u64 => serde::__private::Ok(__Field::__field2),
                    3u64 => serde::__private::Ok(__Field::__field3),
                    4u64 => serde::__private::Ok(__Field::__field4),
                    5u64 => serde::__private::Ok(__Field::__field5),
                    6u64 => serde::__private::Ok(__Field::__field6),
                    _ => serde::__private::Ok(__Field::__ignore),
                }
            }
            fn visit_str<__E>(self, __value: &str) -> serde::__private::Result<Self::Value, __E>
            where
                __E: serde::de::Error,
            {
                match __value {
                    "auto_simplify" => serde::__private::Ok(__Field::__field0),
                    "brick_dim" => serde::__private::Ok(__Field::__field1),
                    "octree_size" => serde::__private::Ok(__Field::__field2),
                    "nodes" => serde::__private::Ok(__Field::__field3),
                    "node_children" => serde::__private::Ok(__Field::__field4),
                    "voxel_color_palette" => serde::__private::Ok(__Field::__field5),
                    "voxel_data_palette" => serde::__private::Ok(__Field::__field6),
                    _ => serde::__private::Ok(__Field::__ignore),
                }
            }
            fn visit_bytes<__E>(self, __value: &[u8]) -> serde::__private::Result<Self::Value, __E>
            where
                __E: serde::de::Error,
            {
                match __value {
                    b"auto_simplify" => serde::__private::Ok(__Field::__field0),
                    b"brick_dim" => serde::__private::Ok(__Field::__field1),
                    b"octree_size" => serde::__private::Ok(__Field::__field2),
                    b"nodes" => serde::__private::Ok(__Field::__field3),
                    b"node_children" => serde::__private::Ok(__Field::__field4),
                    b"voxel_color_palette" => serde::__private::Ok(__Field::__field5),
                    b"voxel_data_palette" => serde::__private::Ok(__Field::__field6),
                    _ => serde::__private::Ok(__Field::__ignore),
                }
            }
        }

        impl<'de> serde::Deserialize<'de> for __Field {
            #[inline]
            fn deserialize<__D>(__deserializer: __D) -> serde::__private::Result<Self, __D::Error>
            where
                __D: serde::Deserializer<'de>,
            {
                serde::Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
            }
        }
        #[doc(hidden)]
        struct __Visitor<'de, T>
        where
            T: Default + Clone + Eq + Hash,
            T: serde::Deserialize<'de>,
            T: serde::__private::Default,
        {
            marker: serde::__private::PhantomData<Octree<T>>,
            lifetime: serde::__private::PhantomData<&'de ()>,
        }

        impl<'de, T> serde::de::Visitor<'de> for __Visitor<'de, T>
        where
            T: Default + Clone + Eq + Hash,
            T: serde::Deserialize<'de>,
            T: serde::__private::Default,
        {
            type Value = Octree<T>;
            fn expecting(
                &self,
                __formatter: &mut serde::__private::Formatter,
            ) -> serde::__private::fmt::Result {
                serde::__private::Formatter::write_str(__formatter, "struct Octree")
            }
            #[inline]
            fn visit_seq<__A>(
                self,
                mut __seq: __A,
            ) -> serde::__private::Result<Self::Value, __A::Error>
            where
                __A: serde::de::SeqAccess<'de>,
            {
                let __field0 = match serde::de::SeqAccess::next_element::<bool>(&mut __seq)? {
                    serde::__private::Some(__value) => __value,
                    serde::__private::None => {
                        return serde::__private::Err(serde::de::Error::invalid_length(
                            0usize,
                            &"struct Octree with 7 elements",
                        ));
                    }
                };
                let __field1 = match serde::de::SeqAccess::next_element::<u32>(&mut __seq)? {
                    serde::__private::Some(__value) => __value,
                    serde::__private::None => {
                        return serde::__private::Err(serde::de::Error::invalid_length(
                            1usize,
                            &"struct Octree with 7 elements",
                        ));
                    }
                };
                let __field2 = match serde::de::SeqAccess::next_element::<u32>(&mut __seq)? {
                    serde::__private::Some(__value) => __value,
                    serde::__private::None => {
                        return serde::__private::Err(serde::de::Error::invalid_length(
                            2usize,
                            &"struct Octree with 7 elements",
                        ));
                    }
                };
                let __field3 =
                    match serde::de::SeqAccess::next_element::<ObjectPool<NodeData>>(&mut __seq)? {
                        serde::__private::Some(__value) => __value,
                        serde::__private::None => {
                            return serde::__private::Err(serde::de::Error::invalid_length(
                                3usize,
                                &"struct Octree with 7 elements",
                            ));
                        }
                    };
                let __field4 =
                    match serde::de::SeqAccess::next_element::<Vec<NodeConnection>>(&mut __seq)? {
                        serde::__private::Some(__value) => __value,
                        serde::__private::None => {
                            return serde::__private::Err(serde::de::Error::invalid_length(
                                4usize,
                                &"struct Octree with 7 elements",
                            ));
                        }
                    };
                let __field5 = match serde::de::SeqAccess::next_element::<Vec<Albedo>>(&mut __seq)?
                {
                    serde::__private::Some(__value) => __value,
                    serde::__private::None => {
                        return serde::__private::Err(serde::de::Error::invalid_length(
                            5usize,
                            &"struct Octree with 7 elements",
                        ));
                    }
                };
                let __field6 = match serde::de::SeqAccess::next_element::<Vec<T>>(&mut __seq)? {
                    serde::__private::Some(__value) => __value,
                    serde::__private::None => {
                        return serde::__private::Err(serde::de::Error::invalid_length(
                            6usize,
                            &"struct Octree with 7 elements",
                        ));
                    }
                };
                let __field7 = serde::__private::Default::default();
                let __field8 = serde::__private::Default::default();
                serde::__private::Ok(Octree {
                    auto_simplify: __field0,
                    brick_dim: __field1,
                    octree_size: __field2,
                    nodes: __field3,
                    node_children: __field4,
                    voxel_color_palette: __field5,
                    voxel_data_palette: __field6,
                    map_to_color_index_in_palette: __field7,
                    map_to_data_index_in_palette: __field8,
                })
            }
            #[inline]
            fn visit_map<__A>(
                self,
                mut __map: __A,
            ) -> serde::__private::Result<Self::Value, __A::Error>
            where
                __A: serde::de::MapAccess<'de>,
            {
                let mut __field0: serde::__private::Option<bool> = serde::__private::None;
                let mut __field1: serde::__private::Option<u32> = serde::__private::None;
                let mut __field2: serde::__private::Option<u32> = serde::__private::None;
                let mut __field3: serde::__private::Option<ObjectPool<NodeData>> =
                    serde::__private::None;
                let mut __field4: serde::__private::Option<Vec<NodeConnection>> =
                    serde::__private::None;
                let mut __field5: serde::__private::Option<Vec<Albedo>> = serde::__private::None;
                let mut __field6: serde::__private::Option<Vec<T>> = serde::__private::None;
                while let serde::__private::Some(__key) =
                    serde::de::MapAccess::next_key::<__Field>(&mut __map)?
                {
                    match __key {
                        __Field::__field0 => {
                            if serde::__private::Option::is_some(&__field0) {
                                return serde::__private::Err(
                                    <__A::Error as serde::de::Error>::duplicate_field(
                                        "auto_simplify",
                                    ),
                                );
                            }
                            __field0 =
                                serde::__private::Some(serde::de::MapAccess::next_value::<bool>(
                                    &mut __map,
                                )?);
                        }
                        __Field::__field1 => {
                            if serde::__private::Option::is_some(&__field1) {
                                return serde::__private::Err(
                                    <__A::Error as serde::de::Error>::duplicate_field("brick_dim"),
                                );
                            }
                            __field1 =
                                serde::__private::Some(serde::de::MapAccess::next_value::<u32>(
                                    &mut __map,
                                )?);
                        }
                        __Field::__field2 => {
                            if serde::__private::Option::is_some(&__field2) {
                                return serde::__private::Err(
                                    <__A::Error as serde::de::Error>::duplicate_field(
                                        "octree_size",
                                    ),
                                );
                            }
                            __field2 =
                                serde::__private::Some(serde::de::MapAccess::next_value::<u32>(
                                    &mut __map,
                                )?);
                        }
                        __Field::__field3 => {
                            if serde::__private::Option::is_some(&__field3) {
                                return serde::__private::Err(
                                    <__A::Error as serde::de::Error>::duplicate_field("nodes"),
                                );
                            }
                            __field3 = serde::__private::Some(serde::de::MapAccess::next_value::<
                                ObjectPool<NodeData>,
                            >(
                                &mut __map
                            )?);
                        }
                        __Field::__field4 => {
                            if serde::__private::Option::is_some(&__field4) {
                                return serde::__private::Err(
                                    <__A::Error as serde::de::Error>::duplicate_field(
                                        "node_children",
                                    ),
                                );
                            }
                            __field4 = serde::__private::Some(serde::de::MapAccess::next_value::<
                                Vec<NodeConnection>,
                            >(
                                &mut __map
                            )?);
                        }
                        __Field::__field5 => {
                            if serde::__private::Option::is_some(&__field5) {
                                return serde::__private::Err(
                                    <__A::Error as serde::de::Error>::duplicate_field(
                                        "voxel_color_palette",
                                    ),
                                );
                            }
                            __field5 = serde::__private::Some(serde::de::MapAccess::next_value::<
                                Vec<Albedo>,
                            >(
                                &mut __map
                            )?);
                        }
                        __Field::__field6 => {
                            if serde::__private::Option::is_some(&__field6) {
                                return serde::__private::Err(
                                    <__A::Error as serde::de::Error>::duplicate_field(
                                        "voxel_data_palette",
                                    ),
                                );
                            }
                            __field6 =
                                serde::__private::Some(serde::de::MapAccess::next_value::<Vec<T>>(
                                    &mut __map,
                                )?);
                        }
                        _ => {
                            let _ = serde::de::MapAccess::next_value::<serde::de::IgnoredAny>(
                                &mut __map,
                            )?;
                        }
                    }
                }
                let __field0 = match __field0 {
                    serde::__private::Some(__field0) => __field0,
                    serde::__private::None => serde::__private::de::missing_field("auto_simplify")?,
                };
                let __field1 = match __field1 {
                    serde::__private::Some(__field1) => __field1,
                    serde::__private::None => serde::__private::de::missing_field("brick_dim")?,
                };
                let __field2 = match __field2 {
                    serde::__private::Some(__field2) => __field2,
                    serde::__private::None => serde::__private::de::missing_field("octree_size")?,
                };
                let __field3 = match __field3 {
                    serde::__private::Some(__field3) => __field3,
                    serde::__private::None => serde::__private::de::missing_field("nodes")?,
                };
                let __field4 = match __field4 {
                    serde::__private::Some(__field4) => __field4,
                    serde::__private::None => serde::__private::de::missing_field("node_children")?,
                };
                let __field5 = match __field5 {
                    serde::__private::Some(__field5) => __field5,
                    serde::__private::None => {
                        serde::__private::de::missing_field("voxel_color_palette")?
                    }
                };
                let __field6 = match __field6 {
                    serde::__private::Some(__field6) => __field6,
                    serde::__private::None => {
                        serde::__private::de::missing_field("voxel_data_palette")?
                    }
                };

                // The following are the two fields this whole implementation is required
                // There is no other reason not to use the derive macro `Deserialize`
                // As there is an `ArbitraryMapKeysUnsupported` error when trying to serialize them,
                // Re-building them manually is required
                // It's not a bad thing as they won't waste space in data streams this way
                // --{
                let mut map_to_color_index_in_palette = HashMap::new();
                for i in 0..__field5.len() {
                    map_to_color_index_in_palette.insert(__field5[i], i);
                }

                let mut map_to_data_index_in_palette = HashMap::new();
                for i in 0..__field6.len() {
                    map_to_data_index_in_palette.insert(__field6[i].clone(), i);
                }
                // }--

                serde::__private::Ok(Octree {
                    auto_simplify: __field0,
                    brick_dim: __field1,
                    octree_size: __field2,
                    nodes: __field3,
                    node_children: __field4,
                    voxel_color_palette: __field5,
                    voxel_data_palette: __field6,
                    map_to_color_index_in_palette,
                    map_to_data_index_in_palette,
                })
            }
        }
        #[doc(hidden)]
        const FIELDS: &'static [&'static str] = &[
            "auto_simplify",
            "brick_dim",
            "octree_size",
            "nodes",
            "node_children",
            "voxel_color_palette",
            "voxel_data_palette",
        ];
        serde::Deserializer::deserialize_struct(
            __deserializer,
            "Octree",
            FIELDS,
            __Visitor {
                marker: serde::__private::PhantomData::<Octree<T>>,
                lifetime: serde::__private::PhantomData,
            },
        )
    }
}
