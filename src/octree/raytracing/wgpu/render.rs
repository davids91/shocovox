pub use crate::octree::raytracing::wgpu::types::SvxRenderBackend;

impl SvxRenderBackend {
    pub fn execute_pipeline(&self) {
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

        if let Some(tree_group) = &self.tree_group {
            // Raytracing
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("raytracing_compute_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(
                self.compute_pipeline
                    .as_ref()
                    .expect("Expected Raytracing pipeline to be valid"),
            );
            compute_pass.set_bind_group(
                0,
                &self
                    .dynamic_group
                    .as_ref()
                    .expect("Expected Dynamic Bind Group"),
                &[],
            );
            compute_pass.set_bind_group(1, tree_group, &[]);
            compute_pass.dispatch_workgroups(self.output_width / 8, self.output_height / 8, 1);
        }

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
            // Rendering
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
            render_pass.set_pipeline(
                &self
                    .render_pipeline
                    .as_ref()
                    .expect("Expected Rendering pipeline to be valid"),
            );
            render_pass.set_bind_group(
                0,
                &self
                    .dynamic_group
                    .as_ref()
                    .expect("Expected Dynamic Bind Group"),
                &[],
            );
            if let Some(tree_group) = &self.tree_group {
                render_pass.set_bind_group(1, tree_group, &[]);
            }
            render_pass.draw(0..6, 0..1);
        }

        self.queue
            .as_ref()
            .expect("Render Queue not available")
            .submit(Some(encoder.finish()));
        frame.present();
    }
}
