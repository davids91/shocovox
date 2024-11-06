use crate::object_pool::empty_marker;
use crate::octree::{
    raytracing::bevy::types::{
        OctreeMetaData, ShocoVoxRenderData, ShocoVoxRenderPipeline, SizedNode, Voxelement,
    },
    types::{NodeChildrenArray, NodeContent},
    Octree, V3c, VoxelData,
};
use bevy::{
    ecs::system::{Res, ResMut},
    math::Vec4,
    render::{renderer::RenderDevice, MainWorld},
};
use std::collections::HashMap;

impl<T, const DIM: usize> Octree<T, DIM>
where
    T: VoxelData + PartialEq + Default + Clone + Copy,
{
    fn set_node_meta_inner(
        &self,
        data_meta_bytes: &mut Vec<u32>,
        data_index: usize,
        is_leaf: bool,
        occupancy_bitmap: u8,
    ) {
        // Erase the initial data
        data_meta_bytes[data_index / 2] &= if 0 == data_index % 2 {
            0xFFFF00FAu32
        } else {
            0x00FAFFFFu32
        };

        // Add occupancy bitmap to meta
        data_meta_bytes[data_index / 2] |= if 0 == data_index % 2 {
            (occupancy_bitmap as u32) << 8u32
        } else {
            (occupancy_bitmap as u32) << 24u32
        };

        // Add node is_leaf bit to meta
        if is_leaf {
            data_meta_bytes[data_index / 2] |= if 0 == data_index % 2 {
                0x000000004u32
            } else {
                0x00040000u32
            };
        }

        // Add node is_used bit to meta
        if is_leaf {
            data_meta_bytes[data_index / 2] |= if 0 == data_index % 2 {
                0x000000001u32
            } else {
                0x00010000u32
            };
        }
    }

    fn set_meta_bytes_for_node(
        &self,
        data_meta_bytes: &mut Vec<u32>,
        node_key: usize,
        data_index: usize,
    ) {
        let node = self.nodes.get(node_key);
        match node {
            NodeContent::Leaf(_) => {
                self.set_node_meta_inner(
                    data_meta_bytes,
                    data_index,
                    true,
                    self.occupied_8bit(node_key as u32),
                );
            }
            NodeContent::Internal(occupied_bits) => {
                self.set_node_meta_inner(data_meta_bytes, data_index, false, *occupied_bits);
            }
            _ => {}
        }
    }

    pub fn create_bevy_view(&self) -> ShocoVoxRenderData {
        let octree_meta = OctreeMetaData {
            octree_size: self.octree_size,
            voxel_brick_dim: DIM as u32,
            ambient_light_color: V3c::new(1., 1., 1.),
            ambient_light_position: V3c::new(
                self.octree_size as f32,
                self.octree_size as f32,
                self.octree_size as f32,
            ),
        };

        let mut nodes = Vec::new();
        let mut node_children = Vec::new();
        let mut voxels = Vec::new();
        let mut color_palette = Vec::new();
        // Size of meta for one element is 2 Bytes,
        let mut data_meta_bytes = vec![0u32; self.nodes.len() / 2];

        // Build up Nodes
        let mut map_to_node_index_in_nodes_buffer = HashMap::new();
        for node_key in 0..self.nodes.len() {
            if self.nodes.key_is_valid(node_key) {
                map_to_node_index_in_nodes_buffer.insert(node_key as usize, nodes.len());
                self.set_meta_bytes_for_node(&mut data_meta_bytes, node_key, nodes.len());
                nodes.push(SizedNode {
                    children_start_at: empty_marker(),
                    voxels_start_at: empty_marker(),
                });
            }
        }

        // Build up voxel content
        let mut map_to_color_index_in_palette = HashMap::new();
        for i in 0..self.nodes.len() {
            if !self.nodes.key_is_valid(i) {
                continue;
            }
            nodes[map_to_node_index_in_nodes_buffer[&i]].children_start_at =
                node_children.len() as u32;
            if let NodeContent::Leaf(data) = self.nodes.get(i) {
                debug_assert!(matches!(
                    self.node_children[i].content,
                    NodeChildrenArray::OccupancyBitmap(_)
                ));
                let occupied_bits = match self.node_children[i].content {
                    NodeChildrenArray::OccupancyBitmap(bitmap) => bitmap,
                    _ => panic!("Found Leaf Node without occupancy bitmap!"),
                };
                node_children.extend_from_slice(&[
                    (occupied_bits & 0x00000000FFFFFFFF) as u32,
                    ((occupied_bits & 0xFFFFFFFF00000000) >> 32) as u32,
                ]);
                nodes[map_to_node_index_in_nodes_buffer[&i]].voxels_start_at = voxels.len() as u32;
                for z in 0..DIM {
                    for y in 0..DIM {
                        for x in 0..DIM {
                            let albedo = data[x][y][z].albedo();
                            if !map_to_color_index_in_palette.contains_key(&albedo) {
                                map_to_color_index_in_palette.insert(albedo, color_palette.len());
                                color_palette.push(Vec4::new(
                                    albedo.r as f32 / 255.,
                                    albedo.g as f32 / 255.,
                                    albedo.b as f32 / 255.,
                                    albedo.a as f32 / 255.,
                                ));
                            }
                            let albedo_index = map_to_color_index_in_palette[&albedo];

                            voxels.push(Voxelement {
                                albedo_index: albedo_index as u32,
                                content: data[x][y][z].user_data(),
                            })
                        }
                    }
                }
            } else {
                //Internal nodes
                for c in 0..8 {
                    let child_index = &self.node_children[i][c];
                    if *child_index != self.node_children[i].empty_marker {
                        debug_assert!(map_to_node_index_in_nodes_buffer
                            .contains_key(&(*child_index as usize)));
                        node_children.push(
                            map_to_node_index_in_nodes_buffer[&(*child_index as usize)] as u32,
                        );
                    } else {
                        node_children.push(*child_index);
                    }
                }
            }
        }

        ShocoVoxRenderData {
            do_the_thing: false,
            data_meta_bytes,
            // root_node: SizedNode {
            //     sized_node_meta: self.create_meta(Self::ROOT_NODE_KEY as usize),
            //     children_start_at: empty_marker(),
            //     voxels_start_at: empty_marker(),
            // },
            octree_meta,
            nodes,
            node_children,
            voxels,
            color_palette,
        }
    }

    pub(crate) fn insert_elements_into_cache(
        &self,
        render_data: &mut ShocoVoxRenderData,
        requested_nodes: Vec<u32>,
    ) {
        //TODO: find the first unused element, and overwrite it with the item
    }
}

