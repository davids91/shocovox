use crate::octree::raytracing::bevy::types::{
    OctreeGPUView, OctreeRenderData, OctreeSpyGlass, SvxRenderNode, SvxRenderPipeline,
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
            AsBindGroup, BufferDescriptor, BufferInitDescriptor, BufferUsages, CachedPipelineState,
            ComputePassDescriptor, ComputePipelineDescriptor, OwnedBindingResource, PipelineCache,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::{FallbackImage, GpuImage},
    },
};
use std::borrow::Cow;

use super::types::OctreeRenderDataResources;

impl FromWorld for SvxRenderPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let spyglass_bind_group_layout = OctreeSpyGlass::bind_group_layout(render_device);
        let render_data_bind_group_layout = OctreeRenderData::bind_group_layout(render_device);
        let shader = world
            .resource::<AssetServer>()
            .load("shaders/viewport_render.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![
                spyglass_bind_group_layout.clone(),
                render_data_bind_group_layout.clone(),
            ],
            push_constant_ranges: Vec::new(),
            shader,
            shader_defs: vec![],
            entry_point: Cow::from("update"),
        });

        SvxRenderPipeline {
            render_queue: world.resource::<RenderQueue>().clone(),
            update_tree: true,
            spyglass_bind_group_layout,
            render_data_bind_group_layout,
            update_pipeline,
            resources: None,
        }
    }
}

