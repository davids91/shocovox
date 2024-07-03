use std::{borrow::Cow, sync::Arc};
use wgpu::{Adapter, Device, Queue, RenderPipeline, Surface};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

pub struct SvxRenderApp {
    output_width: u32,
    output_height: u32,
    wgpu_instance: wgpu::Instance,
    adapter: Option<Adapter>,
    window: Option<Arc<Window>>,
    surface: Option<Surface<'static>>,
    device: Option<Device>,
    pipeline: Option<RenderPipeline>,
    queue: Option<Queue>,
}

impl SvxRenderApp {
    async fn rebuild_pipeline(&mut self) {
        assert!(self.window.is_some());
        let surface = self
            .wgpu_instance
            .create_surface(self.window.as_ref().unwrap().clone())
            .unwrap();

        let adapter = self
            .wgpu_instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                // Request an adapter which can render to our surface
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .expect("Failed to create device");
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("raytracing.wgsl"))),
        });

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];
        self.pipeline = Some(
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    compilation_options: Default::default(),
                    targets: &[Some(swapchain_format.into())],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            }),
        );

        let config = surface
            .get_default_config(&adapter, self.output_width, self.output_height)
            .unwrap();
        surface.configure(&device, &config);

        self.adapter = Some(adapter);
        self.surface = Some(surface);
        self.queue = Some(queue);
        self.device = Some(device);
    }

    pub fn new(output_width: u32, output_height: u32) -> Self {
        let wgpu_instance = wgpu::Instance::default();
        // for adapter in wgpu_instance.enumerate_adapters(wgpu::Backends::all()) {
        //     println!("{:?}", adapter.get_info())
        // }

        Self {
            wgpu_instance,
            output_width,
            output_height,
            window: None,
            adapter: None,
            surface: None,
            device: None,
            pipeline: None,
            queue: None,
        }
    }
}

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
                if self.window.is_none() {
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
                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                    rpass.set_pipeline(&self.pipeline.as_ref().unwrap());
                    rpass.draw(0..6, 0..1);
                }

                self.queue
                    .as_ref()
                    .expect("Render Queue not available")
                    .submit(Some(encoder.finish()));
                frame.present();

                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::Resized(size) => {
                self.output_width = size.width;
                self.output_height = size.height;
                futures::executor::block_on(self.rebuild_pipeline())
            }
            _ => (),
        }
    }
}
