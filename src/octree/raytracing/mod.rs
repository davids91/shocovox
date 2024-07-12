pub mod raytracing_on_cpu;
mod tests;

pub use crate::spatial::raytracing::Ray;

#[cfg(feature = "bevy_wgpu")]
pub mod bevy;

#[cfg(feature = "bevy_wgpu")]
pub use bevy::types::{ShocoVoxRenderPlugin, ShocoVoxViewingGlass, Viewport};

#[cfg(feature = "wgpu")]
pub mod wgpu;

#[cfg(feature = "wgpu")]
pub use wgpu::types::{SvxRenderApp, Viewport};
