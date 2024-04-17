#[cfg(feature = "raytracing")]
pub mod raytracing_on_cpu;

#[cfg(feature = "bevy_wgpu")]
pub mod classic_raytracing_on_bevy_wgpu;

mod tests;