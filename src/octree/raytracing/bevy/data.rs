use crate::object_pool::empty_marker;
use crate::spatial::math::flat_projection;
use crate::{
    octree::{
        raytracing::bevy::{
            create_output_texture,
            types::{
                OctreeGPUView, OctreeMetaData, ShocoVoxRenderData, ShocoVoxRenderPipeline,
                ShocoVoxViewingGlass, Viewport, Voxelement,
            },
        },
        types::{NodeChildrenArray, NodeContent},
        BrickData, Octree, V3c, VoxelData,
    },
    spatial::lut::BITMAP_MASK_FOR_OCTANT_LUT,
};
use bevy::{
    ecs::system::{Res, ResMut},
    math::Vec4,
    prelude::{Assets, Commands, Handle, Image},
    render::renderer::RenderDevice,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use super::types::{OctreeGPUDataHandler, VictimPointer};

impl<T, const DIM: usize> Octree<T, DIM>
where
    T: Default + Clone + Copy + PartialEq + VoxelData,
{
    //##############################################################################
    //     ███████      █████████  ███████████ ███████████   ██████████ ██████████
    //   ███░░░░░███   ███░░░░░███░█░░░███░░░█░░███░░░░░███ ░░███░░░░░█░░███░░░░░█
    //  ███     ░░███ ███     ░░░ ░   ░███  ░  ░███    ░███  ░███  █ ░  ░███  █ ░
    // ░███      ░███░███             ░███     ░██████████   ░██████    ░██████
    // ░███      ░███░███             ░███     ░███░░░░░███  ░███░░█    ░███░░█
    // ░░███     ███ ░░███     ███    ░███     ░███    ░███  ░███ ░   █ ░███ ░   █
    //  ░░░███████░   ░░█████████     █████    █████   █████ ██████████ ██████████
    //    ░░░░░░░      ░░░░░░░░░     ░░░░░    ░░░░░   ░░░░░ ░░░░░░░░░░ ░░░░░░░░░░
    //    █████████  ███████████  █████  █████
    //   ███░░░░░███░░███░░░░░███░░███  ░░███
    //  ███     ░░░  ░███    ░███ ░███   ░███
    // ░███          ░██████████  ░███   ░███
    // ░███    █████ ░███░░░░░░   ░███   ░███
    // ░░███  ░░███  ░███         ░███   ░███
    //  ░░█████████  █████        ░░████████
    //   ░░░░░░░░░  ░░░░░          ░░░░░░░░
    //  █████   █████ █████ ██████████ █████   ███   █████
    // ░░███   ░░███ ░░███ ░░███░░░░░█░░███   ░███  ░░███
    //  ░███    ░███  ░███  ░███  █ ░  ░███   ░███   ░███
    //  ░███    ░███  ░███  ░██████    ░███   ░███   ░███
    //  ░░███   ███   ░███  ░███░░█    ░░███  █████  ███
    //   ░░░█████░    ░███  ░███ ░   █  ░░░█████░█████░
    //     ░░███      █████ ██████████    ░░███ ░░███
    //##############################################################################

    /// Creates GPU compatible data renderable on the GPU from an octree
    pub fn create_new_gpu_view(
        &self,
        size: usize,
        viewport: Viewport,
        resolution: [u32; 2],
        commands: &mut Commands,
        images: ResMut<Assets<Image>>,
    ) -> Handle<Image> {
        let mut gpu_data_handler = OctreeGPUDataHandler {
            debug_gpu_interface: None,
            readable_debug_gpu_interface: None,
            render_data: ShocoVoxRenderData {
                debug_gpu_interface: 0,
                octree_meta: OctreeMetaData {
                    octree_size: self.octree_size,
                    voxel_brick_dim: DIM as u32,
                    ambient_light_color: V3c::new(1., 1., 1.),
                    ambient_light_position: V3c::new(
                        self.octree_size as f32,
                        self.octree_size as f32,
                        self.octree_size as f32,
                    ),
                },
                metadata: vec![0; size],
                node_ocbits: vec![0; size * 2],
                node_children: vec![empty_marker(); size * 8],
                color_palette: vec![Vec4::ZERO; u16::MAX as usize],
                voxels: vec![
                    Voxelement {
                        albedo_index: 0,
                        content: 0
                    };
                    size * 8 * (DIM * DIM * DIM)
                ],
            },
            victim_node: VictimPointer::new(size),
            victim_brick: VictimPointer::new(size),
            map_to_color_index_in_palette: HashMap::new(),
            map_to_node_index_in_metadata: HashMap::new(),
        };

        // Push root node and its contents
        gpu_data_handler.add_node(&self, Self::ROOT_NODE_KEY as usize, true);
        // +++ DEBUG +++

        //Push additional nodes to try to overwrite existing ones
        /*for node_key in 10..15 {
            gpu_data_handler.add_node(&self, node_key, false);
        }*/

        //delete some random bricks from leaf nodes
        for i in 15..20 {
            if 0 != (gpu_data_handler.render_data.metadata[i] & 0x00000004) {
                gpu_data_handler.render_data.node_children[i * 8 + 3] = empty_marker();
            }
        }

        // reset used bits
        for meta in gpu_data_handler.render_data.metadata.iter_mut() {
            *meta &= 0x00FFFFFE;
        }
        // --- DEBUG ---

        let output_texture = create_output_texture(resolution, images);
        commands.insert_resource(OctreeGPUView {
            do_the_thing: false,
            read_back: 0,
            data_handler: Arc::new(Mutex::new(gpu_data_handler)),
            viewing_glass: ShocoVoxViewingGlass {
                output_texture: output_texture.clone(),
                viewport: viewport,
            },
        });
        output_texture
    }
}

//##############################################################################
//  █████   █████ █████   █████████  ███████████ █████ ██████   ██████
// ░░███   ░░███ ░░███   ███░░░░░███░█░░░███░░░█░░███ ░░██████ ██████
//  ░███    ░███  ░███  ███     ░░░ ░   ░███  ░  ░███  ░███░█████░███
//  ░███    ░███  ░███ ░███             ░███     ░███  ░███░░███ ░███
//  ░░███   ███   ░███ ░███             ░███     ░███  ░███ ░░░  ░███
//   ░░░█████░    ░███ ░░███     ███    ░███     ░███  ░███      ░███
//     ░░███      █████ ░░█████████     █████    █████ █████     █████
//      ░░░      ░░░░░   ░░░░░░░░░     ░░░░░    ░░░░░ ░░░░░     ░░░░░
//  ███████████  ███████████ ███████████
// ░░███░░░░░███░█░░░███░░░█░░███░░░░░███
//  ░███    ░███░   ░███  ░  ░███    ░███
//  ░██████████     ░███     ░██████████
//  ░███░░░░░░      ░███     ░███░░░░░███
//  ░███            ░███     ░███    ░███
//  █████           █████    █████   █████
// ░░░░░           ░░░░░    ░░░░░   ░░░░░
//##############################################################################
impl VictimPointer {
    pub(crate) fn is_full(&self) -> bool {
        self.stored_items >= (self.max_meta_len - 1)
    }

    pub(crate) fn new(max_meta_len: usize) -> Self {
        Self {
            max_meta_len,
            stored_items: 0,
            meta_index: 0,
            child: 0,
        }
    }

    pub(crate) fn step(&mut self) {
        if self.child >= 7 {
            self.skip_node();
        } else {
            self.child += 1;
        }
    }

    pub(crate) fn skip_node(&mut self) {
        if self.meta_index == 0 {
            self.meta_index = self.max_meta_len - 1;
        } else {
            self.meta_index -= 1;
        }
        self.child = 0;
    }

    /// Provides the first available index in the metadata buffer which can be overwritten
    /// with node related information. It never returns with 0, because that is reserved for the root node,
    /// which should not be overwritten.
    fn first_available_node(&mut self, render_data: &mut ShocoVoxRenderData) -> usize {
        // If there is space left in the cache, use it all up
        if !self.is_full() {
            render_data.metadata[self.stored_items] |= 0x01;
            self.stored_items += 1;
            return self.stored_items - 1;
        }

        //look for the next internal node ( with node children )
        loop {
            let child_node_index =
                render_data.node_children[self.meta_index * 8 + self.child] as usize;

            // child at target is not empty in a non-leaf node, which means
            // the target child might point to an internal node if it's valid
            if
            // parent node has a child at target octant, which isn't invalidated
            child_node_index != empty_marker() as usize
                // parent node is not a leaf
                && (0 == (render_data.metadata[self.meta_index] & 0x00000004))
            {
                let child_meta_index =
                    render_data.node_children[self.meta_index * 8 + self.child] as usize;
                if 0 == (render_data.metadata[child_meta_index] & 0x01) {
                    //mark child as used
                    render_data.metadata[child_meta_index] |= 0x01;

                    //erase connection to parent node
                    render_data.node_children[self.meta_index * 8 + self.child] = empty_marker();

                    if
                    // erased node is a leaf and it has children
                    0 != (render_data.metadata[child_meta_index] & 0x00000004)
                        && 0 != (render_data.metadata[child_meta_index] & 0x00FF0000)
                    {
                        // Since the node is deleted, if it has any stored bricks, those are freed up
                        for octant in 0..8 {
                            let brick_index =
                                render_data.node_children[child_meta_index * 8 + octant] as usize;
                            if
                            // child is valid
                            brick_index != empty_marker() as usize
                            // child is not empty and not solid
                            && 0 != (render_data.metadata[child_meta_index]
                                & (0x01 << (8 + octant)))
                                && 0 != (render_data.metadata[child_meta_index]
                                    & (0x01 << (16 + octant)))
                            {
                                // mark brick unused
                                render_data.metadata[brick_index / 8] &=
                                    !(0x01 << (24 + (brick_index % 8)));
                            }
                        }
                    }

                    self.step();
                    return child_meta_index;
                } else {
                    // mark child as unused
                    render_data.metadata[child_meta_index] &= 0xFFFFFFFE;
                }
            }
            self.step();
        }
    }

    /// Finds the first available brick, and marks it as used
    fn first_available_brick(&mut self, render_data: &mut ShocoVoxRenderData) -> usize {
        let max_brick_count = render_data.metadata.len() * 8;

        // If there is space left in the cache, use it all up
        if self.stored_items < max_brick_count - 1 {
            render_data.metadata[self.stored_items / 8] |= 0x01 << (24 + (self.stored_items % 8));
            self.stored_items += 1;
            return self.stored_items - 1;
        }

        // look for the next victim leaf node with bricks
        loop {
            let brick_index = render_data.node_children[self.meta_index * 8 + self.child] as usize;
            if
            // child at target is not empty
            brick_index != empty_marker() as usize
            //node is leaf
            &&(0 != (render_data.metadata[self.meta_index] & 0x00000004))
                && (
                    // In case of uniform leaf nodes, only the first child might have a brick
                    // So the node is either not uniform, or the target child points to 0
                    (0 == render_data.metadata[self.meta_index] & 0x00000008)
                    || (0 == self.child)
                )
                && (
                    // node child is not empty, and parted at octant
                    0 != (render_data.metadata[self.meta_index]
                        & (0x01 << (8 + self.child)))
                        && 0 != (render_data.metadata[self.meta_index]
                            & (0x01 << (16 + self.child)))
                )
            {
                let brick_used_mask = 0x01 << (24 + (brick_index % 8));
                // if used bit for the brick is 0, it can be safely overwritten
                if 0 == (render_data.metadata[brick_index / 8] & brick_used_mask) {
                    // mark brick as used
                    render_data.metadata[brick_index / 8] |= brick_used_mask;

                    // erase connection of child brick to parent node
                    render_data.node_children[self.meta_index * 8 + self.child] = empty_marker();

                    // step victim pointer forward
                    self.step();
                    return brick_index;
                }

                // set the bricks used bit to 0 for the currently used brick
                render_data.metadata[brick_index / 8] &= !brick_used_mask;
            }
            if
            // node is not leaf
            (0 == (render_data.metadata[self.meta_index] & 0x00000004))
            // in case node is uniform, octant 0 should have been checked for vacancy already, so it is safe to skip to next leaf node
            || (0 != (render_data.metadata[self.meta_index] & 0x00000008))
            {
                self.skip_node();
            } else {
                self.step();
            }
        }
    }
}
impl OctreeGPUDataHandler {
    //##############################################################################
    //  ██████████     █████████   ███████████   █████████
    // ░░███░░░░███   ███░░░░░███ ░█░░░███░░░█  ███░░░░░███
    //  ░███   ░░███ ░███    ░███ ░   ░███  ░  ░███    ░███
    //  ░███    ░███ ░███████████     ░███     ░███████████
    //  ░███    ░███ ░███░░░░░███     ░███     ░███░░░░░███
    //  ░███    ███  ░███    ░███     ░███     ░███    ░███
    //  ██████████   █████   █████    █████    █████   █████
    // ░░░░░░░░░░   ░░░░░   ░░░░░    ░░░░░    ░░░░░   ░░░░░

    //  ██████████   ██████████  █████████  █████   █████████  ██████   █████
    // ░░███░░░░███ ░░███░░░░░█ ███░░░░░███░░███   ███░░░░░███░░██████ ░░███
    //  ░███   ░░███ ░███  █ ░ ░███    ░░░  ░███  ███     ░░░  ░███░███ ░███
    //  ░███    ░███ ░██████   ░░█████████  ░███ ░███          ░███░░███░███
    //  ░███    ░███ ░███░░█    ░░░░░░░░███ ░███ ░███    █████ ░███ ░░██████
    //  ░███    ███  ░███ ░   █ ███    ░███ ░███ ░░███  ░░███  ░███  ░░█████
    //  ██████████   ██████████░░█████████  █████ ░░█████████  █████  ░░█████
    // ░░░░░░░░░░   ░░░░░░░░░░  ░░░░░░░░░  ░░░░░   ░░░░░░░░░  ░░░░░    ░░░░░
    //##############################################################################
    /// Updates the meta element value to store the brick structure of the given leaf node.
    /// Does not erase anything in @sized_node_meta, it's expected to be cleared before
    /// the first use of this function
    /// for the given brick octant
    /// * `sized_node_meta` - the bytes to update
    /// * `brick` - the brick to describe into the bytes
    /// * `brick_octant` - the octant to update in the bytes
    fn meta_add_leaf_brick_structure<T, const DIM: usize>(
        sized_node_meta: &mut u32,
        brick: &BrickData<T, DIM>,
        brick_octant: usize,
    ) where
        T: Default + Clone + PartialEq + VoxelData,
    {
        match brick {
            BrickData::Empty => {} // Child structure properties already set to NIL
            BrickData::Solid(_voxel) => {
                // set child Occupied bits, child Structure bits already set to NIL
                *sized_node_meta |= 0x01 << (8 + brick_octant);
            }
            BrickData::Parted(_brick) => {
                // set child Occupied bits
                *sized_node_meta |= 0x01 << (8 + brick_octant);

                // set child Structure bits
                *sized_node_meta |= 0x01 << (16 + brick_octant);
            }
        };
    }

    /// Creates the descriptor bytes for the given node
    fn create_node_properties<T, const DIM: usize>(node: &NodeContent<T, DIM>) -> u32
    where
        T: Default + Copy + Clone + PartialEq + VoxelData,
    {
        let mut meta = 0;
        match node {
            NodeContent::Internal(_) | NodeContent::Nothing => {
                meta &= 0xFFFFFFFB; // element is not leaf
                meta &= 0xFFFFFFF7; // element is not uniform
            }
            NodeContent::Leaf(bricks) => {
                meta |= 0x00000004; // element is leaf
                meta &= 0xFFFFFFF7; // element is not uniform
                for octant in 0..8 {
                    Self::meta_add_leaf_brick_structure(&mut meta, &bricks[octant], octant);
                }
            }
            NodeContent::UniformLeaf(brick) => {
                meta |= 0x00000004; // element is leaf
                meta |= 0x00000008; // element is uniform
                Self::meta_add_leaf_brick_structure(&mut meta, brick, 0);
            }
        };
        meta
    }

    //##############################################################################
    //    █████████   ██████████   ██████████
    //   ███░░░░░███ ░░███░░░░███ ░░███░░░░███
    //  ░███    ░███  ░███   ░░███ ░███   ░░███
    //  ░███████████  ░███    ░███ ░███    ░███
    //  ░███░░░░░███  ░███    ░███ ░███    ░███
    //  ░███    ░███  ░███    ███  ░███    ███
    //  █████   █████ ██████████   ██████████
    // ░░░░░   ░░░░░ ░░░░░░░░░░   ░░░░░░░░░░
    //  ██████   █████    ███████    ██████████   ██████████
    // ░░██████ ░░███   ███░░░░░███ ░░███░░░░███ ░░███░░░░░█
    //  ░███░███ ░███  ███     ░░███ ░███   ░░███ ░███  █ ░
    //  ░███░░███░███ ░███      ░███ ░███    ░███ ░██████
    //  ░███ ░░██████ ░███      ░███ ░███    ░███ ░███░░█
    //  ░███  ░░█████ ░░███     ███  ░███    ███  ░███ ░   █
    //  █████  ░░█████ ░░░███████░   ██████████   ██████████
    // ░░░░░    ░░░░░    ░░░░░░░    ░░░░░░░░░░   ░░░░░░░░░░
    //##############################################################################
    fn add_node<T, const DIM: usize>(
        &mut self,
        tree: &Octree<T, DIM>,
        node_key: usize,
        try_add_children: bool,
    ) where
        T: Default + Copy + Clone + PartialEq + VoxelData,
    {
        if try_add_children && self.victim_node.is_full() {
            // Do not add additional nodes at initial upload if the cache is already full
            return;
        }

        // Determine the index in meta
        let node_element_index = self.victim_node.first_available_node(&mut self.render_data);
        self.map_to_node_index_in_metadata
            .insert(node_key, node_element_index);

        // Add node properties to metadata
        self.render_data.metadata[node_element_index] =
            Self::create_node_properties(tree.nodes.get(node_key));

        // Update occupancy in ocbits
        let occupied_bits = tree.stored_occupied_bits(node_key);
        self.render_data.node_ocbits[node_element_index * 2] =
            (occupied_bits & 0x00000000FFFFFFFF) as u32;
        self.render_data.node_ocbits[node_element_index * 2 + 1] =
            ((occupied_bits & 0xFFFFFFFF00000000) >> 32) as u32;

        // Add node content
        match tree.nodes.get(node_key) {
            NodeContent::UniformLeaf(brick) => {
                debug_assert!(
                    matches!(
                        tree.node_children[node_key].content,
                        NodeChildrenArray::OccupancyBitmap(_)
                    ),
                    "Expected Uniform leaf to have OccupancyBitmap(_) instead of {:?}",
                    tree.node_children[node_key].content
                );

                let (brick_index, brick_added) = self.add_brick(brick);
                self.render_data.node_children[node_element_index * 8 + 0] = brick_index;
                self.render_data.node_children[node_element_index * 8 + 1] = empty_marker();
                self.render_data.node_children[node_element_index * 8 + 2] = empty_marker();
                self.render_data.node_children[node_element_index * 8 + 3] = empty_marker();
                self.render_data.node_children[node_element_index * 8 + 4] = empty_marker();
                self.render_data.node_children[node_element_index * 8 + 5] = empty_marker();
                self.render_data.node_children[node_element_index * 8 + 6] = empty_marker();
                self.render_data.node_children[node_element_index * 8 + 7] = empty_marker();
                #[cfg(debug_assertions)]
                {
                    if !brick_added {
                        // If no brick was added, the occupied bits should either be empty or full
                        if let NodeChildrenArray::OccupancyBitmap(occupied_bits) =
                            tree.node_children[node_key].content
                        {
                            debug_assert!(occupied_bits == 0 || occupied_bits == u64::MAX);
                        }
                    }
                }
            }
            NodeContent::Leaf(bricks) => {
                debug_assert!(
                    matches!(
                        tree.node_children[node_key].content,
                        NodeChildrenArray::OccupancyBitmap(_)
                    ),
                    "Expected Leaf to have OccupancyBitmaps(_) instead of {:?}",
                    tree.node_children[node_key].content
                );
                for octant in 0..8 {
                    let (brick_index, brick_added) = self.add_brick(&bricks[octant]);
                    self.render_data.node_children[node_element_index * 8 + octant] = brick_index;
                    #[cfg(debug_assertions)]
                    {
                        if !brick_added {
                            // If no brick was added, the relevant occupied bits should either be empty or full
                            if let NodeChildrenArray::OccupancyBitmap(occupied_bits) =
                                tree.node_children[node_key].content
                            {
                                debug_assert!(
                                    0 == occupied_bits & BITMAP_MASK_FOR_OCTANT_LUT[octant]
                                        || BITMAP_MASK_FOR_OCTANT_LUT[octant]
                                            == occupied_bits & BITMAP_MASK_FOR_OCTANT_LUT[octant]
                                );
                            }
                        }
                    }
                }
            }
            NodeContent::Internal(_) => {
                for octant in 0..8 {
                    let child_key = tree.node_children[node_key][octant] as usize;
                    if child_key != empty_marker() as usize {
                        if try_add_children
                            && !self.map_to_node_index_in_metadata.contains_key(&child_key)
                        {
                            self.add_node(tree, child_key, try_add_children);
                        }

                        self.render_data.node_children[node_element_index * 8 + octant as usize] =
                            *self
                                .map_to_node_index_in_metadata
                                .get(&child_key)
                                .unwrap_or(&(empty_marker() as usize))
                                as u32;
                    } else {
                        self.render_data.node_children[node_element_index * 8 + octant as usize] =
                            empty_marker();
                    }
                }
            }
            NodeContent::Nothing => {
                for octant in 0..8 {
                    self.render_data.node_children[node_element_index * 8 + octant as usize] =
                        empty_marker();
                }
            }
        }
    }

    //##############################################################################
    //    █████████   ██████████   ██████████
    //   ███░░░░░███ ░░███░░░░███ ░░███░░░░███
    //  ░███    ░███  ░███   ░░███ ░███   ░░███
    //  ░███████████  ░███    ░███ ░███    ░███
    //  ░███░░░░░███  ░███    ░███ ░███    ░███
    //  ░███    ░███  ░███    ███  ░███    ███
    //  █████   █████ ██████████   ██████████
    // ░░░░░   ░░░░░ ░░░░░░░░░░   ░░░░░░░░░░
    //  ███████████  ███████████   █████   █████████  █████   ████
    // ░░███░░░░░███░░███░░░░░███ ░░███   ███░░░░░███░░███   ███░
    //  ░███    ░███ ░███    ░███  ░███  ███     ░░░  ░███  ███
    //  ░██████████  ░██████████   ░███ ░███          ░███████
    //  ░███░░░░░███ ░███░░░░░███  ░███ ░███          ░███░░███
    //  ░███    ░███ ░███    ░███  ░███ ░░███     ███ ░███ ░░███
    //  ███████████  █████   █████ █████ ░░█████████  █████ ░░████
    // ░░░░░░░░░░░  ░░░░░   ░░░░░ ░░░░░   ░░░░░░░░░  ░░░░░   ░░░░
    //##############################################################################
    /// Loads a brick into the provided voxels vector and color palette
    /// * `brick` - The brick to upload
    /// * `returns` - the identifier to set in @SizedNode and true if a new brick was aded to the voxels vector
    fn add_brick<T, const DIM: usize>(&mut self, brick: &BrickData<T, DIM>) -> (u32, bool)
    where
        T: Default + Clone + PartialEq + VoxelData,
    {
        debug_assert_eq!(
            self.render_data.voxels.len() % (DIM * DIM * DIM),
            0,
            "Expected Voxel buffer length({:?}) to be divisble by {:?}",
            self.render_data.voxels.len(),
            (DIM * DIM * DIM)
        );

        match brick {
            BrickData::Empty => (empty_marker(), false),
            BrickData::Solid(voxel) => {
                let albedo = voxel.albedo();
                // The number of colors inserted into the palette is the size of the color palette map
                let color_palette_size = self.map_to_color_index_in_palette.keys().len();
                if let std::collections::hash_map::Entry::Vacant(e) =
                    self.map_to_color_index_in_palette.entry(albedo)
                {
                    e.insert(color_palette_size);
                    self.render_data.color_palette[color_palette_size] = Vec4::new(
                        albedo.r as f32 / 255.,
                        albedo.g as f32 / 255.,
                        albedo.b as f32 / 255.,
                        albedo.a as f32 / 255.,
                    );
                }
                (self.map_to_color_index_in_palette[&albedo] as u32, false)
            }
            BrickData::Parted(brick) => {
                let brick_index = self
                    .victim_brick
                    .first_available_brick(&mut self.render_data);
                for z in 0..DIM {
                    for y in 0..DIM {
                        for x in 0..DIM {
                            // The number of colors inserted into the palette is the size of the color palette map
                            let potential_new_albedo_index =
                                self.map_to_color_index_in_palette.keys().len();
                            let albedo = brick[x][y][z].albedo();
                            let albedo_index = if let std::collections::hash_map::Entry::Vacant(e) =
                                self.map_to_color_index_in_palette.entry(albedo)
                            {
                                e.insert(potential_new_albedo_index);
                                self.render_data.color_palette[potential_new_albedo_index] =
                                    Vec4::new(
                                        albedo.r as f32 / 255.,
                                        albedo.g as f32 / 255.,
                                        albedo.b as f32 / 255.,
                                        albedo.a as f32 / 255.,
                                    );
                                potential_new_albedo_index
                            } else {
                                self.map_to_color_index_in_palette[&albedo]
                            };
                            self.render_data.voxels[(brick_index * (DIM * DIM * DIM))
                                + flat_projection(x, y, z, DIM)] = Voxelement {
                                albedo_index: albedo_index as u32,
                                content: brick[x][y][z].user_data(),
                            };
                        }
                    }
                }
                (brick_index as u32, true)
            }
        }
    }
}

pub(crate) fn sync_with_main_world(// tree_view: Option<ResMut<OctreeGPUView>>,
    // mut world: ResMut<bevy::render::MainWorld>,
) {
    // This function is unused because ExtractResource plugin is handling the sync
    // However, it is only one way: MainWorld --> RenderWorld
    // Any modification here is overwritten by the plugin if it is active,
    // in order to enable data flow in the opposite direction, extractresource plugin
    // needs to be disabled, and the sync logic (both ways) needs to be implemented here
    // refer to: https://www.reddit.com/r/bevy/comments/1ay50ee/copy_from_render_world_to_main_world/
    // if let Some(tree_view) = tree_view {
    //     let mut tree_view_mainworld = world.get_resource_mut::<OctreeGPUView>().unwrap();
    //     tree_view_mainworld.read_back = tree_view.read_back;
    //     println!("Read back in render world: {:?}", tree_view.read_back);
    // }
}

//##############################################################################
//    █████████  ███████████  █████  █████
//   ███░░░░░███░░███░░░░░███░░███  ░░███
//  ███     ░░░  ░███    ░███ ░███   ░███
// ░███          ░██████████  ░███   ░███
// ░███    █████ ░███░░░░░░   ░███   ░███
// ░░███  ░░███  ░███         ░███   ░███
//  ░░█████████  █████        ░░████████
//   ░░░░░░░░░  ░░░░░          ░░░░░░░░
//  ███████████   ██████████   █████████   ██████████
// ░░███░░░░░███ ░░███░░░░░█  ███░░░░░███ ░░███░░░░███
//  ░███    ░███  ░███  █ ░  ░███    ░███  ░███   ░░███
//  ░██████████   ░██████    ░███████████  ░███    ░███
//  ░███░░░░░███  ░███░░█    ░███░░░░░███  ░███    ░███
//  ░███    ░███  ░███ ░   █ ░███    ░███  ░███    ███
//  █████   █████ ██████████ █████   █████ ██████████
// ░░░░░   ░░░░░ ░░░░░░░░░░ ░░░░░   ░░░░░ ░░░░░░░░░░
//##############################################################################
pub(crate) fn handle_gpu_readback(
    render_device: Res<RenderDevice>,
    mut tree_gpu_view: Option<ResMut<OctreeGPUView>>,
) {
    // Data updates triggered by debug interface
    if let Some(ref mut tree_gpu_view) = tree_gpu_view {
        if tree_gpu_view.do_the_thing {
            let received_data;
            {
                let data_handler = tree_gpu_view.data_handler.lock().unwrap();
                // GPU buffer read
                // https://docs.rs/bevy/latest/src/gpu_readback/gpu_readback.rs.html
                let buffer_slice = data_handler
                    .readable_debug_gpu_interface
                    .as_ref()
                    .unwrap()
                    .slice(..);
                let (s, r) = crossbeam::channel::unbounded::<()>();
                buffer_slice.map_async(bevy::render::render_resource::MapMode::Read, move |d| {
                    match d {
                        Ok(_) => s.send(()).expect("Failed to send map update"),
                        Err(err) => panic!("Couldn't map debug interface buffer!: {err}"),
                    }
                });

                render_device
                    .poll(bevy::render::render_resource::Maintain::wait())
                    .panic_on_timeout();

                r.recv().expect("Failed to receive the map_async message");
                {
                    let buffer_view = buffer_slice.get_mapped_range();

                    let data = buffer_view
                        .chunks(std::mem::size_of::<u32>())
                        .map(|chunk| u32::from_ne_bytes(chunk.try_into().expect("should be a u32")))
                        .collect::<Vec<u32>>();
                    received_data = data[0];
                    // println!("received_data: {:?}", data);
                }
                data_handler
                    .readable_debug_gpu_interface
                    .as_ref()
                    .unwrap()
                    .unmap();
            }
            tree_gpu_view.read_back = received_data;
            tree_gpu_view.do_the_thing = false;
        }
    }
}

//##############################################################################
//    █████████  ███████████  █████  █████
//   ███░░░░░███░░███░░░░░███░░███  ░░███
//  ███     ░░░  ░███    ░███ ░███   ░███
// ░███          ░██████████  ░███   ░███
// ░███    █████ ░███░░░░░░   ░███   ░███
// ░░███  ░░███  ░███         ░███   ░███
//  ░░█████████  █████        ░░████████
//   ░░░░░░░░░  ░░░░░          ░░░░░░░░

//  █████   ███   █████ ███████████   █████ ███████████ ██████████
// ░░███   ░███  ░░███ ░░███░░░░░███ ░░███ ░█░░░███░░░█░░███░░░░░█
//  ░███   ░███   ░███  ░███    ░███  ░███ ░   ░███  ░  ░███  █ ░
//  ░███   ░███   ░███  ░██████████   ░███     ░███     ░██████
//  ░░███  █████  ███   ░███░░░░░███  ░███     ░███     ░███░░█
//   ░░░█████░█████░    ░███    ░███  ░███     ░███     ░███ ░   █
//     ░░███ ░░███      █████   █████ █████    █████    ██████████
//      ░░░   ░░░      ░░░░░   ░░░░░ ░░░░░    ░░░░░    ░░░░░░░░░░
//##############################################################################
pub(crate) fn write_to_gpu(
    tree_gpu_view: Option<ResMut<OctreeGPUView>>,
    svx_pipeline: Option<ResMut<ShocoVoxRenderPipeline>>,
) {
    //TODO: Document that all components are lost during extract transition
    // Data updates triggered by debug interface
    if let Some(tree_gpu_view) = tree_gpu_view {
        let svx_pipeline = svx_pipeline.unwrap();
        let render_queue = &svx_pipeline.render_queue.0;
        if tree_gpu_view.do_the_thing {
            // GPU buffer Write
            use bevy::render::render_resource::encase::StorageBuffer;
            let data_buffer = StorageBuffer::new(vec![0, 0, 0, 1]);
            render_queue.write_buffer(
                tree_gpu_view
                    .data_handler
                    .lock()
                    .unwrap()
                    .debug_gpu_interface
                    .as_ref()
                    .unwrap(),
                0,
                &data_buffer.into_inner(),
            );
        }
    }
}
