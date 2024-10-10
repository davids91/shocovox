mod data;
pub mod types;

pub use crate::octree::raytracing::bevy::types::{
    ShocoVoxRenderPlugin, ShocoVoxViewingGlass, Viewport,
};

use crate::octree::raytracing::bevy::types::{
    ShocoVoxLabel, ShocoVoxRenderData, ShocoVoxRenderNode, ShocoVoxRenderPipeline,
};

use bevy::{
    app::{App, Plugin},
    asset::{AssetServer, Assets, Handle},
    ecs::{
        system::{Res, ResMut},
        world::{FromWorld, World},
    },
    prelude::{ExtractSchedule, IntoSystemConfigs},
    render::{
        extract_resource::ExtractResourcePlugin,
        prelude::Image,
        render_asset::{RenderAssetUsages, RenderAssets},
        render_graph::{self, RenderGraph},
        render_resource::{
            encase::{StorageBuffer, UniformBuffer},
            AsBindGroup, BindingResource, BufferInitDescriptor, BufferUsages, CachedPipelineState,
            ComputePassDescriptor, ComputePipelineDescriptor, Extent3d, PipelineCache,
            TextureDimension, TextureFormat, TextureUsages,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::{FallbackImage, GpuImage},
        MainWorld, Render, RenderApp, RenderSet,
    },
};

use std::borrow::Cow;

pub fn create_viewing_glass(
    viewport: &Viewport,
    resolution: [u32; 2],
    images: ResMut<Assets<Image>>,
) -> ShocoVoxViewingGlass {
    ShocoVoxViewingGlass {
        output_texture: create_ouput_texture(resolution, images),
        viewport: *viewport,
    }
}

impl FromWorld for ShocoVoxRenderPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let viewing_glass_bind_group_layout =
            ShocoVoxViewingGlass::bind_group_layout(render_device);
        let render_data_bind_group_layout = ShocoVoxRenderData::bind_group_layout(render_device);
        let shader = world
            .resource::<AssetServer>()
            .load("shaders/viewport_render.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![
                viewing_glass_bind_group_layout.clone(),
                render_data_bind_group_layout.clone(),
            ],
            push_constant_ranges: Vec::new(),
            shader,
            shader_defs: vec![],
            entry_point: Cow::from("update"),
        });

        ShocoVoxRenderPipeline {
            victim_pointer: 0,
            render_queue: world.resource::<RenderQueue>().clone(),
            viewport_buffer: None,
            octree_meta_buffer: None,
            nodes_buffer: None,
            node_children_buffer: None,
            voxels_buffer: None,
            color_palette_buffer: None,
            data_meta_bytes_buffer: None,
            readable_data_meta_bytes_buffer: None,
            update_tree: true,
            viewing_glass_bind_group_layout,
            render_data_bind_group_layout,
            update_pipeline,
            viewing_glass_bind_group: None,
            tree_bind_group: None,
        }
    }
}

