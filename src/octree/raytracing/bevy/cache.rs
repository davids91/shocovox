use crate::object_pool::empty_marker;
use crate::spatial::math::flat_projection;
use crate::{
    octree::{
        raytracing::bevy::types::{OctreeRenderData, Voxelement},
        types::{NodeChildrenArray, NodeContent},
        BrickData, Octree, VoxelData,
    },
    spatial::lut::BITMAP_MASK_FOR_OCTANT_LUT,
};
use bevy::math::Vec4;

use super::types::{OctreeGPUDataHandler, VictimPointer};

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
        self.max_meta_len <= self.stored_items
    }

    pub(crate) fn new(max_meta_len: usize) -> Self {
        Self {
            max_meta_len,
            stored_items: 0,
            meta_index: max_meta_len - 1,
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

    pub(crate) fn went_around(&self) -> bool {
        self.meta_index == 0 && self.child == 0
    }

    /// Provides the first available index in the metadata buffer which can be overwritten
    /// with node related meta information and the index of the meta from where the child was taken.
    /// The available index is never 0, because that is reserved for the root node, which should not be overwritten.
    fn first_available_node(
        &mut self,
        render_data: &mut OctreeRenderData,
    ) -> (usize, Option<usize>) {
        // If there is space left in the cache, use it all up
        if !self.is_full() {
            render_data.metadata[self.stored_items] |= 0x01;
            self.stored_items += 1;
            return (self.stored_items - 1, None);
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
                    let severed_parent_index = self.meta_index;
                    //mark child as used
                    render_data.metadata[child_meta_index] |= 0x01;

                    //erase connection to parent node
                    render_data.node_children[severed_parent_index * 8 + self.child] =
                        empty_marker();

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

                    // step victim pointer forward and return with the requested data
                    self.step();
                    return (child_meta_index, Some(severed_parent_index));
                } else {
                    // mark child as unused
                    render_data.metadata[child_meta_index] &= 0xFFFFFFFE;
                }
            }
            self.step();
        }
    }

    /// Finds the first available brick, and marks it as used
    /// Returns with the resulting brick index, and optionally
    /// the index in the meta, where the brick was taken from
    fn first_available_brick(
        &mut self,
        render_data: &mut OctreeRenderData,
    ) -> (usize, Option<usize>) {
        let max_brick_count = render_data.metadata.len() * 8;

        // If there is space left in the cache, use it all up
        if self.stored_items < max_brick_count - 1 {
            render_data.metadata[self.stored_items / 8] |= 0x01 << (24 + (self.stored_items % 8));
            self.stored_items += 1;
            return (self.stored_items - 1, None);
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
                    let severed_parent_node = self.meta_index;
                    render_data.node_children[self.meta_index * 8 + self.child] = empty_marker();

                    // step victim pointer forward and return with the requested data
                    self.step();
                    return (brick_index, Some(severed_parent_node));
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
    /// Writes the data of the node to the first available index
    /// returns the index of the overwritten metadata entry and a vector
    /// of index values where metadata entries were taken from, taking a child of another node.
    /// May take children from 0-2 nodes!
    /// It may fail to add the node, when @try_add_children is set to true
    pub(crate) fn add_node<T, const DIM: usize>(
        &mut self,
        tree: &Octree<T, DIM>,
        node_key: usize,
        try_add_children: bool,
    ) -> (Option<usize>, Vec<usize>)
    where
        T: Default + Copy + Clone + PartialEq + VoxelData,
    {
        if try_add_children && self.victim_node.is_full() {
            // Do not add additional nodes at initial upload if the cache is already full
            return (None, Vec::new());
        }

        // Determine the index in meta
        let (node_element_index, severed_parent_index) =
            self.victim_node.first_available_node(&mut self.render_data);
        self.node_key_vs_meta_index
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
        let mut severed_parents = if let Some(severed_parent_index) = severed_parent_index {
            vec![severed_parent_index]
        } else {
            Vec::new()
        };
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

                if try_add_children {
                    let (brick_index, severed_parent_data) = self.add_brick(brick);
                    self.render_data.node_children[node_element_index * 8 + 0] = brick_index;
                    if let Some(severed_parent_index) = severed_parent_data {
                        severed_parents.push(severed_parent_index);
                    }
                } else {
                    self.render_data.node_children[node_element_index * 8 + 0] = empty_marker();
                }

                self.render_data.node_children[node_element_index * 8 + 1] = empty_marker();
                self.render_data.node_children[node_element_index * 8 + 2] = empty_marker();
                self.render_data.node_children[node_element_index * 8 + 3] = empty_marker();
                self.render_data.node_children[node_element_index * 8 + 4] = empty_marker();
                self.render_data.node_children[node_element_index * 8 + 5] = empty_marker();
                self.render_data.node_children[node_element_index * 8 + 6] = empty_marker();
                self.render_data.node_children[node_element_index * 8 + 7] = empty_marker();
                #[cfg(debug_assertions)]
                {
                    if let BrickData::Solid(_) | BrickData::Empty = brick {
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
                if try_add_children {
                    for octant in 0..8 {
                        let (brick_index, severed_parent_data) = self.add_brick(&bricks[octant]);
                        self.render_data.node_children[node_element_index * 8 + octant] =
                            brick_index;
                        if let Some(severed_parent_index) = severed_parent_data {
                            severed_parents.push(severed_parent_index);
                        }
                        #[cfg(debug_assertions)]
                        {
                            if let BrickData::Solid(_) | BrickData::Empty = bricks[octant] {
                                // If no brick was added, the relevant occupied bits should either be empty or full
                                if let NodeChildrenArray::OccupancyBitmap(occupied_bits) =
                                    tree.node_children[node_key].content
                                {
                                    debug_assert!(
                                        0 == occupied_bits & BITMAP_MASK_FOR_OCTANT_LUT[octant]
                                            || BITMAP_MASK_FOR_OCTANT_LUT[octant]
                                                == occupied_bits
                                                    & BITMAP_MASK_FOR_OCTANT_LUT[octant]
                                    );
                                }
                            }
                        }
                    }
                } else {
                    for octant in 0..8 {
                        self.render_data.node_children[node_element_index * 8 + octant] =
                            empty_marker();
                    }
                }
            }
            NodeContent::Internal(_) => {
                for octant in 0..8 {
                    let child_key = tree.node_children[node_key][octant] as usize;
                    if child_key != empty_marker() as usize {
                        if try_add_children
                            && !self.node_key_vs_meta_index.contains_left(&child_key)
                        {
                            // In case @try_add_children is true, no new node is added in case the cache is full,
                            // so there will be no severed parents in this case
                            self.add_node(tree, child_key, try_add_children);
                        }

                        self.render_data.node_children[node_element_index * 8 + octant as usize] =
                            *self
                                .node_key_vs_meta_index
                                .get_by_left(&child_key)
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
        (Some(node_element_index), severed_parents)
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
    /// * `returns` - (index in @render_data.metadata, None) if a brick was uploaded and no other node had its child taken away
    ///             - (index in @render_data.metadata, Some(severed_parent_index))
    ///             --> if a new brick was added to the voxels vector, taking away a brick child from a leaf node
    ///             - (index in @render_data.color_palette, None) if brick is solid, and no node were stripped of its brick
    pub(crate) fn add_brick<T, const DIM: usize>(
        &mut self,
        brick: &BrickData<T, DIM>,
    ) -> (u32, Option<usize>)
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
            BrickData::Empty => (empty_marker(), None),
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
                (self.map_to_color_index_in_palette[&albedo] as u32, None)
            }
            BrickData::Parted(brick) => {
                let (brick_index, severed_parent_data) = self
                    .victim_brick
                    .first_available_brick(&mut self.render_data);
                let severed_parent_index = severed_parent_data;
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
                (brick_index as u32, severed_parent_index)
            }
        }
    }
}
