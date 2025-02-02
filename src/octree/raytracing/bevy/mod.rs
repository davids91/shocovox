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
    Albedo, VoxelData,
};
use bevy::{
    app::{App, Plugin},
    prelude::{ExtractSchedule, IntoSystemConfigs, Vec4},
    render::{
        extract_resource::ExtractResourcePlugin, render_graph::RenderGraph, Render, RenderApp,
        RenderSet,
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

impl<T> RenderBevyPlugin<T>
where
    T: Default + Clone + Eq + VoxelData + Send + Sync + 'static,
{
    pub fn new(resolution: [u32; 2]) -> Self {
        RenderBevyPlugin {
            dummy: std::marker::PhantomData,
            resolution,
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
                write_to_gpu::<T>.in_set(RenderSet::PrepareResources),
                prepare_bind_groups.in_set(RenderSet::PrepareBindGroups),
                handle_gpu_readback.in_set(RenderSet::Cleanup),
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
