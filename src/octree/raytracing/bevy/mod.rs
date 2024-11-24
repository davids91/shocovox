mod cache;
mod data;
mod pipeline;
pub mod types;

pub use crate::octree::raytracing::bevy::types::{
    OctreeGPUHost, OctreeGPUView, OctreeSpyGlass, RenderBevyPlugin, SvxViewSet, Viewport,
};

use crate::octree::{
    raytracing::bevy::{
        data::{handle_gpu_readback, sync_with_main_world, write_to_gpu},
        pipeline::prepare_bind_groups,
        types::{SvxLabel, SvxRenderNode, SvxRenderPipeline},
    },
    VoxelData,
};

use bevy::{
    app::{App, Plugin},
    prelude::{ExtractSchedule, IntoSystemConfigs},
    render::{
        extract_resource::ExtractResourcePlugin, render_graph::RenderGraph, Render, RenderApp,
        RenderSet,
    },
};

impl<T, const DIM: usize> RenderBevyPlugin<T, DIM>
where
    T: Default + Clone + PartialEq + VoxelData + Send + Sync + 'static,
{
    pub fn new(resolution: [u32; 2]) -> Self {
        RenderBevyPlugin {
            dummy: std::marker::PhantomData,
            resolution,
        }
    }
}

impl<T, const DIM: usize> Plugin for RenderBevyPlugin<T, DIM>
where
    T: Default + Clone + PartialEq + VoxelData + Send + Sync + 'static,
{
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractResourcePlugin::<OctreeGPUHost<T, DIM>>::default(),
            ExtractResourcePlugin::<SvxViewSet>::default(),
        ));
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(ExtractSchedule, sync_with_main_world);
        render_app.add_systems(
            Render,
            (
                write_to_gpu::<T, DIM>.in_set(RenderSet::PrepareResources),
                prepare_bind_groups::<T, DIM>.in_set(RenderSet::PrepareBindGroups),
                handle_gpu_readback::<T, DIM>.in_set(RenderSet::Cleanup),
            ),
        );
        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(
            SvxLabel,
            SvxRenderNode {
                ready: false,
                resolution: self.resolution,
            },
        );
        render_graph.add_node_edge(SvxLabel, bevy::render::graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<SvxRenderPipeline>();
    }
}
