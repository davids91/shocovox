use crate::object_pool::empty_marker;
use crate::octree::{
    raytracing::bevy::{
        create_output_texture,
        types::{
            OctreeGPUView, OctreeMetaData, SvxRenderData, SvxRenderPipeline, SvxViewingGlass,
            Viewport, Voxelement,
        },
    },
    Octree, V3c, VoxelData,
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

use super::types::{OctreeGPUDataHandler, VictimPointer};

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
        size: usize,
        viewport: Viewport,
        resolution: [u32; 2],
        commands: &mut Commands,
        images: ResMut<Assets<Image>>,
    ) -> Handle<Image> {
        let mut gpu_data_handler = OctreeGPUDataHandler {
            debug_gpu_interface: None,
            readable_debug_gpu_interface: None,
            render_data: SvxRenderData {
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
        gpu_data_handler.add_node(&self, Self::ROOT_NODE_KEY as usize, true);
        // +++ DEBUG +++

        //Push additional nodes to try to overwrite existing ones
        /*for node_key in 10..15 {
            gpu_data_handler.add_node(&self, node_key, false);
        }*/

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

        let output_texture = create_output_texture(resolution, images);
        commands.insert_resource(OctreeGPUView {
            do_the_thing: false,
            read_back: 0,
            data_handler: Arc::new(Mutex::new(gpu_data_handler)),
            viewing_glass: SvxViewingGlass {
                output_texture: output_texture.clone(),
                viewport: viewport,
            },
        });
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
pub(crate) fn handle_gpu_readback(
    render_device: Res<RenderDevice>,
    mut tree_gpu_view: Option<ResMut<OctreeGPUView>>,
) {
    // Data updates triggered by debug interface
    if let Some(ref mut tree_gpu_view) = tree_gpu_view {
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
                        Err(err) => panic!("Couldn't map debug interface buffer!: {err}"),
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
                    // println!("received_data: {:?}", data);
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
    svx_pipeline: Option<ResMut<SvxRenderPipeline>>,
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
