pub mod raytracing_on_cpu;
mod tests;

#[cfg(feature = "bevy_wgpu")]
pub mod bevy;

pub use crate::spatial::raytracing::Ray;

#[cfg(feature = "bevy_wgpu")]
pub use bevy::types::{
    OctreeGPUHost, OctreeGPUView, OctreeRenderData, OctreeSpyGlass, SvxRenderPlugin, Viewport,
};
