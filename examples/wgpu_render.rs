use shocovox_rs::octree::raytracing::wgpu::SvxRenderApp;
use shocovox_rs::octree::Albedo;
use shocovox_rs::octree::V3c;
use std::sync::Arc;
use winit::event_loop::{ControlFlow, EventLoop};

#[cfg(feature = "wgpu")]
const DISPLAY_RESOLUTION: [u32; 2] = [1024, 768];

#[cfg(feature = "wgpu")]
const ARRAY_DIMENSION: u32 = 128;

#[cfg(feature = "wgpu")]
fn main() {
    let event_loop = EventLoop::new().unwrap();
    env_logger::init();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = SvxRenderApp::new(DISPLAY_RESOLUTION[0], DISPLAY_RESOLUTION[1]);

    // fill octree with data
    let mut tree = shocovox_rs::octree::Octree::<Albedo, 16>::new(ARRAY_DIMENSION)
        .ok()
        .unwrap();

    tree.insert(&V3c::new(1, 3, 3), Albedo::from(0x66FFFF))
        .ok()
        .unwrap();
    for x in 0..ARRAY_DIMENSION {
        for y in 0..ARRAY_DIMENSION {
            for z in 0..ARRAY_DIMENSION {
                if ((x < (ARRAY_DIMENSION / 4)
                    || y < (ARRAY_DIMENSION / 4)
                    || z < (ARRAY_DIMENSION / 4))
                    && (0 == x % 2 && 0 == y % 4 && 0 == z % 2))
                    || ((ARRAY_DIMENSION / 2) <= x
                        && (ARRAY_DIMENSION / 2) <= y
                        && (ARRAY_DIMENSION / 2) <= z)
                {
                    let r = if 0 == x % (ARRAY_DIMENSION / 4) {
                        (x as f32 / ARRAY_DIMENSION as f32 * 255.) as u32
                    } else {
                        128
                    };
                    let g = if 0 == y % (ARRAY_DIMENSION / 4) {
                        (y as f32 / ARRAY_DIMENSION as f32 * 255.) as u32
                    } else {
                        128
                    };
                    let b = if 0 == z % (ARRAY_DIMENSION / 4) {
                        (z as f32 / ARRAY_DIMENSION as f32 * 255.) as u32
                    } else {
                        128
                    };
                    tree.insert(
                        &V3c::new(x, y, z),
                        Albedo::from(r | (g << 8) | (b << 16) | 0xFF000000),
                    )
                    .ok()
                    .unwrap();
                }
            }
        }
    }

    let showcase = Arc::new(tree);
    showcase.upload_to(&mut app);
    let _ = event_loop.run_app(&mut app);
}

#[cfg(not(feature = "wgpu"))]
fn main() {
    println!("You probably forgot to enable the wgpu feature!");
    //nothing to do when the feature is not enabled
}
