use crate::object_pool::empty_marker;
use crate::octree::{
    raytracing::bevy::types::{
        BrickOwnedBy, BrickUpdate, OctreeGPUDataHandler, OctreeGPUHost, OctreeGPUView,
        OctreeMetaData, OctreeRenderData, OctreeSpyGlass, SvxRenderPipeline, SvxViewSet,
        VictimPointer, Viewport,
    },
    types::PaletteIndexValues,
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
    hash::Hash,
    sync::{Arc, Mutex},
};

impl<T> OctreeGPUHost<T>
where
    T: Default + Clone + Copy + Eq + Send + Sync + Hash + VoxelData + 'static,
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
                octree_meta: OctreeMetaData {
                    octree_size: self.tree.octree_size,
                    voxel_brick_dim: self.tree.brick_dim,
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
            },
            victim_node: VictimPointer::new(size),
            victim_brick: 0,
            map_to_brick_maybe_owned_by_node: HashMap::new(),
            node_key_vs_meta_index: BiHashMap::new(),
            brick_ownership: vec![BrickOwnedBy::NotOwned; size * 8],
            uploaded_color_palette_size: 0,
        };

        gpu_data_handler.add_node(&self.tree, Octree::<T>::ROOT_NODE_KEY as usize);

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
            data_handler: gpu_data_handler,
            spyglass: OctreeSpyGlass {
                node_requests: vec![empty_marker(); 4],
                output_texture: output_texture.clone(),
                viewport,
            },
        })));
        output_texture
    }
}

