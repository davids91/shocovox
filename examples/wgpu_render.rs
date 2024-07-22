use shocovox_rs::octree::raytracing::wgpu::SvxRenderBackend;
use shocovox_rs::octree::raytracing::Viewport;
use shocovox_rs::octree::Albedo;
use shocovox_rs::octree::Octree;
use shocovox_rs::octree::V3c;
use shocovox_rs::octree::V3cf32;
use shocovox_rs::octree::VoxelData;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::ElementState;
use winit::event::MouseButton;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::NamedKey;
use winit::window::Window;
use winit::window::WindowId;

#[cfg(feature = "wgpu")]
const DISPLAY_RESOLUTION: [u32; 2] = [1024, 768];

#[cfg(feature = "wgpu")]
const OCTREE_SIZE: u32 = 128;

#[cfg(feature = "wgpu")]
const BRICK_DIMENSION: usize = 32;

#[cfg(feature = "wgpu")]
struct SvxRenderExample<T, const DIM: usize>
where
    T: Default + Clone + VoxelData,
{
    backend: SvxRenderBackend,
    window: Option<Arc<Window>>,
    tree: Arc<Octree<T, DIM>>,

    // User input variables
    last_cursor_pos: winit::dpi::PhysicalPosition<f64>,
    left_mouse_btn_pressed: bool,
}

