mod cache;
mod data;
mod pipeline;
pub mod types;

pub use crate::raytracing::bevy::types::{
    OctreeGPUHost, OctreeGPUView, OctreeSpyGlass, RenderBevyPlugin, SvxViewSet, Viewport,
};
use crate::{
    octree::{Albedo, VoxelData},
    raytracing::bevy::{
        data::{handle_gpu_readback, sync_with_main_world, write_to_gpu},
        pipeline::prepare_bind_groups,
        types::{SvxLabel, SvxRenderNode, SvxRenderPipeline},
    },
};
use bevy::{
    app::{App, Plugin},
    asset::LoadState,
    prelude::{
        AssetServer, Assets, ExtractSchedule, Handle, Image, IntoSystemConfigs, Res, ResMut, Vec4,
    },
    render::{
        extract_resource::ExtractResourcePlugin,
        render_asset::RenderAssetUsages,
        render_graph::RenderGraph,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        Render, RenderApp, RenderSet,
    },
};
use std::hash::Hash;

impl From<Vec4> for Albedo {
    fn from(vec: Vec4) -> Self {
        Albedo::default()
            .with_red((vec.x * 255.).min(255.) as u8)
            .with_green((vec.y * 255.).min(255.) as u8)
            .with_blue((vec.z * 255.).min(255.) as u8)
            .with_alpha((vec.w * 255.).min(255.) as u8)
    }
}

impl From<Albedo> for Vec4 {
    fn from(color: Albedo) -> Self {
        Vec4::new(
            color.r as f32 / 255.,
            color.g as f32 / 255.,
            color.b as f32 / 255.,
            color.a as f32 / 255.,
        )
    }
}

impl OctreeGPUView {
    /// Erases the whole view to be uploaded to the GPU again
    pub fn reload(&mut self) {
        self.reload = true;
    }

    /// Provides the handle to the output texture
    /// Warning! Handle will no longer being updated after resolution change
    pub fn output_texture(&self) -> &Handle<Image> {
        &self.output_texture
    }

    /// Updates the resolution on which the view operates on.
    /// It will make a new output texture if size is larger, than the current output texture
    pub fn set_resolution(&mut self, resolution: [u32; 2]) {
        self.new_resolution = Some(resolution);
    }
}

impl OctreeSpyGlass {
    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    pub fn viewport_mut(&mut self) -> &mut Viewport {
        self.viewport_changed = true;
        &mut self.viewport
    }
}

impl<T> RenderBevyPlugin<T>
where
    T: Default + Clone + Eq + VoxelData + Send + Sync + 'static,
{
    pub fn new() -> Self {
        RenderBevyPlugin {
            dummy: std::marker::PhantomData,
        }
    }
}

pub(crate) fn handle_resolution_updates(
    viewset: Option<ResMut<SvxViewSet>>,
    images: Option<ResMut<Assets<Image>>>,
    server: Res<AssetServer>,
) {
    if let (Some(viewset), Some(mut images)) = (viewset, images) {
        let mut current_view = viewset.views[0].lock().unwrap();

        // check for resolution update requests
        if let Some(new_resolution) = current_view.new_resolution {
            // see if a new output texture is loaded for the requested resolution yet
            let mut replace_output_texture = false;
            if let Some(new_out_tex) = &current_view.new_output_texture {
                match server.get_load_state(new_out_tex) {
                    None | Some(LoadState::Loading) | Some(LoadState::NotLoaded) => {} // Still not ready
                    Some(LoadState::Failed(error)) => {
                        println!("Asset loading error: {:?}", error);
                    }
                    Some(LoadState::Loaded) => {
                        replace_output_texture = true;
                    }
                }
            } else {
                // not loaded yet! Insert the blank output texture
                let mut output_texture = Image::new_fill(
                    Extent3d {
                        width: new_resolution[0],
                        height: new_resolution[1],
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
                current_view.new_output_texture = Some(images.add(output_texture));
            }

            if replace_output_texture {
                current_view.resolution = new_resolution;
                current_view.output_texture =
                    current_view.new_output_texture.as_ref().unwrap().clone();
            }
        }
    }
}

impl<T> Plugin for RenderBevyPlugin<T>
where
    T: Default + Clone + Copy + Eq + Send + Sync + Hash + VoxelData + 'static,
{
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractResourcePlugin::<OctreeGPUHost<T>>::default(),
            ExtractResourcePlugin::<SvxViewSet>::default(),
        ));
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(ExtractSchedule, sync_with_main_world);
        render_app.add_systems(
            Render,
            (
                handle_resolution_updates.in_set(RenderSet::PrepareAssets),
                write_to_gpu::<T>.in_set(RenderSet::PrepareResources),
                prepare_bind_groups.in_set(RenderSet::PrepareBindGroups),
                handle_gpu_readback.in_set(RenderSet::Cleanup),
            ),
        );
        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(SvxLabel, SvxRenderNode { ready: false });
        render_graph.add_node_edge(SvxLabel, bevy::render::graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<SvxRenderPipeline>();
    }
}
