use crate::octree::raytracing::wgpu::SvxRenderBackend;
use crate::octree::Octree;
use crate::octree::VoxelData;
use encase::UniformBuffer;
use std::borrow::Cow;
use std::sync::Arc;
use wgpu::Features;
use wgpu::{util::DeviceExt, TextureUsages};
use winit::window::Window;

impl SvxRenderBackend {
    pub async fn rebuild_pipeline<T, const DIM: usize>(
        &mut self,
        window: Arc<Window>,
        tree: Option<&Octree<T, DIM>>,
    ) where
        T: Default + Clone + VoxelData,
    {
        //Request WGPU backend
        let surface = self.wgpu_instance.create_surface(window).unwrap();
        let adapter = self
            .wgpu_instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("svc_requested_device"),
                    required_features: Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                        | Features::BUFFER_BINDING_ARRAY
                        | Features::STORAGE_RESOURCE_BINDING_ARRAY,
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
            label: Some("output_texture_render"),
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
        let mut buffer = UniformBuffer::new(Vec::<u8>::new());
        buffer.write(&self.viewport).unwrap();
        let viewport_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Octree Metadata Buffer"),
            contents: &buffer.into_inner(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        self.viewport_buffer = Some(viewport_buffer);

        // Configuring surface and setting object state
        let config = surface
            .get_default_config(&adapter, self.output_width, self.output_height)
            .unwrap();
        surface.configure(&device, &config);

        self.adapter = Some(adapter);
        self.surface = Some(surface);
        self.queue = Some(queue);
        self.device = Some(device);
        self.output_texture = Some(output_texture);
        self.output_texture_render = Some(output_texture_render);

        // Create dynamic group
        let dynamic_group_layout = self.device.as_ref().unwrap().create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
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
                        visibility: wgpu::ShaderStages::FRAGMENT,
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
                        visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("dynamic_bind_group_layout"),
            },
        );

        self.dynamic_group = Some(
            self.device
                .as_ref()
                .unwrap()
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &dynamic_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&output_texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(
                                &output_texture_render_view,
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&output_texture_sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: self
                                .viewport_buffer
                                .as_ref()
                                .expect("Expected SvxRenderBackend to have a valid Viewport Buffer")
                                .as_entire_binding(),
                        },
                    ],
                    label: Some("dynamic_bind_group"),
                }),
        );

        // create tree group
        let tree_group_layout;
        if let Some(tree) = tree {
            let (tree_bind_group_layout, tree_bind_group) = tree.upload_to(self);
            tree_group_layout = Some(tree_bind_group_layout);
            self.tree_group = Some(tree_bind_group);
        } else {
            tree_group_layout = None;
        }

        // create pipelines
        let compute_shader =
            self.device
                .as_ref()
                .unwrap()
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("svx_raytracing_shader"),
                    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                        "raytracing.wgsl"
                    ))),
                });
        let render_shader =
            self.device
                .as_ref()
                .unwrap()
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("svx_rendering_shader"),
                    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("rendering.wgsl"))),
                });

        let pipeline_layout;
        let debug_label;
        if tree_group_layout.is_some() {
            pipeline_layout = self.device.as_ref().unwrap().create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("svx_render_layout_with_tree"),
                    bind_group_layouts: &[
                        &dynamic_group_layout,
                        tree_group_layout.as_ref().unwrap(),
                    ],
                    push_constant_ranges: &[],
                },
            );
            self.compute_pipeline = Some(self.device.as_ref().unwrap().create_compute_pipeline(
                &wgpu::ComputePipelineDescriptor {
                    label: Some("svx_compute_layout_with_tree"),
                    layout: Some(&pipeline_layout),
                    module: &compute_shader,
                    entry_point: "update",
                    compilation_options: Default::default(),
                },
            ));
            debug_label = "_with_tree";
        } else {
            pipeline_layout = self.device.as_ref().unwrap().create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("svx_layout_wo_tree"),
                    bind_group_layouts: &[&dynamic_group_layout],
                    push_constant_ranges: &[],
                },
            );
            debug_label = "_wo_tree";
        };

        let swapchain_capabilities = self
            .surface
            .as_ref()
            .unwrap()
            .get_capabilities(self.adapter.as_ref().unwrap());
        let swapchain_format = swapchain_capabilities.formats[0];
        self.render_pipeline = Some(self.device.as_ref().unwrap().create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some(&("svx_render_layout".to_owned() + debug_label)),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &render_shader,
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &render_shader,
                    entry_point: "fs_main",
                    compilation_options: Default::default(),
                    targets: &[Some(swapchain_format.into())],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            },
        ));
    }
}
