use crate::{
    object_pool::empty_marker,
    octree::{
        types::{BrickData, NodeContent},
        Octree, VoxelData,
    },
    raytracing::bevy::types::{
        BrickOwnedBy, BrickUpdate, CacheUpdatePackage, OctreeGPUDataHandler, OctreeRenderData,
        VictimPointer,
    },
    spatial::lut::OOB_OCTANT,
};
use bendy::{decoding::FromBencode, encoding::ToBencode};
use std::{hash::Hash, ops::Range};

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
    /// Optionally the source where the child can be taken from.
    /// And finally the index range where node modified was updated
    fn first_available_node(
        &mut self,
        render_data: &mut OctreeRenderData,
    ) -> (usize, Option<(usize, u8)>, Range<usize>) {
        // If there is space left in the cache, use it all up
        if !self.is_full() {
            render_data.metadata[self.stored_items] |= OctreeGPUDataHandler::NODE_USED_MASK;
            self.meta_index = self.stored_items;
            self.stored_items += 1;
            return (
                self.meta_index,
                None,
                self.meta_index..(self.meta_index + 1),
            );
        }

        //look for the next internal node ( with node children )
        let mut modified_range = self.meta_index..(self.meta_index + 1);
        loop {
            modified_range.start = modified_range.start.min(self.meta_index);
            modified_range.end = modified_range.end.max(self.meta_index + 1);

            // child at target is not empty in a non-leaf node, which means
            // the target child might point to an internal node if it's valid
            if 0 == (render_data.metadata[self.meta_index] & OctreeGPUDataHandler::NODE_LEAF_MASK)
                && render_data.node_children[self.meta_index * 8 + self.child]
                    != empty_marker::<u32>()
            {
                let child_meta_index =
                    render_data.node_children[self.meta_index * 8 + self.child] as usize;
                debug_assert!(
                    child_meta_index < render_data.metadata.len(),
                    "Expected children {:?} of node[{:?}]({:#10X}) to point inside metadata index",
                    [
                        render_data.node_children[self.meta_index * 8],
                        render_data.node_children[self.meta_index * 8 + 1],
                        render_data.node_children[self.meta_index * 8 + 2],
                        render_data.node_children[self.meta_index * 8 + 3],
                        render_data.node_children[self.meta_index * 8 + 4],
                        render_data.node_children[self.meta_index * 8 + 5],
                        render_data.node_children[self.meta_index * 8 + 6],
                        render_data.node_children[self.meta_index * 8 + 7],
                    ],
                    self.meta_index,
                    render_data.metadata[self.meta_index],
                );
                if 0 == (render_data.metadata[child_meta_index]
                    & OctreeGPUDataHandler::NODE_USED_MASK)
                {
                    render_data.metadata[child_meta_index] |= OctreeGPUDataHandler::NODE_USED_MASK;
                    return (
                        child_meta_index,
                        Some((self.meta_index, self.child as u8)),
                        modified_range,
                    );
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

    /// Bitmask in metadata set to 1 if the node has MIP
    const NODE_HAS_MIP_MASK: u32 = 0x00000010;

    /// Bitmask in metadata set to 1 if the node MIP is parted
    const NODE_MIP_PARTED_MASK: u32 = 0x00000020;

    /// Provides the mask used with one metadata element to signal that the contained brick is used.
    /// Index of the metadata element should be brick index divided by 8, as one metadata element contains 8 bricks
    fn brick_used_mask(brick_index: usize) -> u32 {
        0x01 << (24 + (brick_index % 8))
    }

    /// Returns true if child is of parted brick structure absed on the given metadata
    fn child_is_parted(meta: u32, child_octant: u8) -> bool {
        0 != (meta & (0x01 << (child_octant + 16)))
    }

    /// Creates the descriptor bytes for the given node
    fn create_node_properties<T>(tree: &Octree<T>, node_key: usize) -> u32
    where
        T: Default + Clone + Eq + VoxelData + Hash,
    {
        let mut meta = 0;

        // set node MIP properties
        if let BrickData::Solid(_) | BrickData::Parted(_) = tree.node_mips[node_key] {
            meta |= Self::NODE_HAS_MIP_MASK;
            if let BrickData::Parted(_) = tree.node_mips[node_key] {
                meta |= Self::NODE_MIP_PARTED_MASK;
            }
        }

        // set node type
        match tree.nodes.get(node_key) {
            NodeContent::Internal(_) | NodeContent::Nothing => {
                meta &= !Self::NODE_LEAF_MASK; // element is not leaf
                meta &= !Self::NODE_UNIFORM_MASK; // element is not uniform
            }
            NodeContent::Leaf(bricks) => {
                meta |= Self::NODE_LEAF_MASK; // element is leaf
                meta &= !Self::NODE_UNIFORM_MASK; // element is not uniform

                // set child Structure bits
                for octant in 0..8 {
                    match &bricks[octant] {
                        BrickData::Empty | BrickData::Solid(_) => {} // Child structure properties should already be set to NIL
                        BrickData::Parted(_brick) => {
                            meta |= 0x01 << (16 + octant);
                        }
                    };
                }
            }
            NodeContent::UniformLeaf(brick) => {
                meta |= Self::NODE_LEAF_MASK; // element is leaf
                meta |= Self::NODE_UNIFORM_MASK; // element is uniform

                // set child Structure bits
                match brick {
                    BrickData::Empty | BrickData::Solid(_) => {} // Child structure properties should already be set to NIL
                    BrickData::Parted(_brick) => {
                        meta |= 0x01 << 16;
                    }
                };
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
    /// returns with the vector of brick updates and node index values modified
    fn erase_node_child<'a, T>(
        &mut self,
        meta_index: usize,
        child_octant: usize,
        tree: &'a Octree<T>,
    ) -> (Vec<BrickUpdate<'a>>, Vec<usize>)
    where
        T: Default + Clone + Eq + VoxelData + Hash,
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
        self.render_data.node_children[meta_index * 8 + child_octant] = empty_marker::<u32>();
        debug_assert_ne!(
            child_index,
            empty_marker::<u32>() as usize,
            "Expected erased child of node[{}](meta[{}]) to be an erasable node/brick",
            parent_key,
            meta_index
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

                // Erase MIP connection, Erase ownership as well
                let child_mip = self.render_data.node_mips[child_index];
                if child_mip != empty_marker::<u32>() {
                    self.render_data.node_mips[child_index] = empty_marker();
                    if matches!(tree.node_mips[*child_key], BrickData::Parted(_)) {
                        self.brick_ownership[child_mip as usize] = BrickOwnedBy::NotOwned;
                    }
                }

                // Free up child bricks if erased child node was a leaf
                if let NodeContent::Leaf(_) | NodeContent::UniformLeaf(_) =
                    tree.nodes.get(*child_key)
                {
                    // make the children bricks of the removed leaf orphan
                    for octant in 0..8 {
                        let brick_index =
                            self.render_data.node_children[child_index * 8 + octant] as usize;

                        if (brick_index != empty_marker::<u32>() as usize)
                            && Self::child_is_parted(
                                self.render_data.metadata[child_index],
                                octant as u8,
                            )
                        {
                            self.brick_ownership[brick_index] = BrickOwnedBy::NotOwned;

                            // No need to eliminate child connections
                            // as they will be overwritten later(expected to be overwritten in @add_node)
                            // --> as internal nodes are expected to be only deleted by the fn @add_node
                            // Just mark bricks as unused
                            self.render_data.metadata[brick_index / 8] &=
                                !Self::brick_used_mask(brick_index);

                            modified_bricks.push(BrickUpdate {
                                brick_index,
                                data: None,
                            });
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
                if child_index != empty_marker::<u32>() as usize {
                    // update ownership of bricks, remove from hints as well
                    self.brick_ownership[child_index] = BrickOwnedBy::NotOwned;

                    modified_bricks.push(BrickUpdate {
                        brick_index: child_index,
                        data: None,
                    });

                    // mark brick as unused
                    self.render_data.metadata[child_index / 8] &=
                        !Self::brick_used_mask(child_index);
                }
            }
        }

        //return with updated ranges in voxels and metadata
        (modified_bricks, modified_nodes)
    }

    //##############################################################################
    //  ██████   █████    ███████    ██████████   ██████████
    // ░░██████ ░░███   ███░░░░░███ ░░███░░░░███ ░░███░░░░░█
    //  ░███░███ ░███  ███     ░░███ ░███   ░░███ ░███  █ ░
    //  ░███░░███░███ ░███      ░███ ░███    ░███ ░██████
    //  ░███ ░░██████ ░███      ░███ ░███    ░███ ░███░░█
    //  ░███  ░░█████ ░░███     ███  ░███    ███  ░███ ░   █
    //  █████  ░░█████ ░░░███████░   ██████████   ██████████
    // ░░░░░    ░░░░░    ░░░░░░░    ░░░░░░░░░░   ░░░░░░░░░░
    //##############################################################################
    /// Writes the data of the given node to the first available index
    /// * `returns` - Upon success, returns (meta_index, brick updates, modified_nodes)
    pub(crate) fn add_node<
        'a,
        #[cfg(all(feature = "bytecode", feature = "serialization"))] T: FromBencode
            + ToBencode
            + Serialize
            + DeserializeOwned
            + Default
            + Eq
            + Clone
            + Hash
            + VoxelData
            + Send
            + Sync
            + 'static,
        #[cfg(all(feature = "bytecode", not(feature = "serialization")))] T: FromBencode + ToBencode + Default + Eq + Clone + Hash + VoxelData + Send + Sync + 'static,
        #[cfg(all(not(feature = "bytecode"), feature = "serialization"))] T: Serialize
            + DeserializeOwned
            + Default
            + Eq
            + Clone
            + Hash
            + VoxelData
            + Send
            + Sync
            + 'static,
        #[cfg(all(not(feature = "bytecode"), not(feature = "serialization")))] T: Default + Eq + Clone + Hash + VoxelData,
    >(
        &mut self,
        tree: &'a Octree<T>,
        node_key: usize,
    ) -> (usize, CacheUpdatePackage<'a>) {
        debug_assert!(
            !self.node_key_vs_meta_index.contains_left(&node_key),
            "Trying to add already available node twice!"
        );

        // Determine the index in meta, overwrite a currently present node if needed
        let (node_element_index, robbed_parent, modified_node_range) =
            self.victim_node.first_available_node(&mut self.render_data);
        let (modified_bricks, modified_nodes) = if let Some(robbed_parent) = robbed_parent {
            debug_assert_eq!(
                self.render_data.node_children[robbed_parent.0 * 8 + robbed_parent.1 as usize]
                    as usize,
                node_element_index,
                "Expected child[{:?}] of node[{:?}] to be node[{:?}]!",
                robbed_parent.1,
                robbed_parent.0,
                node_element_index
            );
            self.erase_node_child(robbed_parent.0, robbed_parent.1 as usize, tree)
        } else {
            (Vec::new(), vec![node_element_index])
        };

        self.node_key_vs_meta_index
            .insert(node_key, node_element_index);

        // Add node properties to metadata
        self.render_data.metadata[node_element_index] &= 0xFF000000;
        self.render_data.metadata[node_element_index] |=
            Self::create_node_properties(tree, node_key);

        // Update occupancy in ocbits
        let occupied_bits = tree.stored_occupied_bits(node_key);
        self.render_data.node_ocbits[node_element_index * 2] =
            (occupied_bits & 0x00000000FFFFFFFF) as u32;
        self.render_data.node_ocbits[node_element_index * 2 + 1] =
            ((occupied_bits & 0xFFFFFFFF00000000) >> 32) as u32;

        // Add empty children
        self.render_data.node_children.splice(
            (node_element_index * 8)..(node_element_index * 8 + 8),
            vec![empty_marker::<u32>(); 8],
        );

        // Add MIP entry
        self.render_data.node_mips[node_element_index] = match tree.node_mips[node_key] {
            BrickData::Solid(voxel) => voxel, // In case MIP is solid, it is pointing to the color palette
            BrickData::Empty | BrickData::Parted(_) => empty_marker(), // parted bricks need to be uploaded; empty MIPS are stored with empty_marker
        };

        // Add child nodes if any is available
        match tree.nodes.get(node_key) {
            NodeContent::Nothing => {}
            NodeContent::Internal(_) => {
                for octant in 0..8 {
                    let child_key = tree.node_children[node_key].child(octant);
                    if child_key != empty_marker::<u32>() as usize {
                        self.render_data.node_children[node_element_index * 8 + octant as usize] =
                            *self
                                .node_key_vs_meta_index
                                .get_by_left(&child_key)
                                .unwrap_or(&(empty_marker::<u32>() as usize))
                                as u32;
                    } else {
                        self.render_data.node_children[node_element_index * 8 + octant as usize] =
                            empty_marker::<u32>();
                    }
                }
            }
            NodeContent::UniformLeaf(brick) => {
                if let BrickData::Solid(voxel) = brick {
                    self.render_data.node_children[node_element_index * 8] = *voxel;
                } else {
                    self.render_data.node_children[node_element_index * 8] = empty_marker::<u32>();
                }
            }
            NodeContent::Leaf(bricks) => {
                for octant in 0..8 {
                    if let BrickData::Solid(voxel) = bricks[octant] {
                        self.render_data.node_children[node_element_index * 8 + octant] = voxel;
                    } else {
                        self.render_data.node_children[node_element_index * 8 + octant] =
                            empty_marker::<u32>();
                    }
                }
            }
        }
        (
            node_element_index,
            CacheUpdatePackage {
                brick_updates: modified_bricks,
                modified_usage_range: modified_node_range,
                modified_nodes,
            },
        )
    }

    //##############################################################################
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
    /// * `returns` - The index of the first erasable brick inside the cache and the range of bricks updated
    fn first_available_brick(&mut self) -> (usize, Range<usize>) {
        let mut brick_index;
        let mut index_range = self.victim_brick..(self.victim_brick + 1);
        loop {
            brick_index = self.victim_brick;
            index_range.start = index_range.start.min(brick_index);
            index_range.end = index_range.end.max(brick_index + 1);
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

        (brick_index, index_range)
    }

    /// Makes space for the requested brick and updates brick ownership if needed
    /// * `tree` - The octree where the brick is found
    /// * `node_key` - The key for the requested leaf node, whoose child needs to be uploaded
    /// * `target_octant` - The octant where the target brick lies
    /// * `returns` - brick/palette index where the brick data is found, brick updates applied, nodes updated during insertion
    pub(crate) fn add_brick<'a, T>(
        &mut self,
        tree: &'a Octree<T>,
        node_key: usize,
        target_octant: u8,
    ) -> (usize, CacheUpdatePackage<'a>)
    where
        T: Default + Clone + Eq + Send + Sync + Hash + VoxelData + 'static,
    {
        // In case OOB octant, the target brick to add is the MIP for the node
        let (brick, node_entry) = if target_octant == OOB_OCTANT {
            (
                &tree.node_mips[node_key],
                BrickOwnedBy::NodeAsMIP(node_key as u32),
            )
        } else {
            (
                match tree.nodes.get(node_key) {
                    NodeContent::UniformLeaf(brick) => brick,
                    NodeContent::Leaf(bricks) => &bricks[target_octant as usize],
                    NodeContent::Nothing | NodeContent::Internal(_) => {
                        panic!("Trying to add brick from Internal or empty node!")
                    }
                },
                BrickOwnedBy::NodeAsChild(node_key as u32, target_octant),
            )
        };

        match brick {
            BrickData::Empty => (
                empty_marker::<u32>() as usize,
                CacheUpdatePackage::default(),
            ),
            BrickData::Solid(_voxel) => unreachable!("Shouldn't try to upload solid MIP bricks"),
            BrickData::Parted(brick) => {
                let (brick_index, modified_brick_range) = self.first_available_brick();
                let (mut modified_bricks, modified_nodes) = match self.brick_ownership[brick_index]
                {
                    BrickOwnedBy::NodeAsChild(key, octant) => {
                        debug_assert!(
                            self.node_key_vs_meta_index.contains_left(&(key as usize)),
                            "Expected brick[{}] to be owned by a node used in cache. Node key: {}",
                            brick_index,
                            key
                        );
                        self.erase_node_child(
                            *self
                                .node_key_vs_meta_index
                                .get_by_left(&(key as usize))
                                .unwrap(),
                            octant as usize,
                            tree,
                        )
                    }
                    BrickOwnedBy::NodeAsMIP(key) => {
                        debug_assert!(
                            self.node_key_vs_meta_index.contains_left(&(key as usize)),
                            "Expected brick to be owned by a node used in cache"
                        );

                        // erase MIP from node
                        let robbed_meta_index = *self
                            .node_key_vs_meta_index
                            .get_by_left(&(key as usize))
                            .unwrap();
                        self.render_data.node_mips[robbed_meta_index] = empty_marker();
                        (Vec::new(), vec![robbed_meta_index])
                    }
                    BrickOwnedBy::NotOwned => (Vec::new(), Vec::new()),
                };

                self.brick_ownership[brick_index] = node_entry;

                debug_assert_eq!(
                    tree.brick_dim.pow(3) as usize,
                    brick.len(),
                    "Expected Brick slice to align to tree brick dimension"
                );
                modified_bricks.push(BrickUpdate {
                    brick_index,
                    data: Some(&brick[..]),
                });

                (
                    brick_index,
                    CacheUpdatePackage {
                        brick_updates: modified_bricks,
                        modified_usage_range: Range {
                            start: modified_brick_range.start / 8,
                            end: modified_brick_range.end / 8,
                        },
                        modified_nodes,
                    },
                )
            }
        }
    }
}
