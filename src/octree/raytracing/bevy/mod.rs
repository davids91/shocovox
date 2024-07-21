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
    ecs::system::{Res, ResMut},
    ecs::world::{FromWorld, World},
    prelude::IntoSystemConfigs,
    render::{
        extract_resource::ExtractResourcePlugin,
        prelude::Image,
        render_asset::{RenderAssetUsages, RenderAssets},
        render_graph,
        render_graph::RenderGraph,
        render_resource::{
            AsBindGroup, CachedPipelineState, ComputePassDescriptor, ComputePipelineDescriptor,
            Extent3d, PipelineCache, TextureDimension, TextureFormat, TextureUsages,
        },
        renderer::{RenderContext, RenderDevice},
        texture::{FallbackImage, GpuImage},
        Render, RenderApp, RenderSet,
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
        let tree_bind_group = render_data
            .as_bind_group(
                &pipeline.render_data_bind_group_layout,
                &render_device,
                &gpu_images,
                &fallback_image,
            )
            .ok()
            .unwrap();
        pipeline.tree_bind_group = Some(tree_bind_group.bind_group);
        pipeline.update_tree = false;
    }
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
        app.add_plugins(ExtractResourcePlugin::<ShocoVoxViewingGlass>::default());
        app.add_plugins(ExtractResourcePlugin::<ShocoVoxRenderData>::default());
        let render_app = app.sub_app_mut(RenderApp);
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
        let pipeline = world.resource::<ShocoVoxRenderPipeline>();
        let render_data = world.get_resource::<ShocoVoxRenderData>();
        let pipeline_cache = world.resource::<PipelineCache>();
        if !self.ready {
            if let CachedPipelineState::Ok(_) =
                pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline)
            {
                self.ready = render_data.is_some();
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
        let pipeline = world.resource::<ShocoVoxRenderPipeline>();

        if self.ready {
            let mut pass = render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor::default());

            pass.set_bind_group(0, pipeline.viewing_glass_bind_group.as_ref().unwrap(), &[]);
            pass.set_bind_group(1, pipeline.tree_bind_group.as_ref().unwrap(), &[]);
            let pipeline = pipeline_cache
                .get_compute_pipeline(pipeline.update_pipeline)
                .unwrap();
            pass.set_pipeline(pipeline);
            pass.dispatch_workgroups(
                self.resolution[0] / WORKGROUP_SIZE,
                self.resolution[1] / WORKGROUP_SIZE,
                1,
            );
        }
        Ok(())
    }
}
