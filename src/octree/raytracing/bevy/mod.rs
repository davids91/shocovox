mod data;
mod pipeline;
pub mod types;

pub use crate::octree::raytracing::bevy::types::{
    ShocoVoxRenderPlugin, ShocoVoxViewingGlass, Viewport,
};

use crate::octree::raytracing::bevy::{
    data::{handle_cache, handle_gpu_readback, sync_with_main_world},
    pipeline::prepare_bind_groups,
    types::{ShocoVoxLabel, ShocoVoxRenderData, ShocoVoxRenderNode, ShocoVoxRenderPipeline},
};

use bevy::{
    app::{App, Plugin},
    asset::{Assets, Handle},
    ecs::system::ResMut,
    prelude::{ExtractSchedule, IntoSystemConfigs},
    render::{
        extract_resource::ExtractResourcePlugin,
        prelude::Image,
        render_asset::RenderAssetUsages,
        render_graph::RenderGraph,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        Render, RenderApp, RenderSet,
    },
};

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
        render_app.add_systems(ExtractSchedule, sync_with_main_world);
        render_app.add_systems(
            Render,
            (
                handle_cache.in_set(RenderSet::PrepareResources),
                prepare_bind_groups.in_set(RenderSet::PrepareBindGroups),
                handle_gpu_readback.in_set(RenderSet::Cleanup),
            ),
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
