use crate::object_pool::empty_marker;
use crate::octree::raytracing::bevy::types::BrickOwnedBy;
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
    /// Returns the size of the buffer this pointer covers
    pub(crate) fn len(&self) -> usize {
        self.max_meta_len
    }

    /// Returns true if no new nodes can be added without overwriting another
    pub(crate) fn is_full(&self) -> bool {
        self.max_meta_len <= self.stored_items
    }

    /// Creates object, based on the given cache length it should cover
    pub(crate) fn new(max_meta_len: usize) -> Self {
        Self {
            max_meta_len,
            loop_count: 0,
            stored_items: 0,
            meta_index: max_meta_len - 1,
            child: 0,
        }
    }

    /// Steps the iterator forward to the next children, if available, or the next node.
    /// Wraps around
    pub(crate) fn step(&mut self) {
        if self.child >= 7 {
            self.skip_node();
        } else {
            self.child += 1;
        }
    }

    /// Steps the iterator forward one node
    pub(crate) fn skip_node(&mut self) {
        if self.meta_index == 0 {
            self.loop_count += 1;
            self.meta_index = self.max_meta_len - 1;
        } else {
            self.meta_index -= 1;
        }
        self.child = 0;
    }

    /// Provides the number of times the victim node pointer has started from the first element in the cache
    pub(crate) fn get_loop_count(&self) -> usize {
        self.loop_count
    }

    /// Provides the first available index in the metadata buffer which can be overwritten
    /// with node related meta information and optionally the source where the child was taken from.
    fn first_available_node(
        &mut self,
        render_data: &mut OctreeRenderData,
    ) -> (usize, Option<(usize, u8)>) {
        // If there is space left in the cache, use it all up
        if !self.is_full() {
            render_data.metadata[self.stored_items] |= OctreeGPUDataHandler::NODE_USED_MASK;
            self.meta_index = self.stored_items;
            self.stored_items += 1;
            return (self.meta_index, None);
        }

        //look for the next internal node ( with node children )
        loop {
            // child at target is not empty in a non-leaf node, which means
            // the target child might point to an internal node if it's valid
            // parent node has a child at target octant, which isn't invalid
            if 0 == (render_data.metadata[self.meta_index] & OctreeGPUDataHandler::NODE_LEAF_MASK)
                && render_data.node_children[self.meta_index * 8 + self.child] != empty_marker()
            {
                let child_meta_index =
                    render_data.node_children[self.meta_index * 8 + self.child] as usize;
                if 0 == (render_data.metadata[child_meta_index]
                    & OctreeGPUDataHandler::NODE_USED_MASK)
                {
                    render_data.metadata[child_meta_index] |= OctreeGPUDataHandler::NODE_USED_MASK;
                    return (child_meta_index, Some((self.meta_index, self.child as u8)));
                } else {
                    // mark child as unused
                    render_data.metadata[child_meta_index] &= !OctreeGPUDataHandler::NODE_USED_MASK;
                }
            }
            self.step();
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

    /// Bitmask in metadata where the non-zero bits represent if the given node is used
    const NODE_USED_MASK: u32 = 0x00000001;

    /// Bitmask in metadata where the non-zero bits represent if the given node is a leaf
    const NODE_LEAF_MASK: u32 = 0x00000004;

    /// Bitmask in metadata where the non-zero bits represent if the given leaf is uniform
    /// Note: Non-leaf nodes can't be uniform
    const NODE_UNIFORM_MASK: u32 = 0x00000008;

    /// Provides the mask used with one metadata element to signal that the contained brick is used.
    /// Index of the metadata element should be brick index divided by 8, as one metadata element contains 8 bricks
    fn brick_used_mask(brick_index: usize) -> u32 {
        0x01 << (24 + (brick_index % 8))
    }

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
                meta &= !Self::NODE_LEAF_MASK; // element is not leaf
                meta &= !Self::NODE_UNIFORM_MASK; // element is not uniform
            }
            NodeContent::Leaf(bricks) => {
                meta |= Self::NODE_LEAF_MASK; // element is leaf
                meta &= !Self::NODE_UNIFORM_MASK; // element is not uniform
                for octant in 0..8 {
                    Self::meta_add_leaf_brick_structure(&mut meta, &bricks[octant], octant);
                }
            }
            NodeContent::UniformLeaf(brick) => {
                meta |= Self::NODE_LEAF_MASK; // element is leaf
                meta |= Self::NODE_UNIFORM_MASK; // element is uniform
                Self::meta_add_leaf_brick_structure(&mut meta, brick, 0);
            }
        };
        meta
    }

    //##############################################################################
    //  ██████████ ███████████     █████████    █████████  ██████████
    // ░░███░░░░░█░░███░░░░░███   ███░░░░░███  ███░░░░░███░░███░░░░░█
    //  ░███  █ ░  ░███    ░███  ░███    ░███ ░███    ░░░  ░███  █ ░
    //  ░██████    ░█████████    ░███████████ ░░█████████  ░██████
    //  ░███░░█    ░███░░░░░███  ░███░░░░░███  ░░░░░░░░███ ░███░░█
    //  ░███ ░   █ ░███    ░███  ░███    ░███  ███    ░███ ░███ ░   █
    //  ██████████ █████   █████ █████   █████░░█████████  ██████████
    // ░░░░░░░░░░ ░░░░░   ░░░░░ ░░░░░   ░░░░░  ░░░░░░░░░  ░░░░░░░░░░
    //  ██████   █████    ███████    ██████████   ██████████
    // ░░██████ ░░███   ███░░░░░███ ░░███░░░░███ ░░███░░░░░█
    //  ░███░███ ░███  ███     ░░███ ░███   ░░███ ░███  █ ░
    //  ░███░░███░███ ░███      ░███ ░███    ░███ ░██████
    //  ░███ ░░██████ ░███      ░███ ░███    ░███ ░███░░█
    //  ░███  ░░█████ ░░███     ███  ░███    ███  ░███ ░   █
    //  █████  ░░█████ ░░░███████░   ██████████   ██████████
    // ░░░░░    ░░░░░    ░░░░░░░    ░░░░░░░░░░   ░░░░░░░░░░
    //    █████████  █████   █████ █████ █████       ██████████
    //   ███░░░░░███░░███   ░░███ ░░███ ░░███       ░░███░░░░███
    //  ███     ░░░  ░███    ░███  ░███  ░███        ░███   ░░███
    // ░███          ░███████████  ░███  ░███        ░███    ░███
    // ░███          ░███░░░░░███  ░███  ░███        ░███    ░███
    // ░░███     ███ ░███    ░███  ░███  ░███      █ ░███    ███
    //  ░░█████████  █████   █████ █████ ███████████ ██████████
    //   ░░░░░░░░░  ░░░░░   ░░░░░ ░░░░░ ░░░░░░░░░░░ ░░░░░░░░░░
    //##############################################################################
    /// Erases the child node pointed by the given victim pointer
    /// returns with the vector of node index values and brick index values modified
    fn erase_node_child<T, const DIM: usize>(
        &mut self,
        meta_index: usize,
        child_octant: usize,
        tree: &Octree<T, DIM>,
    ) -> (Vec<usize>, Vec<usize>)
    where
        T: Default + Clone + PartialEq + VoxelData,
    {
        let mut modified_bricks = Vec::new();
        let mut modified_nodes = vec![meta_index];
        debug_assert!(
            self.node_key_vs_meta_index.contains_right(&meta_index),
            "Expected parent node to be in metadata index hash! (meta: {meta_index})"
        );
        let parent_key = self
            .node_key_vs_meta_index
            .get_by_right(&meta_index)
            .unwrap();

        debug_assert!(
            tree.nodes.key_is_valid(*parent_key),
            "Expected parent node({:?}) to be valid",
            parent_key
        );

        // Erase connection to parent
        let child_index = self.render_data.node_children[meta_index * 8 + child_octant] as usize;
        self.render_data.node_children[meta_index * 8 + child_octant] = empty_marker();
        debug_assert_ne!(
            child_index,
            empty_marker() as usize,
            "Expected victim pointer to point to an erasable node/brick, instead of: {child_index}"
        );

        match tree.nodes.get(*parent_key) {
            NodeContent::Nothing => {
                panic!("HOW DO I ERASE NOTHING. AMERICA EXPLAIN")
            }
            NodeContent::Internal(_occupied_bits) => {
                debug_assert!(
                    self.node_key_vs_meta_index.contains_right(&child_index),
                    "Expected erased child node index[{child_index}] to be in metadata index hash!"
                );
                let child_key = self
                    .node_key_vs_meta_index
                    .get_by_right(&child_index)
                    .unwrap();
                debug_assert!(
                    tree.nodes.key_is_valid(*child_key),
                    "Expected erased child node({child_key}) to be valid"
                );

                if let NodeContent::Leaf(_) | NodeContent::UniformLeaf(_) =
                    tree.nodes.get(*child_key)
                {
                    // make the children bricks of the removed leaf orphan
                    for octant in 0..8 {
                        let brick_index =
                            self.render_data.node_children[child_index * 8 + octant] as usize;
                        if brick_index != empty_marker() as usize {
                            self.brick_ownership[brick_index] = BrickOwnedBy::NotOwned;

                            // mark brick as unused
                            self.render_data.metadata[brick_index / 8] &=
                                !Self::brick_used_mask(brick_index);

                            // Eliminate connection
                            self.render_data.node_children[child_index * 8 + octant] =
                                empty_marker();

                            modified_bricks.push(brick_index);
                        }
                    }
                }
                modified_nodes.push(child_index);
            }
            NodeContent::UniformLeaf(_) | NodeContent::Leaf(_) => {
                debug_assert!(
                    (0 == child_octant)
                        || matches!(tree.nodes.get(*parent_key), NodeContent::Leaf(_)),
                    "Expected child octant in uniform leaf to be 0 in: {:?}",
                    (meta_index, child_octant)
                );
                if child_index != empty_marker() as usize {
                    self.brick_ownership[child_index] = BrickOwnedBy::NotOwned;
                    modified_bricks.push(child_index);

                    // mark brick as unused
                    self.render_data.metadata[child_index / 8] &=
                        !Self::brick_used_mask(child_index);
                }
            }
        }

        //return with updated ranges in voxels and metadata
        (modified_nodes, modified_bricks)
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
    /// Upon success, returns the index in metadata where the node is added
    /// and vectors of modified nodes, bricks:
    /// (meta_index, modified_nodes, modified_bricks)
    pub(crate) fn add_node<T, const DIM: usize>(
        &mut self,
        tree: &Octree<T, DIM>,
        node_key: usize,
        try_add_children: bool,
    ) -> Option<(usize, Vec<usize>, Vec<usize>)>
    where
        T: Default + Copy + Clone + PartialEq + VoxelData + Send + Sync + 'static,
    {
        if try_add_children && self.victim_node.is_full() {
            // Do not add additional nodes at initial upload if the cache is already full
            return None;
        }

        // Determine the index in meta, overwrite a currently present node if needed
        let (node_element_index, robbed_parent) =
            self.victim_node.first_available_node(&mut self.render_data);
        let (mut modified_nodes, mut modified_bricks) = if let Some(robbed_parent) = robbed_parent {
            self.erase_node_child(robbed_parent.0, robbed_parent.1 as usize, tree)
        } else {
            (vec![node_element_index], Vec::new())
        };

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
                    let (brick_index, mut current_modified_nodes, mut current_modified_bricks) =
                        self.add_brick(tree, node_key, 0);
                    modified_bricks.push(brick_index as usize);
                    modified_nodes.append(&mut current_modified_nodes);
                    modified_bricks.append(&mut current_modified_bricks);

                    self.render_data.node_children[node_element_index * 8 + 0] = brick_index;
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
                        let (brick_index, mut current_modified_nodes, mut current_modified_bricks) =
                            self.add_brick(tree, node_key, octant);
                        modified_bricks.push(brick_index as usize);
                        modified_nodes.append(&mut current_modified_nodes);
                        modified_bricks.append(&mut current_modified_bricks);

                        self.render_data.node_children[node_element_index * 8 + octant] =
                            brick_index;
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
        Some((node_element_index, modified_nodes, modified_bricks))
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
    /// Provides the index of the first brick available to be overwritten, through the second chance algorithm
    fn first_available_brick(&mut self) -> usize {
        let mut brick_index;
        loop {
            brick_index = self.victim_brick;
            if
            // current brick is not owned or used
            BrickOwnedBy::NotOwned == self.brick_ownership[brick_index]
                || (0
                    == (self.render_data.metadata[brick_index / 8]
                        & Self::brick_used_mask(brick_index)))
            {
                // mark brick used
                self.render_data.metadata[brick_index / 8] |= Self::brick_used_mask(brick_index);
                break;
            }

            // mark current brick unused and step the iterator forward
            self.render_data.metadata[brick_index / 8] &= !Self::brick_used_mask(brick_index);
            self.victim_brick = (brick_index + 1) % (self.render_data.metadata.len() * 8);
        }

        brick_index
    }

    /// Loads a brick into the provided voxels vector and color palette
    /// * `brick` - The brick to upload
    /// * `tree` - The octree where the brick is found
    /// * `returns` - the index where the brick is found and potentially a list of nodes and bricks modified during insertion
    pub(crate) fn add_brick<T, const DIM: usize>(
        &mut self,
        tree: &Octree<T, DIM>,
        node_key: usize,
        target_octant: usize,
    ) -> (u32, Vec<usize>, Vec<usize>)
    where
        T: Default + Clone + PartialEq + VoxelData + Send + Sync + 'static,
    {
        debug_assert_eq!(
            self.render_data.voxels.len() % (DIM * DIM * DIM),
            0,
            "Expected Voxel buffer length({:?}) to be divisble by {:?}",
            self.render_data.voxels.len(),
            (DIM * DIM * DIM)
        );

        let brick = match tree.nodes.get(node_key) {
            NodeContent::UniformLeaf(brick) => brick,
            NodeContent::Leaf(bricks) => &bricks[target_octant],
            NodeContent::Nothing | NodeContent::Internal(_) => {
                panic!("Expected to add brick of Internal or empty node!")
            }
        };

        match brick {
            BrickData::Empty => (empty_marker(), Vec::new(), Vec::new()),
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
                (
                    self.map_to_color_index_in_palette[&albedo] as u32,
                    Vec::new(),
                    Vec::new(),
                )
            }
            BrickData::Parted(brick) => {
                if let Some(brick_index) = self
                    .map_to_brick_maybe_owned_by_node
                    .get(&(node_key, target_octant as u8))
                {
                    if self.brick_ownership[*brick_index] == BrickOwnedBy::NotOwned {
                        self.brick_ownership[*brick_index] =
                            BrickOwnedBy::Node(node_key as u32, target_octant as u8);
                        return (*brick_index as u32, Vec::new(), Vec::new());
                    } else {
                        // remove from index if it is owned by another node already
                        self.map_to_brick_maybe_owned_by_node
                            .remove(&(node_key, target_octant as u8));
                    }
                }

                let brick_index = self.first_available_brick();
                let (modified_nodes, modified_bricks) =
                    if let BrickOwnedBy::Node(key, octant) = self.brick_ownership[brick_index] {
                        debug_assert!(
                            self.node_key_vs_meta_index.contains_left(&(key as usize)),
                            "Expected brick to be owned by a node used in cache"
                        );

                        self.erase_node_child(
                            *self
                                .node_key_vs_meta_index
                                .get_by_left(&(key as usize))
                                .unwrap(),
                            octant as usize,
                            tree,
                        )
                    } else {
                        (Vec::new(), Vec::new())
                    };

                self.brick_ownership[brick_index as usize] =
                    BrickOwnedBy::Node(node_key as u32, target_octant as u8);

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

                (brick_index as u32, modified_nodes, modified_bricks)
            }
        }
    }
}