#[cfg(feature = "wgpu")]
impl<T, const DIM: usize> ApplicationHandler for SvxRenderExample<T, DIM>
where
    T: Default + Clone + VoxelData,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window = Some(Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Voxel Raytracing Render")
                        .with_inner_size(winit::dpi::PhysicalSize::new(
                            self.backend.output_width(),
                            self.backend.output_height(),
                        )),
                )
                .unwrap(),
        ));
        futures::executor::block_on(
            self.backend
                .rebuild_pipeline(self.window.as_ref().unwrap().clone(), Some(&self.tree)),
        );
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.backend.execute_pipeline();
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::Resized(size) => {
                futures::executor::block_on(self.backend.set_output_size(
                    size.width,
                    size.height,
                    self.window.as_ref().unwrap().clone(),
                    Some(&self.tree),
                ));
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                if button == MouseButton::Left && state.is_pressed() {
                    self.left_mouse_btn_pressed = true;
                } else if button == MouseButton::Left && !state.is_pressed() {
                    self.left_mouse_btn_pressed = false;
                }
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                let delta_x = ((position.x - self.last_cursor_pos.x) as f32)
                    .min(100.)
                    .max(-100.);
                if self.left_mouse_btn_pressed {
                    self.backend.update_viewport_glass_fov(
                        self.backend.viewport().w_h_fov.z * (1. + delta_x / 100.),
                    );
                }
                self.last_cursor_pos = position;
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if let winit::keyboard::Key::Named(named) = event.logical_key {
                    match named {
                        NamedKey::ArrowUp => {
                            self.backend.update_viewport_origin(V3cf32::new(0., 5., 0.));
                        }
                        NamedKey::ArrowDown => {
                            self.backend
                                .update_viewport_origin(V3cf32::new(0., -5., 0.));
                        }
                        NamedKey::ArrowLeft => {
                            self.backend
                                .update_viewport_origin(V3cf32::new(-5., 0., 0.));
                        }
                        NamedKey::ArrowRight => {
                            self.backend.update_viewport_origin(V3cf32::new(5., 0., 0.));
                        }
                        NamedKey::PageUp => {
                            self.backend
                                .update_viewport_origin(self.backend.viewport().direction * 5.);
                        }
                        NamedKey::PageDown => {
                            self.backend
                                .update_viewport_origin(self.backend.viewport().direction * -5.);
                        }
                        _ => {}
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
    let mut tree = shocovox_rs::octree::Octree::<Albedo, BRICK_DIMENSION>::new(OCTREE_SIZE)
        .ok()
        .unwrap();

    // single color voxel
    // tree.insert_at_lod(
    //     &V3c::new(0, 0, 0),
    //     4,
    //     Albedo::default()
    //         .with_red(1.)
    //         .with_green(0.)
    //         .with_blue(1.)
    //         .with_alpha(1.),
    // )
    // .ok()
    // .unwrap();

    // Different color voxels
    // tree.insert(
    //     &V3c::new(0, 0, 0),
    //     Albedo::default()
    //         .with_red(0.)
    //         .with_green(0.)
    //         .with_blue(0.)
    //         .with_alpha(1.),
    // )
    // .ok()
    // .unwrap();

    // tree.insert(
    //     &V3c::new(0, 0, 1),
    //     Albedo::default()
    //         .with_red(0.)
    //         .with_green(0.)
    //         .with_blue(1.)
    //         .with_alpha(1.),
    // )
    // .ok()
    // .unwrap();

    // tree.insert(
    //     &V3c::new(0, 1, 0),
    //     Albedo::default()
    //         .with_red(0.)
    //         .with_green(1.)
    //         .with_blue(0.)
    //         .with_alpha(1.),
    // )
    // .ok()
    // .unwrap();

    // tree.insert(
    //     &V3c::new(0, 1, 1),
    //     Albedo::default()
    //         .with_red(0.)
    //         .with_green(1.)
    //         .with_blue(1.)
    //         .with_alpha(1.),
    // )
    // .ok()
    // .unwrap();

    // tree.insert(
    //     &V3c::new(1, 0, 0),
    //     Albedo::default()
    //         .with_red(1.)
    //         .with_green(0.)
    //         .with_blue(0.)
    //         .with_alpha(1.),
    // )
    // .ok()
    // .unwrap();

    // tree.insert(
    //     &V3c::new(1, 0, 1),
    //     Albedo::default()
    //         .with_red(1.)
    //         .with_green(0.)
    //         .with_blue(1.)
    //         .with_alpha(1.),
    // )
    // .ok()
    // .unwrap();

    // tree.insert(
    //     &V3c::new(1, 1, 0),
    //     Albedo::default()
    //         .with_red(1.)
    //         .with_green(1.)
    //         .with_blue(0.)
    //         .with_alpha(1.),
    // )
    // .ok()
    // .unwrap();

    // tree.insert(
    //     &V3c::new(1, 1, 1),
    //     Albedo::default()
    //         .with_red(1.)
    //         .with_green(1.)
    //         .with_blue(1.)
    //         .with_alpha(1.),
    // )
    // .ok()
    // .unwrap();

    // assert!(
    //     Albedo::default()
    //         .with_red(1.)
    //         .with_green(0.)
    //         .with_blue(0.)
    //         .with_alpha(1.)
    //         == *tree.get(&V3c::new(1, 0, 0)).unwrap()
    // );

    tree.insert(&V3c::new(1, 3, 3), Albedo::from(0x66FFFF))
        .ok()
        .unwrap();
    for x in 0..OCTREE_SIZE {
        for y in 0..OCTREE_SIZE {
            for z in 0..OCTREE_SIZE {
                if ((x < (OCTREE_SIZE / 4) || y < (OCTREE_SIZE / 4) || z < (OCTREE_SIZE / 4))
                    && (0 == x % 2 && 0 == y % 4 && 0 == z % 2))
                    || ((OCTREE_SIZE / 2) <= x && (OCTREE_SIZE / 2) <= y && (OCTREE_SIZE / 2) <= z)
                {
                    let r = if 0 == x % (OCTREE_SIZE / 4) {
                        x as f32 / OCTREE_SIZE as f32
                    } else {
                        0.5
                    };
                    let g = if 0 == y % (OCTREE_SIZE / 4) {
                        y as f32 / OCTREE_SIZE as f32
                    } else {
                        0.5
                    };
                    let b = if 0 == z % (OCTREE_SIZE / 4) {
                        z as f32 / OCTREE_SIZE as f32
                    } else {
                        0.5
                    };
                    tree.insert(
                        &V3c::new(x, y, z),
                        Albedo::default().with_red(r).with_green(g).with_blue(b),
                    )
                    .ok()
                    .unwrap();
                }
            }
        }
    }
    let showcase = Arc::new(tree);

    // Fire up the display
    let origin = V3c::new(
        OCTREE_SIZE as f32 * 2.,
        OCTREE_SIZE as f32 / 2.,
        OCTREE_SIZE as f32 * -2.,
    );

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let backend = SvxRenderBackend::new(DISPLAY_RESOLUTION[0], DISPLAY_RESOLUTION[1])
        .with_viewport(Viewport {
            direction: (V3c::new(0., 0., 0.) - origin).normalized(),
            origin,
            w_h_fov: V3c::new(10., 10., 3.5),
        });
    // .with_viewport(Viewport {
    //     direction: V3c::new(1., 0., 0.),
    //     origin: V3c::new(0., 1., 0.),
    //     w_h_fov: V3c::new(0., 0., 1.),
    // });

    let mut example = SvxRenderExample {
        backend,
        window: None,
        tree: showcase.clone(),
        last_cursor_pos: Default::default(),
        left_mouse_btn_pressed: false,
    };

    env_logger::init();
    let _ = event_loop.run_app(&mut example);
}

#[cfg(not(feature = "wgpu"))]
fn main() {
    println!("You probably forgot to enable the wgpu feature!");
    //nothing to do when the feature is not enabled
}