pub(crate) fn handle_gpu_readback(
    render_device: Res<RenderDevice>,
    svx_data: Option<ResMut<ShocoVoxRenderData>>,
    svx_pipeline: Option<ResMut<ShocoVoxRenderPipeline>>,
) {
    // // Data updates triggered by debug interface
    // if let Some(svx_data) = svx_data {
    //     let mut render_data_mainworld = world.get_resource_mut::<ShocoVoxRenderData>().unwrap();
    //     let svx_pipeline = svx_pipeline.unwrap();
    //     if svx_data.do_the_thing {
    //         // GPU buffer read
    //         // https://docs.rs/bevy/latest/src/gpu_readback/gpu_readback.rs.html
    //         let buffer_slice = svx_pipeline
    //             .readable_cache_bytes_buffer
    //             .as_ref()
    //             .unwrap()
    //             .slice(..);
    //         let (s, r) = crossbeam::channel::unbounded::<()>();
    //         buffer_slice.map_async(
    //             bevy::render::render_resource::MapMode::Read,
    //             move |d| match d {
    //                 Ok(_) => s.send(()).expect("Failed to send map update"),
    //                 Err(err) => println!("Something's wrong: {err}"),
    //             },
    //         );

    //         render_device
    //             .poll(bevy::render::render_resource::Maintain::wait())
    //             .panic_on_timeout();

    //         r.recv().expect("Failed to receive the map_async message");
    //         {
    //             let buffer_view = buffer_slice.get_mapped_range();

    //             let data = buffer_view
    //                 .chunks(std::mem::size_of::<u32>())
    //                 .map(|chunk| u32::from_ne_bytes(chunk.try_into().expect("should be a u32")))
    //                 .collect::<Vec<u32>>();
    //             println!("data: {}", data[0]);
    //         }

    //         svx_pipeline
    //             .readable_cache_bytes_buffer
    //             .as_ref()
    //             .unwrap()
    //             .unmap();

    //         render_data_mainworld.do_the_thing = false;
    //     }
    // }
}

pub(crate) fn sync_with_main_world(// svx_data: Option<ResMut<ShocoVoxRenderData>>,
    // svx_pipeline: Option<ResMut<ShocoVoxRenderPipeline>>,
    // mut world: ResMut<MainWorld>,
) {
}

pub(crate) fn handle_cache(
    svx_data: Option<ResMut<ShocoVoxRenderData>>,
    svx_pipeline: Option<ResMut<ShocoVoxRenderPipeline>>,
) {
    //TODO: Document that all components are lost during extract transition
    // Data updates triggered by debug interface
    // if let Some(svx_data) = svx_data {
    //     let mut render_data_mainworld = world.get_resource_mut::<ShocoVoxRenderData>().unwrap();
    //     let svx_pipeline = svx_pipeline.unwrap();
    //     let render_queue = &svx_pipeline.render_queue.0;
    //     if svx_data.do_the_thing {
    //         let mut data: [u32; 2] = [0; 2];
    //         data[0] = svx_data.children_buffer[1];
    //         data[1] = svx_data.children_buffer[0];

    //         // GPU buffer Write
    //         // render_data_mainworld.children_buffer[0] = data[0];
    //         // render_data_mainworld.children_buffer[1] = data[1];
    //         // use bevy::render::render_resource::encase::StorageBuffer;
    //         // let mut data_buffer = StorageBuffer::new(Vec::<u8>::new());
    //         // data_buffer.write(&data).unwrap();
    //         // render_queue.write_buffer(
    //         //     svx_pipeline.nodes_children_buffer.as_ref().unwrap(),
    //         //     0,
    //         //     &data_buffer.into_inner(),
    //         // );

    //         // GPU buffer read
    //         let buffer_slice = svx_pipeline.cache_bytes_buffer.as_ref().unwrap().slice(..);

    //         // .map_async(
    //         //     bevy::render::render_resource::MapMode::Read,
    //         //     move |d| match d {
    //         //         Ok(_) => println!("data!"),
    //         //         Err(err) => println!("Something's wrong kiddo"),
    //         //     },
    //         // );
    //         let data = 0;
    //         render_data_mainworld.cache_bytes[0] = data;
    //         println!("data: {}", render_data_mainworld.cache_bytes[0]);

    //         render_data_mainworld.do_the_thing = false;
    //     }
    // }
}