fn prepare_bind_groups(
    gpu_images: Res<RenderAssets<GpuImage>>,
    fallback_image: Res<FallbackImage>,
    render_device: Res<RenderDevice>,
    mut pipeline: ResMut<ShocoVoxRenderPipeline>,
    octree_viewing_glass: Res<ShocoVoxViewingGlass>,
    render_data: Res<ShocoVoxRenderData>,
) {
    let bind_group = octree_viewing_glass
        .as_bind_group(
            &pipeline.viewing_glass_bind_group_layout,
            &render_device,
            &gpu_images,
            &fallback_image,
        )
        .ok()
        .unwrap();
    pipeline.viewing_glass_bind_group = Some(bind_group.bind_group);

    if pipeline.update_tree {
        //=================================================================
        // Implementation with WGPU
        //=================================================================
        // Upload data to buffers
        let mut buffer = UniformBuffer::new(Vec::<u8>::new());
        buffer.write(&render_data.octree_meta).unwrap();
        if let Some(metadata_buffer) = &pipeline.octree_meta_buffer {
            pipeline
                .render_queue
                .write_buffer(metadata_buffer, 0, &buffer.into_inner())
        } else {
            let metadata_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("Octree Metadata Buffer"),
                contents: &buffer.into_inner(),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });
            pipeline.octree_meta_buffer = Some(metadata_buffer);
        }

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&render_data.nodes).unwrap();
        if let Some(nodes_buffer) = &pipeline.nodes_buffer {
            pipeline
                .render_queue
                .write_buffer(nodes_buffer, 0, &buffer.into_inner())
        } else {
            let nodes_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("Octree Nodes Buffer"),
                contents: &buffer.into_inner(),
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            });
            pipeline.nodes_buffer = Some(nodes_buffer);
        }

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&render_data.node_children).unwrap();
        if let Some(children_buffer) = &pipeline.node_children_buffer {
            pipeline
                .render_queue
                .write_buffer(children_buffer, 0, &buffer.into_inner())
        } else {
            let children_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("Octree Children Buffer"),
                contents: &buffer.into_inner(),
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            });
            pipeline.node_children_buffer = Some(children_buffer);
        }

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&render_data.voxels).unwrap();
        if let Some(voxels_buffer) = &pipeline.voxels_buffer {
            pipeline
                .render_queue
                .write_buffer(voxels_buffer, 0, &buffer.into_inner())
        } else {
            let voxels_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("Octree Voxels Buffer"),
                contents: &buffer.into_inner(),
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            });
            pipeline.voxels_buffer = Some(voxels_buffer);
        }

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&render_data.data_meta_bytes).unwrap();
        if let Some(data_meta_bytes_buffer) = &pipeline.data_meta_bytes_buffer {
            pipeline
                .render_queue
                .write_buffer(data_meta_bytes_buffer, 0, &buffer.into_inner())
        } else {
            let data_meta_bytes_buffer =
                render_device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("Octree Meta Bytes Buffer"),
                    contents: &buffer.into_inner(),
                    usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
                });
            pipeline.data_meta_bytes_buffer = Some(data_meta_bytes_buffer);
        }

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&render_data.color_palette).unwrap();
        if let Some(color_palette_buffer) = &pipeline.color_palette_buffer {
            pipeline
                .render_queue
                .write_buffer(color_palette_buffer, 0, &buffer.into_inner())
        } else {
            let color_palette_buffer =
                render_device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("Octree Color Palette Buffer"),
                    contents: &buffer.into_inner(),
                    usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                });
            pipeline.color_palette_buffer = Some(color_palette_buffer);
        }

        // Create bind group
        let tree_bind_group = render_device.create_bind_group(
            ShocoVoxRenderData::label(),
            &pipeline.render_data_bind_group_layout,
            &[
                bevy::render::render_resource::BindGroupEntry {
                    binding: 0,
                    resource: pipeline
                        .octree_meta_buffer
                        .as_ref()
                        .unwrap()
                        .as_entire_binding(),
                },
                bevy::render::render_resource::BindGroupEntry {
                    binding: 1,
                    resource: pipeline.nodes_buffer.as_ref().unwrap().as_entire_binding(),
                },
                bevy::render::render_resource::BindGroupEntry {
                    binding: 2,
                    resource: pipeline
                        .node_children_buffer
                        .as_ref()
                        .unwrap()
                        .as_entire_binding(),
                },
                bevy::render::render_resource::BindGroupEntry {
                    binding: 3,
                    resource: pipeline.voxels_buffer.as_ref().unwrap().as_entire_binding(),
                },
                bevy::render::render_resource::BindGroupEntry {
                    binding: 4,
                    resource: pipeline
                        .color_palette_buffer
                        .as_ref()
                        .unwrap()
                        .as_entire_binding(),
                },
                bevy::render::render_resource::BindGroupEntry {
                    binding: 5,
                    resource: pipeline
                        .data_meta_bytes_buffer
                        .as_ref()
                        .unwrap()
                        .as_entire_binding(),
                },
            ],
        );
        pipeline.tree_bind_group = Some(tree_bind_group);

        //=================================================================
        // Implementation with AsBindGroup
        //=================================================================
        // let tree_bind_group = render_data
        //     .as_bind_group(
        //         &pipeline.render_data_bind_group_layout,
        //         &render_device,
        //         &gpu_images,
        //         &fallback_image,
        //     )
        //     .ok()
        //     .unwrap();

        // // println!("bindings: {:?}", tree_bind_group.bindings);
        // // let bindings = ;
        // //TODO: set buffers by binding index(bindings[x].0), instead of array index
        // // if let bevy::render::render_resource::OwnedBindingResource::Buffer(buf) =
        // //     &tree_bind_group.bindings[2].1
        // // {
        // //     debug_assert_eq!(tree_bind_group.bindings[2].0, 2);
        // //     pipeline.nodes_children_buffer = Some(buf.clone());
        // // }
        // if let bevy::render::render_resource::OwnedBindingResource::Buffer(buf) =
        //     &tree_bind_group.bindings[0].1
        // {
        //     // compare binding to ShocoVoxRenderData field
        //     debug_assert_eq!(tree_bind_group.bindings[0].0, 5);
        //     pipeline.cache_bytes_buffer = Some(buf.clone());
        // }

        // pipeline.tree_bind_group = Some(tree_bind_group.bind_group);

        // Create the staging buffer helping in reading data from the GPU
        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&render_data.data_meta_bytes).unwrap();
        if let Some(readable_data_meta_bytes_buffer) = &pipeline.readable_data_meta_bytes_buffer {
            pipeline.render_queue.write_buffer(
                readable_data_meta_bytes_buffer,
                0,
                &buffer.into_inner(),
            )
        } else {
            let readable_data_meta_bytes_buffer =
                render_device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("Octree Cache Bytes Buffer"),
                    contents: &buffer.into_inner(),
                    usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                });
            pipeline.readable_data_meta_bytes_buffer = Some(readable_data_meta_bytes_buffer);
        }

        pipeline.update_tree = false;
    }
}

