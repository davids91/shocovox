pub mod raytracing_on_cpu;
mod tests;

#[cfg(feature = "bevy_wgpu")]
pub mod bevy;

#[cfg(feature = "wgpu")]
pub mod wgpu;

pub use crate::spatial::raytracing::Ray;

#[cfg(feature = "bevy_wgpu")]
pub use bevy::types::{ShocoVoxRenderPlugin, ShocoVoxViewingGlass, Viewport};