//##############################################################################
//  ███████████   █████  █████ ██████   █████
// ░░███░░░░░███ ░░███  ░░███ ░░██████ ░░███
//  ░███    ░███  ░███   ░███  ░███░███ ░███
//  ░██████████   ░███   ░███  ░███░░███░███
//  ░███░░░░░███  ░███   ░███  ░███ ░░██████
//  ░███    ░███  ░███   ░███  ░███  ░░█████
//  █████   █████ ░░████████   █████  ░░█████
// ░░░░░   ░░░░░   ░░░░░░░░   ░░░░░    ░░░░░
//##############################################################################
const WORKGROUP_SIZE: u32 = 8;
impl render_graph::Node for SvxRenderNode {
    fn update(&mut self, world: &mut World) {
        {
            let svx_pipeline = world.resource::<SvxRenderPipeline>();
            let pipeline_cache = world.resource::<PipelineCache>();
            let tree_gpu_view = world.get_resource::<OctreeGPUView>();
            if !self.ready {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(svx_pipeline.update_pipeline)
                {
                    self.ready = tree_gpu_view.is_some();
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
        let svx_pipeline = world.resource::<SvxRenderPipeline>();

        let command_encoder = render_context.command_encoder();
        if self.ready {
            {
                let mut pass =
                    command_encoder.begin_compute_pass(&ComputePassDescriptor::default());

                pass.set_bind_group(
                    0,
                    &svx_pipeline.resources.as_ref().unwrap().spyglass_bind_group,
                    &[],
                );
                pass.set_bind_group(
                    1,
                    &svx_pipeline.resources.as_ref().unwrap().tree_bind_group,
                    &[],
                );
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
                &svx_pipeline.resources.as_ref().unwrap().metadata_buffer,
                0,
                &svx_pipeline
                    .resources
                    .as_ref()
                    .unwrap()
                    .readable_metadata_buffer,
                0,
                std::mem::size_of::<u32>() as u64,
            );
            // +++ DEBUG +++
            command_encoder.copy_buffer_to_buffer(
                &svx_pipeline.resources.as_ref().unwrap().debug_gpu_interface,
                0,
                &svx_pipeline
                    .resources
                    .as_ref()
                    .unwrap()
                    .readable_debug_gpu_interface,
                0,
                std::mem::size_of::<u32>() as u64,
            )
            // --- DEBUG ---
        }
        Ok(())
    }
}

//##############################################################################
//    █████████  ███████████   ██████████   █████████   ███████████ ██████████
//   ███░░░░░███░░███░░░░░███ ░░███░░░░░█  ███░░░░░███ ░█░░░███░░░█░░███░░░░░█
//  ███     ░░░  ░███    ░███  ░███  █ ░  ░███    ░███ ░   ░███  ░  ░███  █ ░
// ░███          ░██████████   ░██████    ░███████████     ░███     ░██████
// ░███          ░███░░░░░███  ░███░░█    ░███░░░░░███     ░███     ░███░░█
// ░░███     ███ ░███    ░███  ░███ ░   █ ░███    ░███     ░███     ░███ ░   █
//  ░░█████████  █████   █████ ██████████ █████   █████    █████    ██████████
//   ░░░░░░░░░  ░░░░░   ░░░░░ ░░░░░░░░░░ ░░░░░   ░░░░░    ░░░░░    ░░░░░░░░░░
//  ███████████  █████ ██████   █████ ██████████
// ░░███░░░░░███░░███ ░░██████ ░░███ ░░███░░░░███
//  ░███    ░███ ░███  ░███░███ ░███  ░███   ░░███
//  ░██████████  ░███  ░███░░███░███  ░███    ░███
//  ░███░░░░░███ ░███  ░███ ░░██████  ░███    ░███
//  ░███    ░███ ░███  ░███  ░░█████  ░███    ███
//  ███████████  █████ █████  ░░█████ ██████████
// ░░░░░░░░░░░  ░░░░░ ░░░░░    ░░░░░ ░░░░░░░░░░
//    █████████  ███████████      ███████    █████  █████ ███████████   █████████
//   ███░░░░░███░░███░░░░░███   ███░░░░░███ ░░███  ░░███ ░░███░░░░░███ ███░░░░░███
//  ███     ░░░  ░███    ░███  ███     ░░███ ░███   ░███  ░███    ░███░███    ░░░
// ░███          ░██████████  ░███      ░███ ░███   ░███  ░██████████ ░░█████████
// ░███    █████ ░███░░░░░███ ░███      ░███ ░███   ░███  ░███░░░░░░   ░░░░░░░░███
// ░░███  ░░███  ░███    ░███ ░░███     ███  ░███   ░███  ░███         ███    ░███
//  ░░█████████  █████   █████ ░░░███████░   ░░████████   █████       ░░█████████
//   ░░░░░░░░░  ░░░░░   ░░░░░    ░░░░░░░      ░░░░░░░░   ░░░░░         ░░░░░░░░░
//##############################################################################
pub(crate) fn prepare_bind_groups(
    gpu_images: Res<RenderAssets<GpuImage>>,
    fallback_image: Res<FallbackImage>,
    render_device: Res<RenderDevice>,
    mut pipeline: ResMut<SvxRenderPipeline>,
    tree_gpu_view: ResMut<OctreeGPUView>,
) {
    if pipeline.resources.is_some() && !pipeline.update_tree {
        return;
    }

    let data_handler = tree_gpu_view.data_handler.lock().unwrap();
    if let Some(resources) = &pipeline.resources {
        let mut buffer = UniformBuffer::new(Vec::<u8>::new());
        buffer.write(&data_handler.render_data.octree_meta).unwrap();
        pipeline
            .render_queue
            .write_buffer(&resources.metadata_buffer, 0, &buffer.into_inner());

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&data_handler.render_data.metadata).unwrap();
        pipeline
            .render_queue
            .write_buffer(&resources.metadata_buffer, 0, &buffer.into_inner());

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer
            .write(&data_handler.render_data.node_children)
            .unwrap();
        pipeline.render_queue.write_buffer(
            &resources.node_children_buffer,
            0,
            &buffer.into_inner(),
        );

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&data_handler.render_data.node_ocbits).unwrap();
        pipeline
            .render_queue
            .write_buffer(&resources.node_ocbits_buffer, 0, &buffer.into_inner());

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&data_handler.render_data.voxels).unwrap();
        pipeline
            .render_queue
            .write_buffer(&resources.voxels_buffer, 0, &buffer.into_inner());

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer
            .write(&data_handler.render_data.color_palette)
            .unwrap();
        pipeline
            .render_queue
            .write_buffer(&resources.color_palette_buffer, 0, &buffer.into_inner())
    } else {
        // Create the staging buffer helping in reading data from the GPU
        let readable_metadata_buffer = render_device.create_buffer(&BufferDescriptor {
            mapped_at_creation: false,
            size: (data_handler.render_data.metadata.len() * 4) as u64,
            label: Some("Octree Node metadata Buffer"),
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        });

        // +++ DEBUG +++
        let buffer = UniformBuffer::new(vec![0, 0, 0, 0]);
        let debug_gpu_interface = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Octree Debug Buffer"),
            contents: &buffer.into_inner(),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });
        let readable_debug_gpu_interface = render_device.create_buffer(&BufferDescriptor {
            mapped_at_creation: false,
            size: 4,
            label: Some("Octree Debug interface Buffer"),
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        });
        // --- DEBUG ---
        let mut buffer = UniformBuffer::new(Vec::<u8>::new());
        buffer.write(&data_handler.render_data.octree_meta).unwrap();
        let octree_meta_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Octree Metadata Buffer"),
            contents: &buffer.into_inner(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&data_handler.render_data.metadata).unwrap();
        let metadata_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Octree Nodes Buffer"),
            contents: &buffer.into_inner(),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
        });

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer
            .write(&data_handler.render_data.node_children)
            .unwrap();
        let node_children_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Octree Node Children Buffer"),
            contents: &buffer.into_inner(),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&data_handler.render_data.node_ocbits).unwrap();
        let node_ocbits_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Octree Node Occupied bits Buffer"),
            contents: &buffer.into_inner(),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&data_handler.render_data.voxels).unwrap();
        let voxels_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Octree Voxels Buffer"),
            contents: &buffer.into_inner(),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer
            .write(&data_handler.render_data.color_palette)
            .unwrap();
        let color_palette_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Octree Color Palette Buffer"),
            contents: &buffer.into_inner(),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        // Create bind group
        let tree_bind_group = render_device.create_bind_group(
            OctreeRenderData::label(),
            &pipeline.render_data_bind_group_layout,
            &[
                bevy::render::render_resource::BindGroupEntry {
                    binding: 0,
                    resource: octree_meta_buffer.as_entire_binding(),
                },
                bevy::render::render_resource::BindGroupEntry {
                    binding: 1,
                    resource: metadata_buffer.as_entire_binding(),
                },
                bevy::render::render_resource::BindGroupEntry {
                    binding: 2,
                    resource: node_children_buffer.as_entire_binding(),
                },
                bevy::render::render_resource::BindGroupEntry {
                    binding: 3,
                    resource: node_ocbits_buffer.as_entire_binding(),
                },
                bevy::render::render_resource::BindGroupEntry {
                    binding: 4,
                    resource: voxels_buffer.as_entire_binding(),
                },
                bevy::render::render_resource::BindGroupEntry {
                    binding: 5,
                    resource: color_palette_buffer.as_entire_binding(),
                },
                // +++ DEBUG +++
                bevy::render::render_resource::BindGroupEntry {
                    binding: 6,
                    resource: debug_gpu_interface.as_entire_binding(),
                },
                // --- DEBUG ---
            ],
        );

        let spyglass_prepared_bind_group = tree_gpu_view
            .spyglass
            .as_bind_group(
                &pipeline.spyglass_bind_group_layout,
                &render_device,
                &gpu_images,
                &fallback_image,
            )
            .ok()
            .unwrap();
        let spyglass_bind_group = spyglass_prepared_bind_group.bind_group;

        debug_assert_eq!(
            1, spyglass_prepared_bind_group.bindings[1].0,
            "Expected Spyglass binding to be 1, instead of {:?}",
            spyglass_prepared_bind_group.bindings[1].0
        );
        let viewport_buffer = if let OwnedBindingResource::Buffer(buffer) =
            &spyglass_prepared_bind_group.bindings[1].1
        {
            buffer.clone()
        } else {
            panic!(
                "Unexpected binding for spyglass bind group; Expected buffer instead of : {:?}",
                spyglass_prepared_bind_group.bindings[1].1
            );
        };

        pipeline.resources = Some(OctreeRenderDataResources {
            spyglass_bind_group,
            tree_bind_group,
            viewport_buffer,
            octree_meta_buffer,
            metadata_buffer,
            readable_metadata_buffer,
            node_children_buffer,
            node_ocbits_buffer,
            voxels_buffer,
            color_palette_buffer,
            debug_gpu_interface,
            readable_debug_gpu_interface,
        });
    }

    pipeline.update_tree = false;
}