pub(crate) fn handle_gpu_readback(
    render_device: Res<RenderDevice>,
    svx_data: Option<ResMut<ShocoVoxRenderData>>,
    svx_pipeline: Option<ResMut<ShocoVoxRenderPipeline>>,
    mut world: ResMut<MainWorld>,
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

pub(crate) fn handle_cache(
    svx_data: Option<ResMut<ShocoVoxRenderData>>,
    svx_pipeline: Option<ResMut<ShocoVoxRenderPipeline>>,
    mut world: ResMut<MainWorld>,
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

pub(crate) fn create_ouput_texture(
    resolution: [u32; 2],
    mut images: ResMut<Assets<Image>>,
) -> Handle<Image> {
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
    output_texture.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    images.add(output_texture)
}

impl Plugin for ShocoVoxRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractResourcePlugin::<ShocoVoxViewingGlass>::default(),
            ExtractResourcePlugin::<ShocoVoxRenderData>::default(),
        ));
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            ExtractSchedule,
            handle_cache.in_set(RenderSet::PrepareResources),
        );
        render_app.add_systems(
            ExtractSchedule,
            handle_gpu_readback.in_set(RenderSet::Cleanup),
        );
        render_app.add_systems(
            Render,
            prepare_bind_groups.in_set(RenderSet::PrepareBindGroups),
        );
        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(
            ShocoVoxLabel,
            ShocoVoxRenderNode {
                ready: false,
                resolution: self.resolution,
            },
        );
        render_graph.add_node_edge(ShocoVoxLabel, bevy::render::graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<ShocoVoxRenderPipeline>();
    }
}

const WORKGROUP_SIZE: u32 = 8;
impl render_graph::Node for ShocoVoxRenderNode {
    fn update(&mut self, world: &mut World) {
        {
            let render_data = world.get_resource::<ShocoVoxRenderData>();
            let svx_pipeline = world.resource::<ShocoVoxRenderPipeline>();
            let pipeline_cache = world.resource::<PipelineCache>();
            if !self.ready {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(svx_pipeline.update_pipeline)
                {
                    self.ready = render_data.is_some();
                }
            }
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let svx_pipeline = world.resource::<ShocoVoxRenderPipeline>();

        let command_encoder = render_context.command_encoder();
        if self.ready {
            {
                let mut pass =
                    command_encoder.begin_compute_pass(&ComputePassDescriptor::default());

                pass.set_bind_group(
                    0,
                    svx_pipeline.viewing_glass_bind_group.as_ref().unwrap(),
                    &[],
                );
                pass.set_bind_group(1, svx_pipeline.tree_bind_group.as_ref().unwrap(), &[]);
                let pipeline = pipeline_cache
                    .get_compute_pipeline(svx_pipeline.update_pipeline)
                    .unwrap();
                pass.set_pipeline(pipeline);
                pass.dispatch_workgroups(
                    self.resolution[0] / WORKGROUP_SIZE,
                    self.resolution[1] / WORKGROUP_SIZE,
                    1,
                );
            }

            command_encoder.copy_buffer_to_buffer(
                svx_pipeline.data_meta_bytes_buffer.as_ref().unwrap(),
                0,
                svx_pipeline
                    .readable_data_meta_bytes_buffer
                    .as_ref()
                    .unwrap(),
                0,
                std::mem::size_of::<u32>() as u64,
            )
        }
        Ok(())
    }
}
