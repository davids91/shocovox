use crate::{
    octree::types::PaletteIndexValues,
    raytracing::bevy::types::{
        OctreeGPUView, OctreeMetaData, SvxRenderNode, SvxRenderPipeline, Viewport,
    },
};
use bevy::{
    asset::AssetServer,
    ecs::{
        system::{Res, ResMut},
        world::{FromWorld, World},
    },
    prelude::Vec4,
    render::{
        render_asset::RenderAssets,
        render_graph::{self},
        render_resource::{
            encase::{StorageBuffer, UniformBuffer},
            BindGroup, BindGroupEntry, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
            BufferBindingType, BufferDescriptor, BufferInitDescriptor, BufferUsages,
            CachedPipelineState, ComputePassDescriptor, ComputePipelineDescriptor, PipelineCache,
            ShaderSize, ShaderStages, ShaderType, StorageTextureAccess, TextureFormat,
            TextureViewDimension,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::GpuImage,
    },
};
use std::borrow::Cow;

use super::types::{OctreeRenderDataResources, SvxViewSet};

impl FromWorld for SvxRenderPipeline {
    //##############################################################################
    // ███████████  █████ ██████   █████ ██████████
    // ░░███░░░░░███░░███ ░░██████ ░░███ ░░███░░░░███
    //  ░███    ░███ ░███  ░███░███ ░███  ░███   ░░███
    //  ░██████████  ░███  ░███░░███░███  ░███    ░███
    //  ░███░░░░░███ ░███  ░███ ░░██████  ░███    ░███
    //  ░███    ░███ ░███  ░███  ░░█████  ░███    ███
    //  ███████████  █████ █████  ░░█████ ██████████
    // ░░░░░░░░░░░  ░░░░░ ░░░░░    ░░░░░ ░░░░░░░░░░
    //    █████████  ███████████      ███████    █████  █████ ███████████
    //   ███░░░░░███░░███░░░░░███   ███░░░░░███ ░░███  ░░███ ░░███░░░░░███
    //  ███     ░░░  ░███    ░███  ███     ░░███ ░███   ░███  ░███    ░███
    // ░███          ░██████████  ░███      ░███ ░███   ░███  ░██████████
    // ░███    █████ ░███░░░░░███ ░███      ░███ ░███   ░███  ░███░░░░░░
    // ░░███  ░░███  ░███    ░███ ░░███     ███  ░███   ░███  ░███
    //  ░░█████████  █████   █████ ░░░███████░   ░░████████   █████
    //   ░░░░░░░░░  ░░░░░   ░░░░░    ░░░░░░░      ░░░░░░░░   ░░░░░
    //  █████         █████████   █████ █████    ███████    █████  █████ ███████████
    // ░░███         ███░░░░░███ ░░███ ░░███   ███░░░░░███ ░░███  ░░███ ░█░░░███░░░█
    //  ░███        ░███    ░███  ░░███ ███   ███     ░░███ ░███   ░███ ░   ░███  ░
    //  ░███        ░███████████   ░░█████   ░███      ░███ ░███   ░███     ░███
    //  ░███        ░███░░░░░███    ░░███    ░███      ░███ ░███   ░███     ░███
    //  ░███      █ ░███    ░███     ░███    ░░███     ███  ░███   ░███     ░███
    //  ███████████ █████   █████    █████    ░░░███████░   ░░████████      █████
    // ░░░░░░░░░░░ ░░░░░   ░░░░░    ░░░░░       ░░░░░░░      ░░░░░░░░      ░░░░░
    //##############################################################################

    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let spyglass_bind_group_layout = render_device.create_bind_group_layout(
            "OctreeSpyGlass",
            &[
                BindGroupLayoutEntry {
                    binding: 0u32,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::ReadWrite,
                        format: TextureFormat::Rgba8Unorm,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1u32,
                    visibility: ShaderStages::all(),
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(<Viewport as ShaderType>::min_size()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2u32,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: Some(<Vec<u32> as ShaderType>::min_size()),
                    },
                    count: None,
                },
            ],
        );
        let render_data_bind_group_layout = render_device.create_bind_group_layout(
            "OctreeRenderData",
            &[
                BindGroupLayoutEntry {
                    binding: 0u32,
                    visibility: ShaderStages::all(),
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(<OctreeMetaData as ShaderType>::min_size()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1u32,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: Some(<Vec<u32> as ShaderType>::min_size()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2u32,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: Some(<Vec<u32> as ShaderType>::min_size()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3u32,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: Some(<Vec<u32> as ShaderType>::min_size()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 4u32,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: Some(<Vec<u32> as ShaderType>::min_size()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 5u32,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: Some(<Vec<u32> as ShaderType>::min_size()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 6u32,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: Some(<Vec<PaletteIndexValues> as ShaderType>::min_size()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 7u32,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: Some(<Vec<Vec4> as ShaderType>::min_size()),
                    },
                    count: None,
                },
            ],
        );
        let shader = world
            .resource::<AssetServer>()
            .load("shaders/viewport_render.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            zero_initialize_workgroup_memory: false,
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
        let svx_pipeline = world.resource::<SvxRenderPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        if !self.ready {
            if let CachedPipelineState::Ok(_) =
                pipeline_cache.get_compute_pipeline_state(svx_pipeline.update_pipeline)
            {
                self.ready = !world.resource::<SvxViewSet>().views.is_empty();
            }
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let svx_pipeline = world.resource::<SvxRenderPipeline>();
        let svx_viewset = world.resource::<SvxViewSet>();
        let current_view = svx_viewset.views[0].lock().unwrap();
        let resources = svx_viewset.resources[0].as_ref();
        if self.ready && resources.is_some() {
            let resources = resources.unwrap();
            let pipeline_cache = world.resource::<PipelineCache>();
            let command_encoder = render_context.command_encoder();
            let data_handler = &current_view.data_handler;
            if !current_view.data_ready {
                command_encoder.copy_buffer_to_buffer(
                    &resources.node_metadata_buffer,
                    0,
                    &resources.readable_used_bits_buffer,
                    0,
                    (std::mem::size_of_val(&data_handler.render_data.node_metadata[0])) as u64,
                );
            } else {
                let mut pass =
                    command_encoder.begin_compute_pass(&ComputePassDescriptor::default());

                pass.set_bind_group(0, &resources.spyglass_bind_group, &[]);
                pass.set_bind_group(1, &resources.tree_bind_group, &[]);
                let pipeline = pipeline_cache
                    .get_compute_pipeline(svx_pipeline.update_pipeline)
                    .unwrap();
                pass.set_pipeline(pipeline);
                pass.dispatch_workgroups(
                    current_view.resolution[0] / WORKGROUP_SIZE,
                    current_view.resolution[1] / WORKGROUP_SIZE,
                    1,
                );
            }

            command_encoder.copy_buffer_to_buffer(
                &resources.used_bits_buffer,
                0,
                &resources.readable_used_bits_buffer,
                0,
                (std::mem::size_of_val(&data_handler.render_data.used_bits[0])
                    * data_handler.render_data.used_bits.len()) as u64,
            );

            debug_assert!(
                !current_view.spyglass.node_requests.is_empty(),
                "Expected node requests array to not be empty"
            );
            command_encoder.copy_buffer_to_buffer(
                &resources.node_requests_buffer,
                0,
                &resources.readable_node_requests_buffer,
                0,
                (std::mem::size_of_val(&current_view.spyglass.node_requests[0])
                    * current_view.spyglass.node_requests.len()) as u64,
            );
        }
        Ok(())
    }
}

//##############################################################################
//   █████████  ███████████  █████ █████
//  ███░░░░░███░░███░░░░░███░░███ ░░███
// ░███    ░░░  ░███    ░███ ░░███ ███
// ░░█████████  ░██████████   ░░█████
//  ░░░░░░░░███ ░███░░░░░░     ░░███
//  ███    ░███ ░███            ░███
// ░░█████████  █████           █████
//  ░░░░░░░░░  ░░░░░           ░░░░░
//    █████████  █████         █████████    █████████   █████████
//   ███░░░░░███░░███         ███░░░░░███  ███░░░░░███ ███░░░░░███
//  ███     ░░░  ░███        ░███    ░███ ░███    ░░░ ░███    ░░░
// ░███          ░███        ░███████████ ░░█████████ ░░█████████
// ░███    █████ ░███        ░███░░░░░███  ░░░░░░░░███ ░░░░░░░░███
// ░░███  ░░███  ░███      █ ░███    ░███  ███    ░███ ███    ░███
//  ░░█████████  ███████████ █████   █████░░█████████ ░░█████████
//   ░░░░░░░░░  ░░░░░░░░░░░ ░░░░░   ░░░░░  ░░░░░░░░░   ░░░░░░░░░
//    █████████  ███████████      ███████    █████  █████ ███████████
//   ███░░░░░███░░███░░░░░███   ███░░░░░███ ░░███  ░░███ ░░███░░░░░███
//  ███     ░░░  ░███    ░███  ███     ░░███ ░███   ░███  ░███    ░███
// ░███          ░██████████  ░███      ░███ ░███   ░███  ░██████████
// ░███    █████ ░███░░░░░███ ░███      ░███ ░███   ░███  ░███░░░░░░
// ░░███  ░░███  ░███    ░███ ░░███     ███  ░███   ░███  ░███
//  ░░█████████  █████   █████ ░░░███████░   ░░████████   █████
//##############################################################################
pub(crate) fn create_spyglass_bind_group(
    pipeline: &mut SvxRenderPipeline,
    render_device: &Res<RenderDevice>,
    gpu_images: &Res<RenderAssets<GpuImage>>,
    tree_view: &OctreeGPUView,
) -> (BindGroup, Buffer, Buffer, Buffer) {
    let mut buffer = UniformBuffer::new([0u8; Viewport::SHADER_SIZE.get() as usize]);
    buffer.write(&tree_view.spyglass.viewport).unwrap();
    let viewport_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("BoxTree Viewport Buffer"),
        contents: &buffer.into_inner(),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    debug_assert!(
        !tree_view.spyglass.node_requests.is_empty(),
        "Expected node requests array to not be empty"
    );
    let mut buffer = StorageBuffer::new(Vec::<u8>::new());
    buffer.write(&tree_view.spyglass.node_requests).unwrap();
    let node_requests_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("BoxTree Node requests Buffer"),
        contents: &buffer.into_inner(),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
    });

    let readable_node_requests_buffer = render_device.create_buffer(&BufferDescriptor {
        mapped_at_creation: false,
        size: (tree_view.spyglass.node_requests.len()
            * std::mem::size_of_val(&tree_view.spyglass.node_requests[0])) as u64,
        label: Some("BoxTree Node requests staging Buffer"),
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
    });

    (
        render_device.create_bind_group(
            "OctreeSpyGlass",
            &pipeline.spyglass_bind_group_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(
                        &gpu_images
                            .get(&tree_view.output_texture)
                            .unwrap()
                            .texture_view,
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: viewport_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: node_requests_buffer.as_entire_binding(),
                },
            ],
        ),
        viewport_buffer,
        node_requests_buffer,
        readable_node_requests_buffer,
    )
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
//  █████   █████ █████ ██████████ █████   ███   █████    ███████████   ██████████  █████████
// ░░███   ░░███ ░░███ ░░███░░░░░█░░███   ░███  ░░███    ░░███░░░░░███ ░░███░░░░░█ ███░░░░░███
//  ░███    ░███  ░███  ░███  █ ░  ░███   ░███   ░███     ░███    ░███  ░███  █ ░ ░███    ░░░
//  ░███    ░███  ░███  ░██████    ░███   ░███   ░███     ░██████████   ░██████   ░░█████████
//  ░░███   ███   ░███  ░███░░█    ░░███  █████  ███      ░███░░░░░███  ░███░░█    ░░░░░░░░███
//   ░░░█████░    ░███  ░███ ░   █  ░░░█████░█████░       ░███    ░███  ░███ ░   █ ███    ░███
//     ░░███      █████ ██████████    ░░███ ░░███         █████   █████ ██████████░░█████████
//      ░░░      ░░░░░ ░░░░░░░░░░      ░░░   ░░░         ░░░░░   ░░░░░ ░░░░░░░░░░  ░░░░░░░░░
//##############################################################################
fn create_view_resources(
    pipeline: &mut SvxRenderPipeline,
    render_device: Res<RenderDevice>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    tree_view: &OctreeGPUView,
) -> OctreeRenderDataResources {
    let render_data = &tree_view.data_handler.render_data;
    //##############################################################################
    //  ███████████ ███████████   ██████████ ██████████
    // ░█░░░███░░░█░░███░░░░░███ ░░███░░░░░█░░███░░░░░█
    // ░   ░███  ░  ░███    ░███  ░███  █ ░  ░███  █ ░
    //     ░███     ░██████████   ░██████    ░██████
    //     ░███     ░███░░░░░███  ░███░░█    ░███░░█
    //     ░███     ░███    ░███  ░███ ░   █ ░███ ░   █
    //     █████    █████   █████ ██████████ ██████████
    //    ░░░░░    ░░░░░   ░░░░░ ░░░░░░░░░░ ░░░░░░░░░░
    //    █████████  ███████████      ███████    █████  █████ ███████████
    //   ███░░░░░███░░███░░░░░███   ███░░░░░███ ░░███  ░░███ ░░███░░░░░███
    //  ███     ░░░  ░███    ░███  ███     ░░███ ░███   ░███  ░███    ░███
    // ░███          ░██████████  ░███      ░███ ░███   ░███  ░██████████
    // ░███    █████ ░███░░░░░███ ░███      ░███ ░███   ░███  ░███░░░░░░
    // ░░███  ░░███  ░███    ░███ ░░███     ███  ░███   ░███  ░███
    //  ░░█████████  █████   █████ ░░░███████░   ░░████████   █████
    //   ░░░░░░░░░  ░░░░░   ░░░░░    ░░░░░░░      ░░░░░░░░   ░░░░░
    //##############################################################################
    // Create the staging buffer helping in reading data from the GPU
    let readable_used_bits_buffer = render_device.create_buffer(&BufferDescriptor {
        mapped_at_creation: false,
        size: (render_data.used_bits.len() * 4) as u64,
        label: Some("BoxTree Node metadata staging Buffer"),
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
    });

    let mut buffer = UniformBuffer::new(Vec::<u8>::new());
    buffer.write(&render_data.octree_meta).unwrap();
    let boxtree_meta_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("BoxTree Tree Metadata Buffer"),
        contents: &buffer.into_inner(),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let mut buffer = StorageBuffer::new(Vec::<u8>::new());
    buffer.write(&render_data.used_bits).unwrap();
    let used_bits_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("BoxTree Used Bits Buffer"),
        contents: &buffer.into_inner(),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
    });

    let mut buffer = StorageBuffer::new(Vec::<u8>::new());
    buffer.write(&render_data.node_metadata).unwrap();
    let node_metadata_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("BoxTree Node Metadata Buffer"),
        contents: &buffer.into_inner(),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
    });

    let mut buffer = StorageBuffer::new(Vec::<u8>::new());
    buffer.write(&render_data.node_children).unwrap();
    let node_children_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("BoxTree Node Children Buffer"),
        contents: &buffer.into_inner(),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
    });

    let mut buffer = StorageBuffer::new(Vec::<u8>::new());
    buffer.write(&render_data.node_mips).unwrap();
    let node_mips_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("BoxTree Node MIPs Buffer"),
        contents: &buffer.into_inner(),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
    });

    let mut buffer = StorageBuffer::new(Vec::<u8>::new());
    buffer.write(&render_data.node_ocbits).unwrap();
    let node_ocbits_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("BoxTree Node Occupied Bits Buffer"),
        contents: &buffer.into_inner(),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
    });

    // One element in the brick metadata holds 16 bricks. See @OctreeRenderData
    let brick_size = (render_data.octree_meta.tree_properties & 0x0000FFFF).pow(3);
    let brick_element_count = (render_data.used_bits.len() * 31 * brick_size as usize) as u64;
    let one_voxel_byte_size = std::mem::size_of::<PaletteIndexValues>() as u64;
    let voxels_buffer = render_device.create_buffer(&BufferDescriptor {
        mapped_at_creation: false,
        size: one_voxel_byte_size * brick_element_count,
        label: Some("BoxTree Voxels Buffer"),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
    });

    let mut buffer = StorageBuffer::new(Vec::<u8>::new());
    buffer.write(&render_data.color_palette).unwrap();
    let color_palette_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("BoxTree Color Palette Buffer"),
        contents: &buffer.into_inner(),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
    });

    // Create bind group
    let tree_bind_group = render_device.create_bind_group(
        "BoxTreeRenderData",
        &pipeline.render_data_bind_group_layout,
        &[
            bevy::render::render_resource::BindGroupEntry {
                binding: 0,
                resource: boxtree_meta_buffer.as_entire_binding(),
            },
            bevy::render::render_resource::BindGroupEntry {
                binding: 1,
                resource: used_bits_buffer.as_entire_binding(),
            },
            bevy::render::render_resource::BindGroupEntry {
                binding: 2,
                resource: node_metadata_buffer.as_entire_binding(),
            },
            bevy::render::render_resource::BindGroupEntry {
                binding: 3,
                resource: node_children_buffer.as_entire_binding(),
            },
            bevy::render::render_resource::BindGroupEntry {
                binding: 4,
                resource: node_mips_buffer.as_entire_binding(),
            },
            bevy::render::render_resource::BindGroupEntry {
                binding: 5,
                resource: node_ocbits_buffer.as_entire_binding(),
            },
            bevy::render::render_resource::BindGroupEntry {
                binding: 6,
                resource: voxels_buffer.as_entire_binding(),
            },
            bevy::render::render_resource::BindGroupEntry {
                binding: 7,
                resource: color_palette_buffer.as_entire_binding(),
            },
        ],
    );

    let (spyglass_bind_group, viewport_buffer, node_requests_buffer, readable_node_requests_buffer) =
        create_spyglass_bind_group(pipeline, &render_device, &gpu_images, tree_view);

    OctreeRenderDataResources {
        spyglass_bind_group,
        viewport_buffer,
        node_requests_buffer,
        readable_node_requests_buffer,
        tree_bind_group,
        boxtree_meta_buffer,
        used_bits_buffer,
        node_metadata_buffer,
        node_children_buffer,
        node_mips_buffer,
        node_ocbits_buffer,
        voxels_buffer,
        color_palette_buffer,
        readable_used_bits_buffer,
    }
}

