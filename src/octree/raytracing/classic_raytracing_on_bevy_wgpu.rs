use crate::octree::NodeContent;

use bevy::ecs::system::Resource;
use bevy::math::{Vec2, Vec3};
use bevy::pbr::Material;
use bevy::render::color::Color;
use bevy::render::render_resource::{ShaderRef, ShaderType};
use bevy::{
    reflect::{TypePath, TypeUuid},
    render::render_resource::AsBindGroup,
};

#[derive(Clone, ShaderType)]
struct SizedNode {
    contains_nodes: u32, // it is a leaf if it contains 1 node and has no children
    albedo: Color,
    content: u32,
    children: [u32; 8],
}

#[derive(Clone, ShaderType)]
struct OctreeMetaData {
    root_size: u32,
    ambient_light_color: Color,
    ambient_light_position: Vec3,
}

#[derive(Clone, Copy, ShaderType)]
pub struct Viewport {
    pub origin: Vec3,
    pub direction: Vec3,
    pub size: Vec2,
    pub fov: f32,
}

#[derive(Resource, Clone, AsBindGroup, TypeUuid, TypePath)]
#[uuid = "9c5a0ddf-1eaf-41b4-9832-ed736fd26af3"]
pub struct OctreeViewMaterial {
    #[uniform(0)]
    pub viewport: Viewport,

    #[uniform(1)]
    meta: OctreeMetaData,

    #[storage(2)]
    nodes: Vec<SizedNode>,
}

impl Material for OctreeViewMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/viewport_render.wgsl".into()
    }
}

use crate::octree::{Octree, VoxelData};
impl<T: Default + Clone + VoxelData> Octree<T> {
    pub fn create_bevy_material_view(&self, viewport: &Viewport) -> OctreeViewMaterial {
        let meta = OctreeMetaData {
            root_size: self.root_size,
            ambient_light_color: Color::rgba(1., 1., 1., 1.),
            ambient_light_position: Vec3::new(
                self.root_size as f32,
                self.root_size as f32,
                self.root_size as f32,
            ),
        };
        let mut nodes = Vec::new();
        for i in 0..self.nodes.len() {
            match self.nodes.get(i) {
                NodeContent::Leaf(data) => {
                    let albedo = data.albedo();
                    nodes.push(SizedNode {
                        contains_nodes: 1,
                        albedo: Color::rgb(
                            albedo[0] as f32 / 255.,
                            albedo[1] as f32 / 255.,
                            albedo[2] as f32 / 255.,
                        ),
                        content: data.user_data().unwrap_or(0),
                        children: self.node_children[i].get_full(),
                    });
                }
                NodeContent::Internal(count) => {
                    nodes.push(SizedNode {
                        contains_nodes: *count,
                        albedo: Color::rgba(0., 0., 0., 0.),
                        content: 0,
                        children: self.node_children[i].get_full(),
                    });
                }
                NodeContent::Nothing => {
                    nodes.push(SizedNode {
                        contains_nodes: 0,
                        albedo: Color::rgba(0., 0., 0., 0.),
                        content: 0,
                        children: self.node_children[i].get_full(),
                    });
                }
            }
        }
        OctreeViewMaterial {
            viewport: *viewport,
            meta,
            nodes,
        }
    }
}
