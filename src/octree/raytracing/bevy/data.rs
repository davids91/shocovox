use crate::object_pool::empty_marker;
use crate::octree::{
    raytracing::bevy::types::{
        OctreeGPUDataHandler, OctreeGPUHost, OctreeGPUView, OctreeMetaData, OctreeRenderData,
        OctreeSpyGlass, SvxRenderPipeline, SvxViewSet, VictimPointer, Viewport, Voxelement,
    },
    BrickData, NodeContent, Octree, V3c, VoxelData,
};
use bevy::{
    ecs::system::{Res, ResMut},
    math::Vec4,
    prelude::{Assets, Handle, Image},
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{
            encase::{internal::WriteInto, StorageBuffer, UniformBuffer},
            Buffer, Extent3d, ShaderSize, TextureDimension, TextureFormat, TextureUsages,
        },
        renderer::{RenderDevice, RenderQueue},
    },
};
use bimap::BiHashMap;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

impl<T, const DIM: usize> OctreeGPUHost<T, DIM>
where
    T: Default + Clone + Copy + PartialEq + VoxelData + Send + Sync + 'static,
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
    pub fn create_new_view(
        &mut self,
        svx_view_set: &mut SvxViewSet,
        size: usize,
        viewport: Viewport,
        resolution: [u32; 2],
        mut images: ResMut<Assets<Image>>,
    ) -> Handle<Image> {
        let mut gpu_data_handler = OctreeGPUDataHandler {
            render_data: OctreeRenderData {
                debug_gpu_interface: 0,
                octree_meta: OctreeMetaData {
                    octree_size: self.tree.octree_size,
                    voxel_brick_dim: DIM as u32,
                    ambient_light_color: V3c::new(1., 1., 1.),
                    ambient_light_position: V3c::new(
                        self.tree.octree_size as f32,
                        self.tree.octree_size as f32,
                        self.tree.octree_size as f32,
                    ),
                },
                metadata: vec![0; size],
                node_ocbits: vec![0; size * 2],
                node_children: vec![empty_marker(); size * 8],
                color_palette: vec![Vec4::ZERO; u16::MAX as usize],
                voxels: vec![
                    Voxelement {
                        albedo_index: 0,
                        content: 0
                    };
                    size * 8 * (DIM * DIM * DIM)
                ],
            },
            victim_node: VictimPointer::new(size),
            victim_brick: VictimPointer::new(size),
            map_to_color_index_in_palette: HashMap::new(),
            node_key_vs_meta_index: BiHashMap::new(),
            uploaded_color_palette_size: 0,
        };

        // Push root node and its contents
        gpu_data_handler.add_node(&self.tree, Octree::<T, DIM>::ROOT_NODE_KEY as usize, false);
        // +++ DEBUG +++

        //delete some random bricks from leaf nodes
        for i in 15..20 {
            if 0 != (gpu_data_handler.render_data.metadata[i] & 0x00000004) {
                gpu_data_handler.render_data.node_children[i * 8 + 3] = empty_marker();
            }
        }

        // --- DEBUG ---

        let mut output_texture = Image::new_fill(
            Extent3d {
                width: resolution[0],
                height: resolution[1],
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        );
        output_texture.texture_descriptor.usage = TextureUsages::COPY_DST
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING;
        let output_texture = images.add(output_texture);

        svx_view_set.views.push(Arc::new(Mutex::new(OctreeGPUView {
            do_the_thing: false,
            data_handler: gpu_data_handler,
            spyglass: OctreeSpyGlass {
                node_requests: vec![empty_marker(); 4],
                output_texture: output_texture.clone(),
                viewport: viewport,
            },
        })));
        output_texture
    }
}

