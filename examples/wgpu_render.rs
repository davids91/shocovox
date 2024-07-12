use core::sync::atomic::Ordering;
use shocovox_rs::octree::raytracing::wgpu::SvxRenderApp;
use shocovox_rs::octree::Albedo;
use shocovox_rs::octree::V3c;
use shocovox_rs::octree::V3cf32;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;
use winit::window::WindowId;

#[cfg(feature = "wgpu")]
const DISPLAY_RESOLUTION: [u32; 2] = [1024, 768];

#[cfg(feature = "wgpu")]
const ARRAY_DIMENSION: u32 = 128;

#[cfg(feature = "wgpu")]
struct SvxRenderExample {
    app: SvxRenderApp,
    window: Option<Arc<Window>>,
}

#[cfg(feature = "wgpu")]
impl ApplicationHandler for SvxRenderExample {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window = Some(Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Voxel Raytracing Render")
                        .with_inner_size(winit::dpi::PhysicalSize::new(
                            self.app.output_width(),
                            self.app.output_height(),
                        )),
                )
                .unwrap(),
        ));
        futures::executor::block_on(
            self.app
                .rebuild_pipeline(self.window.as_ref().unwrap().clone()),
        )
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.app.execute_pipeline();
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::Resized(size) => {
                futures::executor::block_on(self.app.set_output_size(
                    size.width,
                    size.height,
                    self.window.as_ref().unwrap().clone(),
                ));
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if let winit::keyboard::Key::Named(named) = event.logical_key {
                    if matches!(named, winit::keyboard::NamedKey::ArrowUp) {
                        self.app.update_viewport_origin(V3cf32::new(0.002, 0., 0.));
                    }
                }
            }
            _ => (),
        }
    }
}

#[cfg(feature = "wgpu")]
fn main() {
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

    // Fire up the display
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let app = SvxRenderApp::new(DISPLAY_RESOLUTION[0], DISPLAY_RESOLUTION[1]);
    let mut example = SvxRenderExample { app, window: None };
    //showcase.upload_to(&mut app);

    env_logger::init();
    let _ = event_loop.run_app(&mut example);
}

#[cfg(not(feature = "wgpu"))]
fn main() {
    println!("You probably forgot to enable the wgpu feature!");
    //nothing to do when the feature is not enabled
}
