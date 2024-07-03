use shocovox_rs::octree::raytracing::wgpu::SvxRenderApp;
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    let event_loop = EventLoop::new().unwrap();
    env_logger::init();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = SvxRenderApp::new(1024, 768);
    let _ = event_loop.run_app(&mut app);
}
