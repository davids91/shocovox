use crate::octree::raytracing::wgpu::SvxRenderApp;
use std::borrow::Cow;
use wgpu::{util::DeviceExt, TextureUsages};

impl SvxRenderApp {
    pub(crate) async fn rebuild_pipeline(&mut self) {
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
                    required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                    // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                    required_limits: wgpu::Limits::downlevel_defaults()
                        .using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        // Create output texture and fill it with a single color
        let output_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: self.texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: TextureUsages::STORAGE_BINDING
                | TextureUsages::COPY_SRC
                | TextureUsages::COPY_DST,
            label: Some("output_texture"),
            view_formats: &[],
        });
        let output_texture_render = device.create_texture(&wgpu::TextureDescriptor {
            size: self.texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            label: Some("output_texture"),
            view_formats: &[],
        });
        let mut default_image = image::DynamicImage::ImageRgba8(image::RgbaImage::new(
            self.output_width,
            self.output_height,
        ));

        for x in 0..self.output_width {
            for y in 0..self.output_height {
                use image::GenericImage;
                if 0 == x % 50 || 0 == y % 50 {
                    default_image.put_pixel(x, y, image::Rgba([200, 255, 200, 255]));
                } else {
                    default_image.put_pixel(
                        x,
                        y,
                        image::Rgba([
                            (255 * x / self.output_width) as u8,
                            255,
                            (255 * y / self.output_width) as u8,
                            255,
                        ]),
                    );
                }
            }
        }

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &output_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &default_image.to_rgba8(),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * self.output_width),
                rows_per_image: Some(self.output_height),
            },
            self.texture_extent,
        );

        // create texture view and sampler
        let output_texture_view =
            output_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let output_texture_render_view =
            output_texture_render.create_view(&wgpu::TextureViewDescriptor::default());
        let output_texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // re-create viewport
        let viewport_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Octree Metadata Buffer"),
            contents: bytemuck::cast_slice(&[self.viewport]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                // output_texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::ReadWrite,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                // output_texture_render
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                // output_texture sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // viewport
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("Dynamic_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&output_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&output_texture_render_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&output_texture_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: viewport_buffer.as_entire_binding(),
                },
            ],
            label: Some("Dynamic_bind_group"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&layout],
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

        // Set object state
        self.adapter = Some(adapter);
        self.surface = Some(surface);
        self.queue = Some(queue);
        self.device = Some(device);
        self.dynamic_group = Some(bind_group);
        self.output_texture = Some(output_texture);
        self.output_texture_render = Some(output_texture_render);
    }
}
