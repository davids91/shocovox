pub use crate::octree::raytracing::wgpu::types::SvxRenderApp;

use std::sync::{atomic::Ordering, Arc};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

impl ApplicationHandler for SvxRenderApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window = Some(Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Voxel Raytracing Render")
                        .with_inner_size(winit::dpi::PhysicalSize::new(
                            self.output_width,
                            self.output_height,
                        )),
                )
                .unwrap(),
        ));
        futures::executor::block_on(self.rebuild_pipeline())
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if !self.can_render.load(Ordering::Relaxed) {
                    return;
                }
                let mut size = self.window.as_ref().unwrap().inner_size();
                size.width = size.width.max(1);
                size.height = size.height.max(1);

                let frame = self
                    .surface
                    .as_ref()
                    .expect("Render Surface not available")
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture");

                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder = self
                    .device
                    .as_ref()
                    .expect("Device Encoder not found")
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                encoder.copy_texture_to_texture(
                    self.output_texture
                        .as_ref()
                        .expect("Expected Output texture")
                        .as_image_copy(),
                    self.output_texture_render
                        .as_ref()
                        .expect("Expected Output render texture")
                        .as_image_copy(),
                    self.texture_extent,
                );

                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None, //render target!
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    render_pass.set_pipeline(&self.pipeline.as_ref().unwrap());
                    render_pass.set_bind_group(
                        0,
                        &self
                            .dynamic_group
                            .as_ref()
                            .expect("Expected Dynamic Bind Group"),
                        &[],
                    );
                    render_pass.draw(0..6, 0..1);
                }

                self.queue
                    .as_ref()
                    .expect("Render Queue not available")
                    .submit(Some(encoder.finish()));
                frame.present();

                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::Resized(size) => {
                self.can_render.store(false, Ordering::Relaxed);
                self.output_width = size.width;
                self.output_height = size.height;
                futures::executor::block_on(self.rebuild_pipeline());
                self.can_render.store(true, Ordering::Relaxed);
            }
            _ => (),
        }
    }
}
