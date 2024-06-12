use crate::object_pool::empty_marker;
use crate::octree::{
    raytracing::types::{OctreeMetaData, ShocoVoxViewingGlass, SizedNode, Viewport, Voxelement},
    types::{NodeChildrenArray, NodeContent},
};

use bevy::{
    app::{App, Plugin},
    asset::{AssetServer, Assets},
    ecs::system::{Commands, Res, ResMut, Resource},
    ecs::world::{FromWorld, World},
    math::Vec3,
    prelude::IntoSystemConfigs,
    render::{
        color::Color,
        extract_resource::ExtractResourcePlugin,
        render_asset::{RenderAssetUsages, RenderAssets},
        render_graph,
        render_graph::{RenderGraph, RenderLabel},
        render_resource::{
            AsBindGroup, BindGroup, BindGroupLayout, CachedComputePipelineId, CachedPipelineState,
            ComputePassDescriptor, ComputePipelineDescriptor, Extent3d, PipelineCache,
            TextureDimension, TextureFormat, TextureUsages,
        },
        renderer::{RenderContext, RenderDevice},
        texture::{FallbackImage, Image},
        Render, RenderApp, RenderSet,
    },
};
use std::borrow::Cow;

const OUTPUT_DISPLAY_SIZE: (u32, u32) = (640, 480);
const WORKGROUP_SIZE: u32 = 8;

#[derive(Resource)]
struct ShocoVoxPipelineBindGroup(BindGroup);

#[derive(Resource)]
struct ShocoVoxRenderPipeline {
    viewing_glass_bind_group_layout: BindGroupLayout,
    update_pipeline: CachedComputePipelineId,
}

impl FromWorld for ShocoVoxRenderPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let viewing_glass_bind_group_layout =
            ShocoVoxViewingGlass::bind_group_layout(render_device);
        let shader = world
            .resource::<AssetServer>()
            .load("shaders/viewport_render.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![viewing_glass_bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader,
            shader_defs: vec![],
            entry_point: Cow::from("update"),
        });

        ShocoVoxRenderPipeline {
            viewing_glass_bind_group_layout,
            update_pipeline,
        }
    }
}

pub struct ShocoVoxRenderPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct ShocoVoxLabel;

fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<ShocoVoxRenderPipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
    octree_viewing_glass: Res<ShocoVoxViewingGlass>,
    render_device: Res<RenderDevice>,
) {
    let bind_group = octree_viewing_glass
        .as_bind_group(
            &pipeline.viewing_glass_bind_group_layout,
            &render_device,
            &gpu_images,
            &fallback_image,
        )
        .ok()
        .unwrap();
    commands.insert_resource(ShocoVoxPipelineBindGroup(bind_group.bind_group));
}

impl Plugin for ShocoVoxRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<ShocoVoxViewingGlass>::default());
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
        );

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node(ShocoVoxLabel, ShocoVoxRenderNode::default());
        render_graph.add_node_edge(ShocoVoxLabel, bevy::render::graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<ShocoVoxRenderPipeline>();
    }
}

impl Default for ShocoVoxRenderNode {
    fn default() -> Self {
        Self { ready: false }
    }
}

struct ShocoVoxRenderNode {
    ready: bool,
}

impl render_graph::Node for ShocoVoxRenderNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<ShocoVoxRenderPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        // if the corresponding pipeline has loaded, transition to the next stage
        if !self.ready {
            if let CachedPipelineState::Ok(_) =
                pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline)
            {
                self.ready = true;
            }
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let pipeline_bind_group = &world.resource::<ShocoVoxPipelineBindGroup>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<ShocoVoxRenderPipeline>();

        if self.ready {
            let mut pass = render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor::default());

            pass.set_bind_group(0, pipeline_bind_group, &[]);
            let pipeline = pipeline_cache
                .get_compute_pipeline(pipeline.update_pipeline)
                .unwrap();
            pass.set_pipeline(pipeline);
            pass.dispatch_workgroups(
                OUTPUT_DISPLAY_SIZE.0 / WORKGROUP_SIZE,
                OUTPUT_DISPLAY_SIZE.1 / WORKGROUP_SIZE,
                1,
            );
        }
        Ok(())
    }
}