pub(crate) fn sync_with_main_world(// tree_view: Option<ResMut<OctreeGPUView>>,
    // mut world: ResMut<bevy::render::MainWorld>,
) {
    // This function is unused because ExtractResource plugin is handling the sync
    // However, it is only one way: MainWorld --> RenderWorld
    // Any modification here is overwritten by the plugin if it is active,
    // in order to enable data flow in the opposite direction, extractresource plugin
    // needs to be disabled, and the sync logic (both ways) needs to be implemented here
    // refer to: https://www.reddit.com/r/bevy/comments/1ay50ee/copy_from_render_world_to_main_world/
    // if let Some(tree_view) = tree_view {
    //     let mut tree_view_mainworld = world.get_resource_mut::<OctreeGPUView>().unwrap();
    //     tree_view_mainworld.read_back = tree_view.read_back;
    //     println!("Read back in render world: {:?}", tree_view.read_back);
    // }
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
/// Handles data reads from GPU every loop.
/// Based on https://docs.rs/bevy/latest/src/gpu_readback/gpu_readback.rs.html
pub(crate) fn handle_gpu_readback<T, const DIM: usize>(
    render_device: Res<RenderDevice>,
    tree_gpu_host: Option<ResMut<OctreeGPUHost<T, DIM>>>,
    svx_view_set: ResMut<SvxViewSet>,
    svx_pipeline: Option<ResMut<SvxRenderPipeline>>,
) where
    T: Default + Clone + PartialEq + VoxelData + Send + Sync + 'static,
{
    if let (Some(ref mut tree_gpu_host), Some(ref mut pipeline)) = (tree_gpu_host, svx_pipeline) {
        let resources = pipeline.resources.as_ref().unwrap();

        let node_requests_buffer_slice = resources.readable_node_requests_buffer.slice(..);
        let (s, node_requests_recv) = crossbeam::channel::unbounded::<()>();
        node_requests_buffer_slice.map_async(
            bevy::render::render_resource::MapMode::Read,
            move |d| match d {
                Ok(_) => s.send(()).expect("Failed to send map update"),
                Err(err) => panic!("Couldn't map debug interface buffer!: {err}"),
            },
        );

        render_device
            .poll(bevy::render::render_resource::Maintain::wait())
            .panic_on_timeout();

        let mut view = svx_view_set.views[0].lock().unwrap();
        node_requests_recv
            .recv()
            .expect("Failed to receive the map_async message");
        {
            let buffer_view = node_requests_buffer_slice.get_mapped_range();
            view.spyglass.node_requests = buffer_view
                .chunks(std::mem::size_of::<u32>())
                .map(|chunk| u32::from_ne_bytes(chunk.try_into().expect("should be a u32")))
                .collect::<Vec<u32>>();
        }
        resources.readable_node_requests_buffer.unmap();

        if {
            let mut is_metadata_required_this_loop = false;
            for node_request in &view.spyglass.node_requests {
                if *node_request != empty_marker() {
                    is_metadata_required_this_loop = true;
                    break;
                }
            }
            is_metadata_required_this_loop
        } {
            let metadata_buffer_slice = resources.readable_metadata_buffer.slice(..);
            let (s, metadata_recv) = crossbeam::channel::unbounded::<()>();
            metadata_buffer_slice.map_async(
                bevy::render::render_resource::MapMode::Read,
                move |d| match d {
                    Ok(_) => s.send(()).expect("Failed to send map update"),
                    Err(err) => panic!("Couldn't map debug interface buffer!: {err}"),
                },
            );

            render_device
                .poll(bevy::render::render_resource::Maintain::wait())
                .panic_on_timeout();
            metadata_recv
                .recv()
                .expect("Failed to receive the map_async message");
            {
                let buffer_view = metadata_buffer_slice.get_mapped_range();
                view.data_handler.render_data.metadata = buffer_view
                    .chunks(std::mem::size_of::<u32>())
                    .map(|chunk| u32::from_ne_bytes(chunk.try_into().expect("should be a u32")))
                    .collect::<Vec<u32>>();
            }
            resources.readable_metadata_buffer.unmap();
        }

        // +++ DEBUG +++
        // Data updates triggered by debug interface
        // let data_handler = tree_gpu_view.data_handler.lock().unwrap();
        // if tree_gpu_view.do_the_thing {
        //     let buffer_slice = resources.readable_debug_gpu_interface.slice(..);
        //     let (s, r) = crossbeam::channel::unbounded::<()>();
        //     buffer_slice.map_async(
        //         bevy::render::render_resource::MapMode::Read,
        //         move |d| match d {
        //             Ok(_) => s.send(()).expect("Failed to send map update"),
        //             Err(err) => panic!("Couldn't map debug interface buffer!: {err}"),
        //         },
        //     );

        //     render_device
        //         .poll(bevy::render::render_resource::Maintain::wait())
        //         .panic_on_timeout();

        //     r.recv().expect("Failed to receive the map_async message");
        //     {
        //         let buffer_view = buffer_slice.get_mapped_range();

        //         let data = buffer_view
        //             .chunks(std::mem::size_of::<u32>())
        //             .map(|chunk| u32::from_ne_bytes(chunk.try_into().expect("should be a u32")))
        //             .collect::<Vec<u32>>();
        //         // println!("received_data: {:?}", data);
        //     }
        //     resources.readable_debug_gpu_interface.unmap();
        //     std::mem::drop(data_handler);
        //     tree_gpu_view.do_the_thing = false;
        // }
        // --- DEBUG ---
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

fn write_range_to_buffer<U>(
    array: &Vec<U>,
    range: std::ops::Range<usize>,
    buffer: &Buffer,
    render_queue: &RenderQueue,
) where
    U: Send + Sync + 'static + ShaderSize + WriteInto,
{
    if !range.is_empty() {
        let element_size = std::mem::size_of_val(&array[0]);
        let byte_offset = (range.start * element_size) as u64;
        let slice = array.get(range.clone()).expect(
            &format!(
                "Expected range {:?} to be in bounds of {:?}",
                range,
                array.len(),
            )
            .to_owned(),
        );
        let mut buf = StorageBuffer::new(Vec::<u8>::new());
        buf.write(slice).unwrap();
        render_queue.write_buffer(buffer, byte_offset, &buf.into_inner());
    }
}

pub(crate) fn write_to_gpu<T, const DIM: usize>(
    tree_gpu_host: Option<ResMut<OctreeGPUHost<T, DIM>>>,
    svx_pipeline: Option<ResMut<SvxRenderPipeline>>,
    svx_view_set: ResMut<SvxViewSet>,
) where
    T: Default + Clone + Copy + PartialEq + VoxelData + Send + Sync + 'static,
{
    if let (Some(pipeline), Some(tree_host)) = (svx_pipeline, tree_gpu_host) {
        let render_queue = &pipeline.render_queue;
        let resources = if let Some(resources) = &pipeline.resources {
            resources
        } else {
            // No resources available yet, can't write to them
            return;
        };

        let mut view = svx_view_set.views[0].lock().unwrap();

        // Data updates for spyglass viewport
        let mut buffer = UniformBuffer::new(Vec::<u8>::new());
        buffer.write(&view.spyglass.viewport).unwrap();
        render_queue.write_buffer(&resources.viewport_buffer, 0, &buffer.into_inner());

        // Handle node requests, update cache
        let tree = &tree_host.tree;
        {
            let mut meta_updated = std::ops::Range {
                start: view.data_handler.render_data.metadata.len(),
                end: 0,
            };
            let mut ocbits_updated = std::ops::Range {
                start: view.data_handler.render_data.node_ocbits.len(),
                end: 0,
            };
            let mut node_children_updated = std::ops::Range {
                start: view.data_handler.render_data.node_children.len(),
                end: 0,
            };
            let mut voxels_updated = std::ops::Range {
                start: view.data_handler.render_data.voxels.len(),
                end: 0,
            };
            let mut node_requests = view.spyglass.node_requests.clone();
            let mut updated_this_loop = HashSet::<usize>::new();
            for node_request in &mut node_requests {
                if *node_request == empty_marker() {
                    continue;
                }
                let requested_parent_meta_index = (*node_request & 0x00FFFFFF) as usize;
                let requested_child_octant = (*node_request & 0xFF000000) >> 24;

                if updated_this_loop.contains(&requested_parent_meta_index) {
                    // Do not accept a request if the requester meta is already overwritten
                    continue;
                }

                debug_assert!(view
                    .data_handler
                    .node_key_vs_meta_index
                    .contains_right(&requested_parent_meta_index));
                let requested_parent_node_key = view
                    .data_handler
                    .node_key_vs_meta_index
                    .get_by_right(&requested_parent_meta_index)
                    .unwrap();

                debug_assert!(
                    tree.nodes.key_is_valid(*requested_parent_node_key),
                    "Expected parent node({:?}) to be valid in GPU request.",
                    requested_parent_node_key
                );

                match tree.nodes.get(*requested_parent_node_key) {
                    NodeContent::Nothing => {} // parent is empty, nothing to do
                    NodeContent::Internal(_) => {

                        let requested_child_node_key = tree_host.tree.node_children
                            [*requested_parent_node_key][requested_child_octant]
                            as usize;
                        debug_assert!(
                            tree.nodes.key_is_valid(requested_child_node_key),
                            "Expected key({:?}, child of node[{:?}][{:?}] in meta[{:?}]) to be valid in GPU request.",
                            requested_child_node_key, requested_parent_node_key, requested_child_octant, requested_parent_meta_index
                        );
                        let (child_index, severed_parents) = if !view
                            .data_handler
                            .node_key_vs_meta_index
                            .contains_left(&requested_child_node_key)
                        {
                            let (child_index, severed_parents) =
                                view.data_handler
                                    .add_node(&tree, requested_child_node_key, false);
                            // println!("overwriting meta[{:?}]", child_index);
                            (child_index.expect("Expected to succeed adding a node into the GPU cache through data_handler"), severed_parents)
                        } else {
                            (
                                *view
                                    .data_handler
                                    .node_key_vs_meta_index
                                    .get_by_left(&requested_child_node_key)
                                    .unwrap(),
                                Vec::new(),
                            )
                        };

                        // Update connection to parent
                        view.data_handler.render_data.node_children
                            [requested_parent_meta_index * 8 + requested_child_octant as usize] =
                            child_index as u32;

                        debug_assert!(
                            view.data_handler
                                .node_key_vs_meta_index
                                .contains_right(&requested_parent_meta_index),
                            "Requester parent erased while adding its child node to meta"
                        );

                        // Set updated buffers range
                        meta_updated.start = meta_updated.start.min(child_index);
                        meta_updated.end = meta_updated.end.max(child_index + 1);
                        ocbits_updated.start = ocbits_updated.start.min(child_index * 2);
                        ocbits_updated.end = ocbits_updated.end.max(child_index * 2 + 2);
                        node_children_updated.start = node_children_updated
                            .start
                            .min(requested_parent_meta_index * 8);
                        node_children_updated.end = node_children_updated
                            .end
                            .max(requested_parent_meta_index * 8 + 8);
                        for severed_parent_index in severed_parents {
                            updated_this_loop.insert(severed_parent_index);
                            node_children_updated.start =
                                node_children_updated.start.min(severed_parent_index * 8);
                            node_children_updated.end =
                                node_children_updated.end.max(severed_parent_index * 8 + 8);
                        }
                    }
                    NodeContent::UniformLeaf(brick) => {
                        // Only upload brick if it's not already available
                        if matches!(brick, BrickData::Parted(_) | BrickData::Solid(_))
                            && view.data_handler.render_data.node_children
                                [requested_parent_meta_index * 8]
                                == empty_marker()
                        {
                            let (brick_index, severed_parent_index) =
                                view.data_handler.add_brick(&brick);
                            view.data_handler.render_data.node_children
                                [requested_parent_meta_index * 8] = brick_index;

                            // Set updated buffers range
                            meta_updated.start =
                                meta_updated.start.min(requested_parent_meta_index);
                            meta_updated.end =
                                meta_updated.end.max(requested_parent_meta_index + 1);
                            node_children_updated.start = node_children_updated
                                .start
                                .min((requested_parent_meta_index * 8) as usize);
                            node_children_updated.end = node_children_updated
                                .end
                                .max((requested_parent_meta_index * 8) as usize + 8);

                            if let BrickData::Parted(_) = brick {
                                voxels_updated.start = voxels_updated
                                    .start
                                    .min(brick_index as usize * (DIM * DIM * DIM));
                                voxels_updated.end = voxels_updated.end.max(
                                    brick_index as usize * (DIM * DIM * DIM) + (DIM * DIM * DIM),
                                );
                            }

                            if let Some(severed_parent_index) = severed_parent_index {
                                updated_this_loop.insert(severed_parent_index);
                                node_children_updated.start =
                                    node_children_updated.start.min(severed_parent_index * 8);
                                node_children_updated.end =
                                    node_children_updated.end.max(severed_parent_index * 8 + 8);
                            }
                        }
                    }
                    NodeContent::Leaf(bricks) => {
                        // Only upload brick if it's not already available
                        if matches!(
                            bricks[requested_child_octant as usize],
                            BrickData::Parted(_) | BrickData::Solid(_)
                        ) && view.data_handler.render_data.node_children
                            [requested_parent_meta_index * 8 + requested_child_octant as usize]
                            == empty_marker()
                        {
                            let (brick_index, severed_parent_index) = view
                                .data_handler
                                .add_brick(&bricks[requested_child_octant as usize]);
                            view.data_handler.render_data.node_children[requested_parent_meta_index
                                * 8
                                + requested_child_octant as usize] = brick_index;

                            // Set updated buffers range
                            meta_updated.start =
                                meta_updated.start.min(requested_parent_meta_index);
                            meta_updated.end =
                                meta_updated.end.max(requested_parent_meta_index + 1);
                            node_children_updated.start = node_children_updated
                                .start
                                .min((requested_parent_meta_index * 8) as usize);
                            node_children_updated.end = node_children_updated
                                .end
                                .max((requested_parent_meta_index * 8) as usize + 8);

                            if let BrickData::Parted(_) = bricks[requested_child_octant as usize] {
                                voxels_updated.start = voxels_updated
                                    .start
                                    .min(brick_index as usize * (DIM * DIM * DIM));
                                voxels_updated.end = voxels_updated.end.max(
                                    brick_index as usize * (DIM * DIM * DIM) + (DIM * DIM * DIM),
                                );
                            }
                            if let Some(severed_parent_index) = severed_parent_index {
                                updated_this_loop.insert(severed_parent_index);
                                node_children_updated.start =
                                    node_children_updated.start.min(severed_parent_index * 8);
                                node_children_updated.end =
                                    node_children_updated.end.max(severed_parent_index * 8 + 8);
                            }

                        }
                    }
                }

            }

            for node_request in &mut node_requests {
                *node_request = empty_marker();
            }

            // write back updated data
            let host_color_count = view.data_handler.map_to_color_index_in_palette.keys().len();
            let color_palette_size_diff =
                host_color_count - view.data_handler.uploaded_color_palette_size;
            let resources = &pipeline.resources.as_ref().unwrap();

            debug_assert!(
                host_color_count >= view.data_handler.uploaded_color_palette_size,
                "Expected host color palette({:?}), to be larger, than colors stored on the GPU({:?})",
                host_color_count, view.data_handler.uploaded_color_palette_size
            );
            view.data_handler.uploaded_color_palette_size =
                view.data_handler.map_to_color_index_in_palette.keys().len();

            // Node requests
            let mut buffer = StorageBuffer::new(Vec::<u8>::new());
            buffer.write(&node_requests).unwrap();
            render_queue.write_buffer(&resources.node_requests_buffer, 0, &buffer.into_inner());

            // Color palette
            if 0 < color_palette_size_diff {
                // Upload color palette delta to GPU
                write_range_to_buffer(
                    &view.data_handler.render_data.color_palette,
                    (host_color_count - color_palette_size_diff)..(host_color_count),
                    &resources.color_palette_buffer,
                    &render_queue,
                );
            }

            // Render data
            write_range_to_buffer(
                &view.data_handler.render_data.metadata,
                meta_updated,
                &resources.metadata_buffer,
                &render_queue,
            );
            write_range_to_buffer(
                &view.data_handler.render_data.node_children,
                node_children_updated,
                &resources.node_children_buffer,
                &render_queue,
            );
            write_range_to_buffer(
                &view.data_handler.render_data.node_ocbits,
                ocbits_updated,
                &resources.node_ocbits_buffer,
                &render_queue,
            );
            write_range_to_buffer(
                &view.data_handler.render_data.voxels,
                voxels_updated,
                &resources.voxels_buffer,
                &render_queue,
            );
        }

        /*// +++ DEBUG +++
        // Data updates triggered by debug interface
        if tree_gpu_host.views[0].do_the_thing {
            // GPU buffer Write
            let data_buffer = StorageBuffer::new(vec![0, 0, 0, 1]);
            render_queue.write_buffer(
                &pipeline.resources.as_ref().unwrap().debug_gpu_interface,
                0,
                &data_buffer.into_inner(),
            );
        }
        */// --- DEBUG ---
    }
}
