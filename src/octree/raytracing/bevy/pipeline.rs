use crate::octree::raytracing::bevy::types::{
    ShocoVoxRenderData, ShocoVoxRenderNode, ShocoVoxRenderPipeline, ShocoVoxViewingGlass,
};

use bevy::{
    asset::AssetServer,
    ecs::{
        system::{Res, ResMut},
        world::{FromWorld, World},
    },
    render::{
        render_asset::RenderAssets,
        render_graph::{self},
        render_resource::{
            encase::{StorageBuffer, UniformBuffer},
            AsBindGroup, BufferInitDescriptor, BufferUsages, CachedPipelineState,
            ComputePassDescriptor, ComputePipelineDescriptor, PipelineCache,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::{FallbackImage, GpuImage},
    },
};
use std::borrow::Cow;

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
            octree_meta_buffer: None,
            nodes_buffer: None,
            node_children_buffer: None,
            node_ocbits_buffer: None,
            voxels_buffer: None,
            color_palette_buffer: None,
            readable_nodes_buffer: None,
            update_tree: true,
            viewing_glass_bind_group_layout,
            render_data_bind_group_layout,
            update_pipeline,
            viewing_glass_bind_group: None,
            tree_bind_group: None,
        }
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
                svx_pipeline.nodes_buffer.as_ref().unwrap(),
                0,
                svx_pipeline.readable_nodes_buffer.as_ref().unwrap(),
                0,
                std::mem::size_of::<u32>() as u64,
            )
        }
        Ok(())
    }
}

pub(crate) fn prepare_bind_groups(
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
                usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
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
                label: Some("Octree Node Children Buffer"),
                contents: &buffer.into_inner(),
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            });
            pipeline.node_children_buffer = Some(children_buffer);
        }

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&render_data.node_occupied_bits).unwrap();
        if let Some(ocbits_buffer) = &pipeline.node_ocbits_buffer {
            pipeline
                .render_queue
                .write_buffer(ocbits_buffer, 0, &buffer.into_inner())
        } else {
            let ocbits_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("Octree Node Occupied bits Buffer"),
                contents: &buffer.into_inner(),
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            });
            pipeline.node_ocbits_buffer = Some(ocbits_buffer);
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
                    resource: pipeline
                        .node_ocbits_buffer
                        .as_ref()
                        .unwrap()
                        .as_entire_binding(),
                },
                bevy::render::render_resource::BindGroupEntry {
                    binding: 4,
                    resource: pipeline.voxels_buffer.as_ref().unwrap().as_entire_binding(),
                },
                bevy::render::render_resource::BindGroupEntry {
                    binding: 5,
                    resource: pipeline
                        .color_palette_buffer
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
        buffer.write(&render_data.nodes).unwrap();
        if let Some(readable_nodes_buffer) = &pipeline.readable_nodes_buffer {
            pipeline
                .render_queue
                .write_buffer(readable_nodes_buffer, 0, &buffer.into_inner())
        } else {
            let readable_nodes_buffer =
                render_device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("Octree Cache Bytes Buffer"),
                    contents: &buffer.into_inner(),
                    usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                });
            pipeline.readable_nodes_buffer = Some(readable_nodes_buffer);
        }

        pipeline.update_tree = false;
    }
}
