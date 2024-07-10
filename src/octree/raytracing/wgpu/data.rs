use encase::StorageBuffer;
use std::num::NonZero;

use crate::octree::{
    empty_marker, raytracing::wgpu::types::Voxelement, types::NodeChildrenArray, Albedo,
    NodeContent, Octree, VoxelData,
};
use wgpu::util::DeviceExt;

use super::{
    types::{OctreeMetaData, SizedNode},
    SvxRenderApp,
};

impl<T, const DIM: usize> From<&Octree<T, DIM>> for OctreeMetaData
where
    T: Default + Clone + VoxelData,
{
    fn from(tree: &Octree<T, DIM>) -> Self {
        OctreeMetaData {
            octree_size: tree.octree_size,
            voxel_brick_dim: DIM as u32,
            ambient_light_color: [1., 1., 1.].into(),
            ambient_light_position: [DIM as f32, DIM as f32, DIM as f32].into(),
        }
    }
}

impl SvxRenderApp {
    //    fn
}

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

    fn create_meta(&self, node_key: usize) -> u32 {
        let node = self.nodes.get(node_key);
        let mut meta = 0;
        match node {
            NodeContent::Leaf(_) => {
                Self::meta_set_is_leaf(&mut meta, true);
                Self::meta_set_node_occupancy_bitmap(
                    &mut meta,
                    self.occupied_8bit(node_key as u32),
                );
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

    pub(crate) fn upload_to(&self, app: &mut SvxRenderApp) {
        // parse octree
        let mut nodes = Vec::new();
        let mut children = Vec::new();
        let mut voxels = Vec::new();

        for i in 0..self.nodes.len() {
            if !self.nodes.key_is_valid(i) {
                continue;
            }
            let mut sized_node = SizedNode {
                sized_node_meta: self.create_meta(i),
                children_start_at: children.len() as u32,
                voxels_start_at: empty_marker(),
            };
            if let NodeContent::Leaf(data) = self.nodes.get(i) {
                debug_assert!(matches!(
                    self.node_children[i].content,
                    NodeChildrenArray::OccupancyBitmap(_)
                ));
                let occupied_bits = match self.node_children[i].content {
                    NodeChildrenArray::OccupancyBitmap(bitmap) => bitmap,
                    _ => panic!("Found Leaf Node without occupancy bitmap!"),
                };
                children.extend_from_slice(&[
                    (occupied_bits & 0x00000000FFFFFFFF) as u32,
                    ((occupied_bits & 0xFFFFFFFF00000000) >> 32) as u32,
                ]);
                sized_node.voxels_start_at = voxels.len() as u32;
                for z in 0..DIM {
                    for y in 0..DIM {
                        for x in 0..DIM {
                            let albedo = data[x][y][z].albedo();
                            let content = data[x][y][z].user_data();
                            voxels.push(Voxelement { albedo, content })
                        }
                    }
                }
            } else {
                //Internal nodes
                children.extend_from_slice(&self.node_children[i].get_full());
            }
            nodes.push(sized_node);
        }

        debug_assert!(0 < nodes.len());
        debug_assert!(0 < children.len());
        debug_assert!(0 < voxels.len());

        // Create bind group layout
        let layout = app
            .device
            .as_ref()
            .expect("Expected Render Device")
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    // metadata
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // nodes
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: Some(NonZero::new(nodes.len() as u32).unwrap()),
                    },
                    // children
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: Some(NonZero::new(children.len() as u32).unwrap()),
                    },
                    // voxels
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: Some(NonZero::new(voxels.len() as u32).unwrap()),
                    },
                ],
                label: Some("Octree_Layout"),
            });

        // Upload data to buffers
        let octree_meta = OctreeMetaData::from(self);
        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&octree_meta).unwrap();
        let metadata_buffer = app
            .device
            .as_ref()
            .expect("Expected SvxRenderApp to have a vaild device!")
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Octree Metadata Buffer"),
                contents: &buffer.into_inner(),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&nodes).unwrap();
        let nodes_buffer = app
            .device
            .as_ref()
            .expect("Expected SvxRenderApp to have a vaild device!")
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Octree Metadata Buffer"),
                contents: &buffer.into_inner(),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&children).unwrap();
        let children_buffer = app
            .device
            .as_ref()
            .expect("Expected SvxRenderApp to have a vaild device!")
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Octree Metadata Buffer"),
                contents: &buffer.into_inner(),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&voxels).unwrap();
        let voxels_buffer = app
            .device
            .as_ref()
            .expect("Expected SvxRenderApp to have a vaild device!")
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Octree Metadata Buffer"),
                contents: &buffer.into_inner(),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create bind group
        let group = app
            .device
            .as_ref()
            .expect("Expected SvxRenderApp to have a vaild device!")
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &layout,
                entries: &[
                    // wgpu::BindGroupEntry {
                    //     binding: 0,
                    //     resource: ??.as_entire_binding(),
                    // },
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: metadata_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: nodes_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: children_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: voxels_buffer.as_entire_binding(),
                    },
                ],
                label: Some("camera_bind_group"),
            });
        let render_pipeline_layout = app
            .device
            .as_ref()
            .expect("Expected SvxRenderApp to have a vaild device!")
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&layout],
                push_constant_ranges: &[],
            });
    }
}
