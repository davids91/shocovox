use crate::object_pool::empty_marker;
use crate::{
    octree::{
        raytracing::bevy::{
            create_output_texture,
            types::{
                OctreeGPUView, OctreeMetaData, ShocoVoxRenderData, ShocoVoxRenderPipeline,
                ShocoVoxViewingGlass, Viewport, Voxelement,
            },
        },
        types::{NodeChildrenArray, NodeContent},
        BrickData, Octree, V3c, VoxelData,
    },
    spatial::lut::BITMAP_MASK_FOR_OCTANT_LUT,
};
use bevy::{
    ecs::system::{Res, ResMut},
    math::Vec4,
    prelude::{Assets, Commands, Handle, Image},
    render::renderer::RenderDevice,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use super::types::OctreeGPUDataHandler;

impl<T, const DIM: usize> Octree<T, DIM>
where
    T: Default + Clone + Copy + PartialEq + VoxelData,
{
    //##############################################################################
    //     ███████      █████████  ███████████ ███████████   ██████████ ██████████
    //   ███░░░░░███   ███░░░░░███░█░░░███░░░█░░███░░░░░███ ░░███░░░░░█░░███░░░░░█
    //  ███     ░░███ ███     ░░░ ░   ░███  ░  ░███    ░███  ░███  █ ░  ░███  █ ░
    // ░███      ░███░███             ░███     ░██████████   ░██████    ░██████
    // ░███      ░███░███             ░███     ░███░░░░░███  ░███░░█    ░███░░█
    // ░░███     ███ ░░███     ███    ░███     ░███    ░███  ░███ ░   █ ░███ ░   █
    //  ░░░███████░   ░░█████████     █████    █████   █████ ██████████ ██████████
    //    ░░░░░░░      ░░░░░░░░░     ░░░░░    ░░░░░   ░░░░░ ░░░░░░░░░░ ░░░░░░░░░░
    //    █████████  ███████████  █████  █████
    //   ███░░░░░███░░███░░░░░███░░███  ░░███
    //  ███     ░░░  ░███    ░███ ░███   ░███
    // ░███          ░██████████  ░███   ░███
    // ░███    █████ ░███░░░░░░   ░███   ░███
    // ░░███  ░░███  ░███         ░███   ░███
    //  ░░█████████  █████        ░░████████
    //   ░░░░░░░░░  ░░░░░          ░░░░░░░░
    //  █████   █████ █████ ██████████ █████   ███   █████
    // ░░███   ░░███ ░░███ ░░███░░░░░█░░███   ░███  ░░███
    //  ░███    ░███  ░███  ░███  █ ░  ░███   ░███   ░███
    //  ░███    ░███  ░███  ░██████    ░███   ░███   ░███
    //  ░░███   ███   ░███  ░███░░█    ░░███  █████  ███
    //   ░░░█████░    ░███  ░███ ░   █  ░░░█████░█████░
    //     ░░███      █████ ██████████    ░░███ ░░███
    //##############################################################################

    /// Creates GPU compatible data renderable on the GPU from an octree
    pub fn create_new_gpu_view(
        &self,
        viewport: Viewport,
        resolution: [u32; 2],
        commands: &mut Commands,
        images: ResMut<Assets<Image>>,
    ) -> Handle<Image> {
        let mut gpu_data_handler = OctreeGPUDataHandler {
            debug_gpu_interface: None,
            readable_debug_gpu_interface: None,
            render_data: ShocoVoxRenderData {
                debug_gpu_interface: 0,
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
                metadata: Vec::with_capacity(self.nodes.len()),
                node_children: Vec::with_capacity(self.nodes.len() * 8),
                voxels: Vec::new(),
                node_occupied_bits: Vec::with_capacity(self.nodes.len() * 2),
                color_palette: Vec::new(),
            },
            // victim_node_pointer: 0,
            // victim_brick_pointer: 0,
            map_to_color_index_in_palette: HashMap::new(),
            map_to_node_index_in_nodes_buffer: HashMap::new(),
        };

        // Build up Nodes
        for node_key in 0..self.nodes.len() {
            if self.nodes.key_is_valid(node_key) {
                gpu_data_handler.add_node_properties(&self, node_key);
            }
        }

        // Build up node content
        for node_key in 0..self.nodes.len() {
            if self.nodes.key_is_valid(node_key) {
                gpu_data_handler.add_node_content(&self, node_key);
            }
        }

        let output_texture = create_output_texture(resolution, images);
        commands.insert_resource(OctreeGPUView {
            do_the_thing: false,
            read_back: 0,
            data_handler: Arc::new(Mutex::new(gpu_data_handler)),
            viewing_glass: ShocoVoxViewingGlass {
                output_texture: output_texture.clone(),
                viewport: viewport,
            },
        });
        output_texture
    }
}

//##############################################################################
//     ███████      █████████  ███████████ ███████████   ██████████ ██████████
//   ███░░░░░███   ███░░░░░███░█░░░███░░░█░░███░░░░░███ ░░███░░░░░█░░███░░░░░█
//  ███     ░░███ ███     ░░░ ░   ░███  ░  ░███    ░███  ░███  █ ░  ░███  █ ░
// ░███      ░███░███             ░███     ░██████████   ░██████    ░██████
// ░███      ░███░███             ░███     ░███░░░░░███  ░███░░█    ░███░░█
// ░░███     ███ ░░███     ███    ░███     ░███    ░███  ░███ ░   █ ░███ ░   █
//  ░░░███████░   ░░█████████     █████    █████   █████ ██████████ ██████████
//    ░░░░░░░      ░░░░░░░░░     ░░░░░    ░░░░░   ░░░░░ ░░░░░░░░░░ ░░░░░░░░░░
//    █████████  ███████████  █████  █████
//   ███░░░░░███░░███░░░░░███░░███  ░░███
//  ███     ░░░  ░███    ░███ ░███   ░███
// ░███          ░██████████  ░███   ░███
// ░███    █████ ░███░░░░░░   ░███   ░███
// ░░███  ░░███  ░███         ░███   ░███
//  ░░█████████  █████        ░░████████
//   ░░░░░░░░░  ░░░░░          ░░░░░░░░
//  ██████████     █████████   ███████████   █████████
// ░░███░░░░███   ███░░░░░███ ░█░░░███░░░█  ███░░░░░███
//  ░███   ░░███ ░███    ░███ ░   ░███  ░  ░███    ░███
//  ░███    ░███ ░███████████     ░███     ░███████████
//  ░███    ░███ ░███░░░░░███     ░███     ░███░░░░░███
//  ░███    ███  ░███    ░███     ░███     ░███    ░███
//  ██████████   █████   █████    █████    █████   █████
// ░░░░░░░░░░   ░░░░░   ░░░░░    ░░░░░    ░░░░░   ░░░░░
//##############################################################################

impl OctreeGPUDataHandler {
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
    fn meta_add_leaf_brick_structure<T, const DIM: usize>(
        sized_node_meta: &mut u32,
        brick: &BrickData<T, DIM>,
        brick_octant: usize,
    ) where
        T: Default + Clone + PartialEq + VoxelData,
    {
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
    fn meta_set_leaf_structure<T, const DIM: usize>(
        sized_node_meta: &mut u32,
        leaf: &NodeContent<T, DIM>,
    ) where
        T: Default + Copy + Clone + PartialEq + VoxelData,
    {
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
    fn create_node_properties<T, const DIM: usize>(node: &NodeContent<T, DIM>) -> u32
    where
        T: Default + Copy + Clone + PartialEq + VoxelData,
    {
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

    fn add_node_properties<T, const DIM: usize>(&mut self, tree: &Octree<T, DIM>, node_key: usize)
    where
        T: Default + Copy + Clone + PartialEq + VoxelData,
    {
        if tree.nodes.key_is_valid(node_key) {
            self.map_to_node_index_in_nodes_buffer
                .insert(node_key, self.render_data.metadata.len());
            self.render_data
                .metadata
                .push(Self::create_node_properties(tree.nodes.get(node_key)));
        } else {
            panic!("Trying to query invalid node key to set node metadata!");
        }
    }

    fn add_node_content<T, const DIM: usize>(&mut self, tree: &Octree<T, DIM>, node_key: usize)
    where
        T: Default + Copy + Clone + PartialEq + VoxelData,
    {
        let occupied_bits = tree.stored_occupied_bits(node_key);
        self.render_data.node_occupied_bits.extend_from_slice(&[
            (occupied_bits & 0x00000000FFFFFFFF) as u32,
            ((occupied_bits & 0xFFFFFFFF00000000) >> 32) as u32,
        ]);
        match tree.nodes.get(node_key) {
            NodeContent::UniformLeaf(brick) => {
                debug_assert!(
                    matches!(
                        tree.node_children[node_key].content,
                        NodeChildrenArray::OccupancyBitmap(_)
                    ),
                    "Expected Uniform leaf to have OccupancyBitmap(_) instead of {:?}",
                    tree.node_children[node_key].content
                );

                let (brick_index, brick_added) = self.add_brick(brick);

                self.render_data.node_children.extend_from_slice(&[
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
                            tree.node_children[node_key].content
                        {
                            debug_assert!(occupied_bits == 0 || occupied_bits == u64::MAX);
                        }
                    }
                }
            }
            NodeContent::Leaf(bricks) => {
                debug_assert!(
                    matches!(
                        tree.node_children[node_key].content,
                        NodeChildrenArray::OccupancyBitmap(_)
                    ),
                    "Expected Leaf to have OccupancyBitmaps(_) instead of {:?}",
                    tree.node_children[node_key].content
                );

                let mut children = vec![empty_marker(); 8];
                for octant in 0..8 {
                    let (brick_index, brick_added) = self.add_brick(&bricks[octant]);

                    children[octant] = brick_index;
                    #[cfg(debug_assertions)]
                    {
                        if !brick_added {
                            // If no brick was added, the relevant occupied bits should either be empty or full
                            if let NodeChildrenArray::OccupancyBitmap(occupied_bits) =
                                tree.node_children[node_key].content
                            {
                                debug_assert!(
                                    0 == occupied_bits & BITMAP_MASK_FOR_OCTANT_LUT[octant]
                                        || BITMAP_MASK_FOR_OCTANT_LUT[octant]
                                            == occupied_bits & BITMAP_MASK_FOR_OCTANT_LUT[octant]
                                );
                            }
                        }
                    }
                }
                self.render_data.node_children.extend_from_slice(&children);
            }
            NodeContent::Internal(_) => {
                for c in 0..8 {
                    let child_index = &tree.node_children[node_key][c];
                    if *child_index != empty_marker() {
                        debug_assert!(self
                            .map_to_node_index_in_nodes_buffer
                            .contains_key(&(*child_index as usize)));
                        self.render_data.node_children.push(
                            self.map_to_node_index_in_nodes_buffer[&(*child_index as usize)] as u32,
                        );
                    } else {
                        self.render_data.node_children.push(*child_index);
                    }
                }
            }
            NodeContent::Nothing => {
                self.render_data
                    .node_children
                    .extend_from_slice(&[empty_marker(); 8]);
            }
        }
        println!(
            "bricks: {:?} <> nodes: {:?}",
            self.render_data.voxels.len() / (DIM * DIM * DIM),
            self.render_data.metadata.len()
        );

        // debug_assert_eq!(
        //     self.render_data.metadata.len() * 2,
        //     self.render_data.node_occupied_bits.len(),
        //     "Node occupancy bitmaps length({:?}) should match node count({:?})!",
        //     self.render_data.node_occupied_bits.len(),
        //     self.render_data.metadata.len(),
        // );

        // debug_assert_eq!(
        //     self.render_data.metadata.len() * 8,
        //     self.render_data.node_children.len(),
        //     "Node count({:?}) should match length of children buffer({:?})!",
        //     self.render_data.metadata.len(),
        //     self.render_data.node_children.len()
        // );
    }

    /// Loads a brick into the provided voxels vector and color palette
    /// * `brick` - The brick to upload
    /// * `voxels` - The destination buffer
    /// * `color_palette` - The used color palette
    /// * `map_to_color_index_in_palette` - Indexing helper for the color palette
    /// * `returns` - the identifier to set in @SizedNode and true if a new brick was aded to the voxels vector
    fn add_brick<T, const DIM: usize>(&mut self, brick: &BrickData<T, DIM>) -> (u32, bool)
    where
        T: Default + Clone + PartialEq + VoxelData,
    {
        match brick {
            BrickData::Empty => (empty_marker(), false),
            BrickData::Solid(voxel) => {
                let albedo = voxel.albedo();
                if let std::collections::hash_map::Entry::Vacant(e) =
                    self.map_to_color_index_in_palette.entry(albedo)
                {
                    e.insert(self.render_data.color_palette.len());
                    self.render_data.color_palette.push(Vec4::new(
                        albedo.r as f32 / 255.,
                        albedo.g as f32 / 255.,
                        albedo.b as f32 / 255.,
                        albedo.a as f32 / 255.,
                    ));
                }
                (self.map_to_color_index_in_palette[&albedo] as u32, false)
            }
            BrickData::Parted(brick) => {
                self.render_data.voxels.reserve(DIM * DIM * DIM);
                let brick_index = self.render_data.voxels.len() / (DIM * DIM * DIM);
                debug_assert_eq!(
                    self.render_data.voxels.len() % (DIM * DIM * DIM),
                    0,
                    "Expected Voxel buffer length({:?}) to be divisble by {:?}",
                    self.render_data.voxels.len(),
                    (DIM * DIM * DIM)
                );
                for z in 0..DIM {
                    for y in 0..DIM {
                        for x in 0..DIM {
                            let albedo = brick[x][y][z].albedo();
                            if let std::collections::hash_map::Entry::Vacant(e) =
                                self.map_to_color_index_in_palette.entry(albedo)
                            {
                                e.insert(self.render_data.color_palette.len());
                                self.render_data.color_palette.push(Vec4::new(
                                    albedo.r as f32 / 255.,
                                    albedo.g as f32 / 255.,
                                    albedo.b as f32 / 255.,
                                    albedo.a as f32 / 255.,
                                ));
                            }
                            let albedo_index = self.map_to_color_index_in_palette[&albedo];

                            self.render_data.voxels.push(Voxelement {
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
}

pub(crate) fn sync_with_main_world(
    tree_view: Option<ResMut<OctreeGPUView>>,
    mut world: ResMut<bevy::render::MainWorld>,
) {
    if let Some(tree_view) = tree_view {
        let mut tree_view_mainworld = world.get_resource_mut::<OctreeGPUView>().unwrap();
        tree_view_mainworld.read_back = tree_view.read_back;
    }
}

//##############################################################################
//    █████████  ███████████  █████  █████
//   ███░░░░░███░░███░░░░░███░░███  ░░███
//  ███     ░░░  ░███    ░███ ░███   ░███
// ░███          ░██████████  ░███   ░███
// ░███    █████ ░███░░░░░░   ░███   ░███
// ░░███  ░░███  ░███         ░███   ░███
//  ░░█████████  █████        ░░████████
//   ░░░░░░░░░  ░░░░░          ░░░░░░░░
//  ███████████   ██████████   █████████   ██████████
// ░░███░░░░░███ ░░███░░░░░█  ███░░░░░███ ░░███░░░░███
//  ░███    ░███  ░███  █ ░  ░███    ░███  ░███   ░░███
//  ░██████████   ░██████    ░███████████  ░███    ░███
//  ░███░░░░░███  ░███░░█    ░███░░░░░███  ░███    ░███
//  ░███    ░███  ░███ ░   █ ░███    ░███  ░███    ███
//  █████   █████ ██████████ █████   █████ ██████████
// ░░░░░   ░░░░░ ░░░░░░░░░░ ░░░░░   ░░░░░ ░░░░░░░░░░
//##############################################################################
pub(crate) fn handle_gpu_readback(
    render_device: Res<RenderDevice>,
    tree_gpu_view: Option<ResMut<OctreeGPUView>>,
) {
    // Data updates triggered by debug interface
    if let Some(mut tree_gpu_view) = tree_gpu_view {
        if tree_gpu_view.do_the_thing {
            let received_data;
            {
                let data_handler = tree_gpu_view.data_handler.lock().unwrap();
                // GPU buffer read
                // https://docs.rs/bevy/latest/src/gpu_readback/gpu_readback.rs.html
                let buffer_slice = data_handler
                    .readable_debug_gpu_interface
                    .as_ref()
                    .unwrap()
                    .slice(..);
                let (s, r) = crossbeam::channel::unbounded::<()>();
                buffer_slice.map_async(bevy::render::render_resource::MapMode::Read, move |d| {
                    match d {
                        Ok(_) => s.send(()).expect("Failed to send map update"),
                        Err(err) => println!("Something's wrong: {err}"),
                    }
                });

                render_device
                    .poll(bevy::render::render_resource::Maintain::wait())
                    .panic_on_timeout();

                r.recv().expect("Failed to receive the map_async message");
                {
                    let buffer_view = buffer_slice.get_mapped_range();

                    let data = buffer_view
                        .chunks(std::mem::size_of::<u32>())
                        .map(|chunk| u32::from_ne_bytes(chunk.try_into().expect("should be a u32")))
                        .collect::<Vec<u32>>();
                    received_data = data[0];
                }
                data_handler
                    .readable_debug_gpu_interface
                    .as_ref()
                    .unwrap()
                    .unmap();
            }
            tree_gpu_view.read_back = received_data;
            tree_gpu_view.do_the_thing = false;
        }
    }
}

//##############################################################################
//    █████████  ███████████  █████  █████
//   ███░░░░░███░░███░░░░░███░░███  ░░███
//  ███     ░░░  ░███    ░███ ░███   ░███
// ░███          ░██████████  ░███   ░███
// ░███    █████ ░███░░░░░░   ░███   ░███
// ░░███  ░░███  ░███         ░███   ░███
//  ░░█████████  █████        ░░████████
//   ░░░░░░░░░  ░░░░░          ░░░░░░░░

//  █████   ███   █████ ███████████   █████ ███████████ ██████████
// ░░███   ░███  ░░███ ░░███░░░░░███ ░░███ ░█░░░███░░░█░░███░░░░░█
//  ░███   ░███   ░███  ░███    ░███  ░███ ░   ░███  ░  ░███  █ ░
//  ░███   ░███   ░███  ░██████████   ░███     ░███     ░██████
//  ░░███  █████  ███   ░███░░░░░███  ░███     ░███     ░███░░█
//   ░░░█████░█████░    ░███    ░███  ░███     ░███     ░███ ░   █
//     ░░███ ░░███      █████   █████ █████    █████    ██████████
//      ░░░   ░░░      ░░░░░   ░░░░░ ░░░░░    ░░░░░    ░░░░░░░░░░
//##############################################################################
pub(crate) fn write_to_gpu(
    tree_gpu_view: Option<ResMut<OctreeGPUView>>,
    svx_pipeline: Option<ResMut<ShocoVoxRenderPipeline>>,
) {
    //TODO: Document that all components are lost during extract transition
    // Data updates triggered by debug interface
    if let Some(tree_gpu_view) = tree_gpu_view {
        let svx_pipeline = svx_pipeline.unwrap();
        let render_queue = &svx_pipeline.render_queue.0;
        if tree_gpu_view.do_the_thing {
            // GPU buffer Write
            use bevy::render::render_resource::encase::StorageBuffer;
            let data_buffer = StorageBuffer::new(vec![0, 0, 0, 1]);
            render_queue.write_buffer(
                tree_gpu_view
                    .data_handler
                    .lock()
                    .unwrap()
                    .debug_gpu_interface
                    .as_ref()
                    .unwrap(),
                0,
                &data_buffer.into_inner(),
            );
        }
    }
}