//##############################################################################
//  ███████████  ███████████   ██████████ ███████████    █████████   ███████████   ██████████
// ░░███░░░░░███░░███░░░░░███ ░░███░░░░░█░░███░░░░░███  ███░░░░░███ ░░███░░░░░███ ░░███░░░░░█
//  ░███    ░███ ░███    ░███  ░███  █ ░  ░███    ░███ ░███    ░███  ░███    ░███  ░███  █ ░
//  ░██████████  ░██████████   ░██████    ░██████████  ░███████████  ░██████████   ░██████
//  ░███░░░░░░   ░███░░░░░███  ░███░░█    ░███░░░░░░   ░███░░░░░███  ░███░░░░░███  ░███░░█
//  ░███         ░███    ░███  ░███ ░   █ ░███         ░███    ░███  ░███    ░███  ░███ ░   █
//  █████        █████   █████ ██████████ █████        █████   █████ █████   █████ ██████████
// ░░░░░        ░░░░░   ░░░░░ ░░░░░░░░░░ ░░░░░        ░░░░░   ░░░░░ ░░░░░   ░░░░░ ░░░░░░░░░░
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
/// Constructs buffers, bing groups and uploads rendering data at initialization and whenever prompted
pub(crate) fn prepare_bind_groups(
    gpu_images: Res<RenderAssets<GpuImage>>,
    render_device: Res<RenderDevice>,
    mut pipeline: ResMut<SvxRenderPipeline>,
    mut svx_viewset: ResMut<SvxViewSet>,
) {
    {
        // Handle when no udpates are needed
        let view = svx_viewset.views[0].lock().unwrap();
        // No need to update view if
        if view.spyglass.output_texture == view.output_texture // output texture stored in CPU matches with the one used in the GPU bind group
            && (view.new_resolution.is_some() // a new resolution is requested and the texture is pending still
                || (svx_viewset.resources[0].is_some() && !pipeline.update_tree))
        // or tree is up-todate
        {
            return;
        }
    }
    {
        // Rebuild view becasue of changed output texture ( most likely resolution change )
        let output_texture_changed = {
            let view = svx_viewset.views[0].lock().unwrap();
            svx_viewset.resources[0].is_some()
                && view.spyglass.output_texture != view.output_texture
                && gpu_images.get(&view.output_texture).is_some()
        };
        if output_texture_changed {
            let (
                spyglass_bind_group,
                viewport_buffer,
                node_requests_buffer,
                readable_node_requests_buffer,
            ) = create_spyglass_bind_group(
                &mut pipeline,
                &render_device,
                &gpu_images,
                &svx_viewset.views[0].lock().unwrap(),
            );

            let view_resources = svx_viewset.resources[0].as_mut().unwrap();
            view_resources.spyglass_bind_group = spyglass_bind_group;
            view_resources.viewport_buffer = viewport_buffer;
            view_resources.node_requests_buffer = node_requests_buffer;
            view_resources.readable_node_requests_buffer = readable_node_requests_buffer;

            // update spyglass output texture too!
            let mut view = svx_viewset.views[0].lock().unwrap();
            view.spyglass.output_texture = view.output_texture.clone();
        }
    }

    // build everything from the ground up
    if let Some(resources) = &svx_viewset.resources[0] {
        let tree_view = &svx_viewset.views[0].lock().unwrap();
        let render_data = &tree_view.data_handler.render_data;

        let mut buffer = UniformBuffer::new(Vec::<u8>::new());
        buffer.write(&render_data.octree_meta).unwrap();
        pipeline
            .render_queue
            .write_buffer(&resources.boxtree_meta_buffer, 0, &buffer.into_inner());

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&render_data.used_bits).unwrap();
        pipeline
            .render_queue
            .write_buffer(&resources.used_bits_buffer, 0, &buffer.into_inner());

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&render_data.node_metadata).unwrap();
        pipeline.render_queue.write_buffer(
            &resources.node_metadata_buffer,
            0,
            &buffer.into_inner(),
        );

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&render_data.node_children).unwrap();
        pipeline.render_queue.write_buffer(
            &resources.node_children_buffer,
            0,
            &buffer.into_inner(),
        );

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&render_data.node_mips).unwrap();
        pipeline
            .render_queue
            .write_buffer(&resources.node_mips_buffer, 0, &buffer.into_inner());

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&render_data.node_ocbits).unwrap();
        pipeline
            .render_queue
            .write_buffer(&resources.node_ocbits_buffer, 0, &buffer.into_inner());

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&render_data.color_palette).unwrap();
        pipeline
            .render_queue
            .write_buffer(&resources.color_palette_buffer, 0, &buffer.into_inner())
    } else {
        let view_resources = create_view_resources(
            &mut pipeline,
            render_device,
            gpu_images,
            &svx_viewset.views[0].lock().unwrap(),
        );
        svx_viewset.resources[0] = Some(view_resources);
    }

    pipeline.update_tree = false;
}
