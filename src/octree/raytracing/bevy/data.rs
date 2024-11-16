use crate::object_pool::empty_marker;
use crate::{
    octree::{
        raytracing::bevy::types::{
            OctreeMetaData, ShocoVoxRenderData, ShocoVoxRenderPipeline, Voxelement,
        },
        types::{NodeChildrenArray, NodeContent},
        Albedo, BrickData, Octree, V3c, VoxelData,
    },
    spatial::lut::BITMAP_MASK_FOR_OCTANT_LUT,
};
use bevy::{
    ecs::system::{Res, ResMut},
    math::Vec4,
    render::renderer::RenderDevice,
};
use std::collections::HashMap;

impl<T, const DIM: usize> Octree<T, DIM>
where
    T: Default + Clone + Copy + PartialEq + VoxelData,
{
    /// Updates metadata to set every element it might contain unused
    fn meta_set_element_unused(sized_node_meta: &mut u32) {
        *sized_node_meta = *sized_node_meta & 0x00FFFFFC;
    }

    /// Updates the meta element value to store that the corresponding node is a leaf node
    fn meta_set_is_leaf(sized_node_meta: &mut u32, is_leaf: bool) {
        *sized_node_meta =
            (*sized_node_meta & 0xFFFFFFFB) | if is_leaf { 0x00000004 } else { 0x00000000 };
    }

    /// Updates the meta element value to store that the corresponding node is a uniform leaf node
    fn meta_set_is_uniform(sized_node_meta: &mut u32, is_uniform: bool) {
        *sized_node_meta =
            (*sized_node_meta & 0xFFFFFFF7) | if is_uniform { 0x00000008 } else { 0x00000000 };
    }

    /// Updates the meta element value to store the brick structure of the given leaf node.
    /// Does not erase anything in @sized_node_meta, it's expected to be cleared before
    /// the first use of this function
    /// for the given brick octant
    /// * `sized_node_meta` - the bytes to update
    /// * `brick` - the brick to describe into the bytes
    /// * `brick_octant` - the octant to update in the bytes
    fn meta_add_leaf_brick_structure(
        sized_node_meta: &mut u32,
        brick: &BrickData<T, DIM>,
        brick_octant: usize,
    ) {
        match brick {
            BrickData::Empty => {} // Child structure properties already set to NIL
            BrickData::Solid(_voxel) => {
                // set child Occupied bits, child Structure bits already set to NIL
                *sized_node_meta |= 0x01 << (8 + brick_octant);
            }
            BrickData::Parted(_brick) => {
                // set child Occupied bits
                *sized_node_meta |= 0x01 << (8 + brick_octant);

                // set child Structure bits
                *sized_node_meta |= 0x01 << (16 + brick_octant);
            }
        };
    }

    /// Updates the given meta element value to store the leaf structure of the given node
    /// the given NodeContent reference is expected to be a leaf node
    fn meta_set_leaf_structure(sized_node_meta: &mut u32, leaf: &NodeContent<T, DIM>) {
        match leaf {
            NodeContent::UniformLeaf(brick) => {
                Self::meta_set_is_leaf(sized_node_meta, true);
                Self::meta_set_is_uniform(sized_node_meta, true);
                Self::meta_add_leaf_brick_structure(sized_node_meta, brick, 0);
            }
            NodeContent::Leaf(bricks) => {
                Self::meta_set_is_leaf(sized_node_meta, true);
                Self::meta_set_is_uniform(sized_node_meta, false);
                for octant in 0..8 {
                    Self::meta_add_leaf_brick_structure(sized_node_meta, &bricks[octant], octant);
                }
            }
            NodeContent::Internal(_) | NodeContent::Nothing => {
                panic!("Expected node content to be of a leaf");
            }
        }
    }

    /// Creates the descriptor bytes for the given node
    fn create_node_properties(node: &NodeContent<T, DIM>) -> u32 {
        let mut meta = 0;
        Self::meta_set_element_unused(&mut meta);
        match node {
            NodeContent::Leaf(_) | NodeContent::UniformLeaf(_) => {
                Self::meta_set_is_leaf(&mut meta, true);
                Self::meta_set_leaf_structure(&mut meta, node);
            }
            NodeContent::Internal(_) | NodeContent::Nothing => {
                Self::meta_set_is_leaf(&mut meta, false);
            }
        };
        meta
    }

    /// Loads a brick into the provided voxels vector and color palette
    /// * `brick` - The brick to upload
    /// * `voxels` - The destination buffer
    /// * `color_palette` - The used color palette
    /// * `map_to_color_index_in_palette` - Indexing helper for the color palette
    /// * `returns` - the identifier to set in @SizedNode and true if a new brick was aded to the voxels vector
    fn add_brick_to_vec(
        brick: &BrickData<T, DIM>,
        voxels: &mut Vec<Voxelement>,
        color_palette: &mut Vec<Vec4>,
        map_to_color_index_in_palette: &mut HashMap<Albedo, usize>,
    ) -> (u32, bool) {
        match brick {
            BrickData::Empty => (empty_marker(), false),
            BrickData::Solid(voxel) => {
                let albedo = voxel.albedo();
                if let std::collections::hash_map::Entry::Vacant(e) =
                    map_to_color_index_in_palette.entry(albedo)
                {
                    e.insert(color_palette.len());
                    color_palette.push(Vec4::new(
                        albedo.r as f32 / 255.,
                        albedo.g as f32 / 255.,
                        albedo.b as f32 / 255.,
                        albedo.a as f32 / 255.,
                    ));
                }
                (map_to_color_index_in_palette[&albedo] as u32, false)
            }
            BrickData::Parted(brick) => {
                voxels.reserve(DIM * DIM * DIM);
                let brick_index = voxels.len() / (DIM * DIM * DIM);
                debug_assert_eq!(
                    voxels.len() % (DIM * DIM * DIM),
                    0,
                    "Expected Voxel buffer length({:?}) to be divisble by {:?}",
                    voxels.len(),
                    (DIM * DIM * DIM)
                );
                for z in 0..DIM {
                    for y in 0..DIM {
                        for x in 0..DIM {
                            let albedo = brick[x][y][z].albedo();
                            if let std::collections::hash_map::Entry::Vacant(e) =
                                map_to_color_index_in_palette.entry(albedo)
                            {
                                e.insert(color_palette.len());
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
                                content: brick[x][y][z].user_data(),
                            });
                        }
                    }
                }
                (brick_index as u32, true)
            }
        }
    }

    /// Creates GPU compatible data renderable on the GPU from an octree
    pub fn create_bevy_view(&self) -> ShocoVoxRenderData {
        let mut nodes = Vec::with_capacity(self.nodes.len());
        let mut voxels = Vec::new();
        let mut node_occupied_bits = Vec::new();
        let mut color_palette = Vec::new();
        // Size of meta for one element is 2 Bytes, so array should be half + 1 for odd numbers
        let mut node_children = Vec::with_capacity(self.nodes.len() * 8);

        // Build up Nodes
        let mut map_to_node_index_in_nodes_buffer = HashMap::new();
        for i in 0..self.nodes.len() {
            if self.nodes.key_is_valid(i) {
                map_to_node_index_in_nodes_buffer.insert(i, nodes.len());
                nodes.push(Self::create_node_properties(self.nodes.get(i)));
            }
        }

        // Build up voxel content
        let mut map_to_color_index_in_palette = HashMap::new();
        for i in 0..self.nodes.len() {
            if !self.nodes.key_is_valid(i) {
                continue;
            }
            let occupied_bits = self.stored_occupied_bits(i);
            node_occupied_bits.extend_from_slice(&[
                (occupied_bits & 0x00000000FFFFFFFF) as u32,
                ((occupied_bits & 0xFFFFFFFF00000000) >> 32) as u32,
            ]);
            match self.nodes.get(i) {
                NodeContent::UniformLeaf(brick) => {
                    debug_assert!(
                        matches!(
                            self.node_children[i].content,
                            NodeChildrenArray::OccupancyBitmap(_)
                        ),
                        "Expected Uniform leaf to have OccupancyBitmap(_) instead of {:?}",
                        self.node_children[i].content
                    );

                    let (brick_index, brick_added) = Self::add_brick_to_vec(
                        brick,
                        &mut voxels,
                        &mut color_palette,
                        &mut map_to_color_index_in_palette,
                    );

                    node_children.extend_from_slice(&[
                        brick_index,
                        empty_marker(),
                        empty_marker(),
                        empty_marker(),
                        empty_marker(),
                        empty_marker(),
                        empty_marker(),
                        empty_marker(),
                    ]);
                    #[cfg(debug_assertions)]
                    {
                        if !brick_added {
                            // If no brick was added, the occupied bits should either be empty or full
                            if let NodeChildrenArray::OccupancyBitmap(occupied_bits) =
                                self.node_children[i].content
                            {
                                debug_assert!(occupied_bits == 0 || occupied_bits == u64::MAX);
                            }
                        }
                    }
                }
                NodeContent::Leaf(bricks) => {
                    debug_assert!(
                        matches!(
                            self.node_children[i].content,
                            NodeChildrenArray::OccupancyBitmap(_)
                        ),
                        "Expected Leaf to have OccupancyBitmaps(_) instead of {:?}",
                        self.node_children[i].content
                    );

                    let mut children = vec![empty_marker(); 8];
                    for octant in 0..8 {
                        let (brick_index, brick_added) = Self::add_brick_to_vec(
                            &bricks[octant],
                            &mut voxels,
                            &mut color_palette,
                            &mut map_to_color_index_in_palette,
                        );

                        children[octant] = brick_index;
                        #[cfg(debug_assertions)]
                        {
                            if !brick_added {
                                // If no brick was added, the relevant occupied bits should either be empty or full
                                if let NodeChildrenArray::OccupancyBitmap(occupied_bits) =
                                    self.node_children[i].content
                                {
                                    debug_assert!(
                                        0 == occupied_bits & BITMAP_MASK_FOR_OCTANT_LUT[octant]
                                            || BITMAP_MASK_FOR_OCTANT_LUT[octant]
                                                == occupied_bits
                                                    & BITMAP_MASK_FOR_OCTANT_LUT[octant]
                                    );
                                }
                            }
                        }
                    }
                    node_children.extend_from_slice(&children);
                }
                NodeContent::Internal(_) => {
                    for c in 0..8 {
                        let child_index = &self.node_children[i][c];
                        if *child_index != empty_marker() {
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
                NodeContent::Nothing => {
                    node_children.extend_from_slice(&[empty_marker(); 8]);
                }
            }
        }

        // +++ DEBUG +++
        println!(
            "bricks: {:?} <> nodes: {:?}",
            voxels.len() / (DIM * DIM * DIM),
            nodes.len()
        );
        nodes.push(0); // Additional element in node array for debug purposes
                       /*debug_assert_eq!(
                           nodes.len() * 2,
                           node_occupied_bits.len(),
                           "Node occupancy bitmaps length({:?}) should match node count({:?})!",
                           node_occupied_bits.len(),
                           nodes.len(),
                       );

                       debug_assert_eq!(
                           nodes.len() * 8,
                           node_children.len(),
                           "Node count({:?}) should match length of children buffer({:?})!",
                           nodes.len(),
                           node_children.len()
                       );*/
        // --- DEBUG ---
        ShocoVoxRenderData {
            do_the_thing: false,
            octree_meta: OctreeMetaData {
                octree_size: self.octree_size,
                voxel_brick_dim: DIM as u32,
                ambient_light_color: V3c::new(1., 1., 1.),
                ambient_light_position: V3c::new(
                    self.octree_size as f32,
                    self.octree_size as f32,
                    self.octree_size as f32,
                ),
            },
            metadata: nodes,
            node_children,
            voxels,
            node_occupied_bits,
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
    // if let Some(mut svx_data) = svx_data {
    //     let svx_pipeline = svx_pipeline.unwrap();
    //     if svx_data.do_the_thing {
    //         // GPU buffer read
    //         // https://docs.rs/bevy/latest/src/gpu_readback/gpu_readback.rs.html
    //         let buffer_slice = svx_pipeline
    //             .readable_nodes_buffer
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

    //         svx_pipeline.readable_nodes_buffer.as_ref().unwrap().unmap();

    //         svx_data.do_the_thing = false;
    //     }
    // }
}

pub(crate) fn sync_with_main_world(// svx_data: Option<ResMut<ShocoVoxRenderData>>,
    // svx_pipeline: Option<ResMut<ShocoVoxRenderPipeline>>,
    // mut world: ResMut<MainWorld>,
) {
    // let mut render_data_mainworld = world.get_resource_mut::<ShocoVoxRenderData>().unwrap();
}

pub(crate) fn handle_cache(
    svx_data: Option<ResMut<ShocoVoxRenderData>>,
    svx_pipeline: Option<ResMut<ShocoVoxRenderPipeline>>,
) {
    //TODO: Document that all components are lost during extract transition
    // Data updates triggered by debug interface
    if let Some(mut svx_data) = svx_data {
        let svx_pipeline = svx_pipeline.unwrap();
        let render_queue = &svx_pipeline.render_queue.0;
        if svx_data.do_the_thing {
            // GPU buffer Write
            let data: u32 = 1;
            use bevy::render::render_resource::encase::StorageBuffer;
            let mut data_buffer = StorageBuffer::new(Vec::<u8>::new());
            data_buffer.write(&data).unwrap();
            render_queue.write_buffer(
                svx_pipeline.metadata_buffer.as_ref().unwrap(),
                ((svx_data.metadata.len() - 1) * std::mem::size_of::<u32>()) as u64,
                &data_buffer.into_inner(),
            );

            svx_data.do_the_thing = false;
        }
    }
}