use crate::octree::{Octree, VoxelData};
impl<T, const DIM: usize> Octree<T, DIM>
where
    T: Default + Clone + VoxelData,
{
    fn meta_set_is_leaf(sized_node_meta: &mut u32, is_leaf: bool) {
        *sized_node_meta =
            (*sized_node_meta & 0x00FFFFFF) | if is_leaf { 0x01000000 } else { 0x00000000 };
    }

    fn meta_set_node_occupancy_bitmap(sized_node_meta: &mut u32, bitmap: u8) {
        *sized_node_meta = (*sized_node_meta & 0xFFFFFF00) | bitmap as u32;
    }

    pub(in crate::octree) fn meta_set_leaf_occupancy_bitmap(
        bitmap_target: &mut [u32; 8],
        source: u64,
    ) {
        bitmap_target[0] = (source & 0x00000000FFFFFFFF) as u32;
        bitmap_target[1] = ((source & 0xFFFFFFFF00000000) >> 32) as u32;
    }

    fn create_meta(&self, node_key: usize) -> u32 {
        let node = self.nodes.get(node_key);
        let mut meta = 0;
        match node {
            NodeContent::Leaf(_) => {
                Self::meta_set_is_leaf(&mut meta, true);
                Self::meta_set_node_occupancy_bitmap(&mut meta, 0xFF);
            }
            NodeContent::Internal(occupied_bits) => {
                Self::meta_set_is_leaf(&mut meta, false);
                Self::meta_set_node_occupancy_bitmap(&mut meta, *occupied_bits);
            }
            _ => {
                Self::meta_set_is_leaf(&mut meta, false);
                Self::meta_set_node_occupancy_bitmap(&mut meta, 0x00);
            }
        };
        meta
    }

    pub fn create_bevy_view(
        &self,
        viewport: &Viewport,
        mut images: ResMut<Assets<Image>>,
    ) -> ShocoVoxViewingGlass {
        let meta = OctreeMetaData {
            octree_size: self.octree_size,
            voxel_matrix_dim: DIM as u32,
            ambient_light_color: Color::rgba(1., 1., 1., 1.),
            ambient_light_position: Vec3::new(
                self.octree_size as f32,
                self.octree_size as f32,
                self.octree_size as f32,
            ),
        };
        let mut nodes = Vec::new();
        let mut voxels = Vec::new();
        for i in 0..self.nodes.len() {
            if !self.nodes.key_is_valid(i) {
                continue;
            }
            let mut sized_node = SizedNode {
                sized_node_meta: self.create_meta(i),
                children: self.node_children[i].get_full(),
                voxels_start_at: empty_marker(),
            };
            if let NodeContent::Leaf(data) = self.nodes.get(i) {
                debug_assert!(matches!(
                    self.node_children[i].content,
                    NodeChildrenArray::OccupancyBitmap(_)
                ));
                Self::meta_set_leaf_occupancy_bitmap(
                    &mut sized_node.children,
                    match self.node_children[i].content {
                        NodeChildrenArray::OccupancyBitmap(bitmap) => bitmap,
                        _ => panic!("Found Leaf Node without occupancy bitmap!"),
                    },
                );
                sized_node.voxels_start_at = voxels.len() as u32;
                for z in 0..DIM {
                    for y in 0..DIM {
                        for x in 0..DIM {
                            let albedo = data[x][y][z].albedo();
                            let content = data[x][y][z].user_data();
                            voxels.push(Voxelement {
                                albedo: Color::rgba(
                                    albedo[0] as f32 / 255.,
                                    albedo[1] as f32 / 255.,
                                    albedo[2] as f32 / 255.,
                                    albedo[3] as f32 / 255.,
                                ),
                                content,
                            })
                        }
                    }
                }
            }
            nodes.push(sized_node);
        }
        let mut output_texture = Image::new_fill(
            Extent3d {
                width: OUTPUT_DISPLAY_SIZE.0,
                height: OUTPUT_DISPLAY_SIZE.1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        );
        output_texture.texture_descriptor.usage = TextureUsages::COPY_DST
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING;
        let output_texture = images.add(output_texture);

        ShocoVoxViewingGlass {
            output_texture,
            viewport: *viewport,
            meta,
            nodes,
            voxels,
        }
    }
}
