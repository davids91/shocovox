use crate::object_pool::empty_marker;
use crate::octree::{
    raytracing::bevy::types::{
        OctreeGPUDataHandler, OctreeGPUHost, OctreeGPUView, OctreeMetaData, OctreeRenderData,
        OctreeSpyGlass, SvxRenderPipeline, SvxViewSet, VictimPointer, Viewport, Voxelement,
    },
    Octree, V3c, VoxelData,
};
use bevy::{
    ecs::system::{Res, ResMut},
    math::Vec4,
    prelude::{Assets, Commands, Handle, Image},
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{
            encase::{StorageBuffer, UniformBuffer},
            Extent3d, TextureDimension, TextureFormat, TextureUsages,
        },
        renderer::RenderDevice,
    },
};
use std::{
    collections::HashMap,
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
            map_to_node_index_in_metadata: HashMap::new(),
        };

        // Push root node and its contents
        gpu_data_handler.add_node(&self.tree, Octree::<T, DIM>::ROOT_NODE_KEY as usize, true);
        // +++ DEBUG +++

        //delete some random bricks from leaf nodes
        for i in 15..20 {
            if 0 != (gpu_data_handler.render_data.metadata[i] & 0x00000004) {
                gpu_data_handler.render_data.node_children[i * 8 + 3] = empty_marker();
            }
        }

        // reset used bits
        for meta in gpu_data_handler.render_data.metadata.iter_mut() {
            *meta &= 0x00FFFFFE;
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
                node_requests: vec![0; 4],
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

        // Read node requests from GPU
        let buffer_slice = resources.readable_node_requests_buffer.slice(..);
        let (s, r) = crossbeam::channel::unbounded::<()>();
        buffer_slice.map_async(
            bevy::render::render_resource::MapMode::Read,
            move |d| match d {
                Ok(_) => s.send(()).expect("Failed to send map update"),
                Err(err) => panic!("Couldn't map debug interface buffer!: {err}"),
            },
        );

        render_device
            .poll(bevy::render::render_resource::Maintain::wait())
            .panic_on_timeout();

        r.recv().expect("Failed to receive the map_async message");
        {
            let buffer_view = buffer_slice.get_mapped_range();

            svx_view_set.views[0].lock().unwrap().spyglass.node_requests = buffer_view
                .chunks(std::mem::size_of::<u32>())
                .map(|chunk| u32::from_ne_bytes(chunk.try_into().expect("should be a u32")))
                .collect::<Vec<u32>>();
        }
        resources.readable_node_requests_buffer.unmap();

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
pub(crate) fn write_to_gpu<T, const DIM: usize>(
    tree_gpu_host: Option<ResMut<OctreeGPUHost<T, DIM>>>,
    svx_pipeline: Option<ResMut<SvxRenderPipeline>>,
    svx_view_set: ResMut<SvxViewSet>,
) where
    T: Default + Clone + PartialEq + VoxelData + Send + Sync + 'static,
{
    if let Some(pipeline) = svx_pipeline {
        let render_queue = &pipeline.render_queue;
        // Data updates for spyglass viewport
        if let Some(resources) = &pipeline.resources {
            let mut buffer = UniformBuffer::new(Vec::<u8>::new());
            buffer
                .write(&svx_view_set.views[0].lock().unwrap().spyglass.viewport)
                .unwrap();
            render_queue.write_buffer(&resources.viewport_buffer, 0, &buffer.into_inner());
        }

        //TODO: Write node requests
        // let meta_buffer_updated_index_min = 0;
        // let meta_buffer_updated_index_max = 0;
        // for node_request in &mut tree_gpu_view.spyglass.node_requests {
        //     let requested_parent_node_key = *node_request & 0x00FFFFFF;
        //     let requsted_node_child_octant = (*node_request & 0xFF000000) >> 24;
        //     let requested_node_key =
        //     let data_handler = tree_gpu_view.data_handler.lock().unwrap();
        //     if let Some(updated_meta_index) = data_handler.add_node(data, node_key, false) {}
        // }
        //TODO: Write bricks
        //
        //TODO: write back updated node requests array
        //

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