/// Handles data sync between Bevy main(CPU) world and rendering world
pub(crate) fn sync_with_main_world(// tree_view: Option<ResMut<OctreeGPUView>>,
    // mut world: ResMut<bevy::render::MainWorld>,
) {
    // This function is unused because ExtractResource plugin is handling the sync
    // However, it is only one way: MainWorld --> RenderWorld
    // Any modification here is overwritten by the plugin if it is active,
    // in order to enable data flow in the opposite direction, extractresource plugin
    // needs to be disabled, and the sync logic (both ways) needs to be implemented here
    // refer to: https://www.reddit.com/r/bevy/comments/1ay50ee/copy_from_render_world_to_main_world/
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
/// Handles data reads from GPU every loop, mainly data requests and usaage updates.
/// Based on https://docs.rs/bevy/latest/src/gpu_readback/gpu_readback.rs.html
pub(crate) fn handle_gpu_readback(
    render_device: Res<RenderDevice>,
    svx_view_set: ResMut<SvxViewSet>,
    mut svx_pipeline: Option<ResMut<SvxRenderPipeline>>,
) {
    if let Some(ref mut pipeline) = svx_pipeline {
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

        let is_metadata_required_this_loop = {
            let mut is_metadata_required_this_loop = false;
            for node_request in &view.spyglass.node_requests {
                if *node_request != empty_marker::<u32>() {
                    is_metadata_required_this_loop = true;
                    break;
                }
            }
            is_metadata_required_this_loop
        };
        if is_metadata_required_this_loop {
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

/// Converts the given array to `&[u8]` on the given range,
/// and schedules it to be written to the given buffer in the GPU
fn write_range_to_buffer<U>(
    array: &[U],
    index_range: std::ops::Range<usize>,
    buffer: &Buffer,
    render_queue: &RenderQueue,
) where
    U: Send + Sync + 'static + ShaderSize + WriteInto,
{
    if !index_range.is_empty() {
        let element_size = std::mem::size_of_val(&array[0]);
        let byte_offset = (index_range.start * element_size) as u64;
        let slice = array.get(index_range.clone()).unwrap_or_else(|| {
            panic!(
                "{}",
                format!(
                    "Expected range {:?} to be in bounds of {:?}",
                    index_range,
                    array.len(),
                )
                .to_owned()
            )
        });
        unsafe {
            render_queue.write_buffer(buffer, byte_offset, slice.align_to::<u8>().1);
        }
    }
}

/// Extend the given HashMap with a list of brick updates, except avoid overwriting available data with None value
fn extend_brick_updates<'a>(
    brick_updates: &mut HashMap<usize, Option<&'a [PaletteIndexValues]>>,
    addition: Vec<BrickUpdate<'a>>,
) {
    for brick_update in addition {
        brick_updates
            .entry(brick_update.brick_index)
            .and_modify(|current_brick_data| {
                match (*current_brick_data, brick_update.data) {
                    (None, None) | (Some(_), None) => {} // Keep the current brick data if there is already something
                    (Some(_), Some(_)) | (None, Some(_)) => {
                        // Overwrite the current brick request
                        *current_brick_data = brick_update.data
                    }
                }
            })
            .or_insert(brick_update.data);
    }
}

/// Handles Data Streaming to the GPU based on incoming requests from the view(s)
pub(crate) fn write_to_gpu<T>(
    tree_gpu_host: Option<ResMut<OctreeGPUHost<T>>>,
    svx_pipeline: Option<ResMut<SvxRenderPipeline>>,
    svx_view_set: ResMut<SvxViewSet>,
) where
    T: Default + Clone + Copy + Eq + Send + Sync + Hash + VoxelData + 'static,
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
            let mut node_requests = view.spyglass.node_requests.clone();
            let mut modified_nodes = HashSet::<usize>::new();
            let mut modified_bricks = HashMap::<usize, Option<&[PaletteIndexValues]>>::new();
            let victim_node_loop_count = view.data_handler.victim_node.get_loop_count();
            for node_request in &mut node_requests {
                if *node_request == empty_marker::<u32>() {
                    continue;
                }
                let requested_parent_meta_index = (*node_request & 0x00FFFFFF) as usize;
                let requested_child_octant = (*node_request & 0xFF000000) >> 24;

                if modified_nodes.contains(&requested_parent_meta_index) {
                    // Do not accept a request if the requester meta is already overwritten
                    continue;
                }

                debug_assert!(view
                    .data_handler
                    .node_key_vs_meta_index
                    .contains_right(&requested_parent_meta_index));

                let requested_parent_node_key = *view
                    .data_handler
                    .node_key_vs_meta_index
                    .get_by_right(&requested_parent_meta_index)
                    .unwrap();

                debug_assert!(
                    tree.nodes.key_is_valid(requested_parent_node_key),
                    "Expected parent node({:?}) to be valid in GPU request.",
                    requested_parent_node_key
                );

                modified_nodes.insert(requested_parent_meta_index);
                ocbits_updated.start = ocbits_updated.start.min(requested_parent_meta_index * 2);
                ocbits_updated.end = ocbits_updated.end.max(requested_parent_meta_index * 2 + 2);
                match tree.nodes.get(requested_parent_node_key) {
                    NodeContent::Nothing => {} // parent is empty, nothing to do
                    NodeContent::Internal(_) => {
                        let requested_child_node_key = tree_host.tree.node_children
                            [requested_parent_node_key][requested_child_octant]
                            as usize;
                        debug_assert!(
                            tree.nodes.key_is_valid(requested_child_node_key),
                            "Expected key({:?}, child of node[{:?}][{:?}] in meta[{:?}]) to be valid in GPU request.",
                            requested_child_node_key, requested_parent_node_key, requested_child_octant, requested_parent_meta_index
                        );
                        let child_index = if !view
                            .data_handler
                            .node_key_vs_meta_index
                            .contains_left(&requested_child_node_key)
                        {
                            let (child_index, currently_modified_bricks, currently_modified_nodes) =
                                view.data_handler
                                .add_node(tree, requested_child_node_key)
                                .expect("Expected to succeed adding a node into the GPU cache through data_handler");
                            modified_nodes.extend(currently_modified_nodes);
                            extend_brick_updates(&mut modified_bricks, currently_modified_bricks);

                            child_index
                        } else {
                            *view
                                .data_handler
                                .node_key_vs_meta_index
                                .get_by_left(&requested_child_node_key)
                                .unwrap()
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

                        ocbits_updated.start = ocbits_updated.start.min(child_index * 2);
                        ocbits_updated.end = ocbits_updated.end.max(child_index * 2 + 2);
                    }
                    NodeContent::UniformLeaf(brick) => {
                        // Only upload brick if it's not already available
                        if matches!(brick, BrickData::Parted(_) | BrickData::Solid(_))
                            && view.data_handler.render_data.node_children
                                [requested_parent_meta_index * 8]
                                == empty_marker::<u32>()
                        {
                            let (brick_index, currently_modified_bricks, currently_modified_nodes) =
                                view.data_handler
                                    .add_brick(tree, requested_parent_node_key, 0);
                            view.data_handler.render_data.node_children
                                [requested_parent_meta_index * 8] = brick_index as u32;

                            modified_nodes.extend(currently_modified_nodes);
                            extend_brick_updates(&mut modified_bricks, currently_modified_bricks);
                        }
                    }
                    NodeContent::Leaf(bricks) => {
                        // Only upload brick if it's not empty and not already available
                        if matches!(
                            bricks[requested_child_octant as usize],
                            BrickData::Parted(_) | BrickData::Solid(_)
                        ) && view.data_handler.render_data.node_children
                            [requested_parent_meta_index * 8 + requested_child_octant as usize]
                            == empty_marker::<u32>()
                        {
                            let (brick_index, currently_modified_bricks, currently_modified_nodes) =
                                view.data_handler.add_brick(
                                    tree,
                                    requested_parent_node_key,
                                    requested_child_octant as usize,
                                );
                            view.data_handler.render_data.node_children[requested_parent_meta_index
                                * 8
                                + requested_child_octant as usize] = brick_index as u32;

                            modified_nodes.extend(currently_modified_nodes);
                            extend_brick_updates(&mut modified_bricks, currently_modified_bricks);
                        }
                    }
                }

                if victim_node_loop_count != view.data_handler.victim_node.get_loop_count() {
                    break;
                }
            }

            debug_assert!(
                // Either all node requests are empty
                node_requests
                    .iter()
                    .filter(|&v| *v == empty_marker::<u32>())
                    .count()
                    == node_requests.len()
                    // Or some nodes were updated this loop
                    || !modified_nodes.is_empty()
                    // Or the distance traveled by the victim pointer this loop is small enough
                    || (view.data_handler.victim_node.len() as f32 * 0.5) as usize
                        > (victim_node_loop_count - view.data_handler.victim_node.get_loop_count()),
                "Couldn't process a single request because size of the buffer is too small."
            );

            for node_request in &mut node_requests {
                *node_request = empty_marker();
            }

            // Set updated buffers range based on modified nodes and bricks
            for modified_node_index in &modified_nodes {
                meta_updated.start = meta_updated.start.min(*modified_node_index);
                meta_updated.end = meta_updated.end.max(modified_node_index + 1);
                node_children_updated.start =
                    node_children_updated.start.min(modified_node_index * 8);
                node_children_updated.end =
                    node_children_updated.end.max(modified_node_index * 8 + 8);
            }

            for modified_brick_data in &modified_bricks {
                meta_updated.start = meta_updated.start.min(modified_brick_data.0 / 8);
                meta_updated.end = meta_updated.end.max(modified_brick_data.0 / 8 + 1);
            }

            // write back updated data
            let host_color_count = tree.map_to_color_index_in_palette.keys().len();
            let color_palette_size_diff =
                host_color_count - view.data_handler.uploaded_color_palette_size;
            let resources = &pipeline.resources.as_ref().unwrap();

            debug_assert!(
                host_color_count >= view.data_handler.uploaded_color_palette_size,
                "Expected host color palette({:?}), to be larger, than colors stored on the GPU({:?})",
                host_color_count, view.data_handler.uploaded_color_palette_size
            );

            // Node requests
            let mut buffer = StorageBuffer::new(Vec::<u8>::new());
            buffer.write(&node_requests).unwrap();
            render_queue.write_buffer(&resources.node_requests_buffer, 0, &buffer.into_inner());

            // Color palette
            if 0 < color_palette_size_diff {
                for i in view.data_handler.uploaded_color_palette_size..host_color_count {
                    view.data_handler.render_data.color_palette[i] =
                        tree.voxel_color_palette[i].into();
                }

                // Upload color palette delta to GPU
                write_range_to_buffer(
                    &view.data_handler.render_data.color_palette,
                    (host_color_count - color_palette_size_diff)..(host_color_count),
                    &resources.color_palette_buffer,
                    render_queue,
                );
            }
            view.data_handler.uploaded_color_palette_size =
                tree.map_to_color_index_in_palette.keys().len();

            // Render data
            write_range_to_buffer(
                &view.data_handler.render_data.metadata,
                meta_updated,
                &resources.metadata_buffer,
                render_queue,
            );
            write_range_to_buffer(
                &view.data_handler.render_data.node_children,
                node_children_updated,
                &resources.node_children_buffer,
                render_queue,
            );
            write_range_to_buffer(
                &view.data_handler.render_data.node_ocbits,
                ocbits_updated,
                &resources.node_ocbits_buffer,
                render_queue,
            );

            // Upload Voxel data
            for modified_brick_data in &modified_bricks {
                if let Some(new_brick_data) = modified_brick_data.1 {
                    let brick_start_index = *modified_brick_data.0 * new_brick_data.len();
                    debug_assert_eq!(
                        new_brick_data.len(),
                        (tree.brick_dim * tree.brick_dim * tree.brick_dim) as usize,
                        "Expected Brick slice to align to tree brick dimension"
                    );
                    unsafe {
                        render_queue.write_buffer(
                            &resources.voxels_buffer,
                            (brick_start_index * std::mem::size_of_val(&new_brick_data[0])) as u64,
                            new_brick_data.align_to::<u8>().1,
                        );
                    }
                }
            }
        }
    }
}
