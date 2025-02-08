use crate::{
    octree::{
        types::{BrickData, NodeChildren, NodeContent, PaletteIndexValues},
        Octree, OctreeEntry, V3c, VoxelData,
    },
    spatial::{
        lut::{
            BITMAP_INDEX_LUT, BITMAP_MASK_FOR_OCTANT_LUT, OOB_OCTANT,
            RAY_TO_NODE_OCCUPANCY_BITMASK_LUT,
        },
        math::{flat_projection, hash_direction, hash_region, BITMAP_DIMENSION},
        raytracing::{cube_impact_normal, step_octant, Ray, FLOAT_ERROR_TOLERANCE},
        Cube,
    },
};
use std::hash::Hash;

#[derive(Debug)]
pub(crate) struct NodeStack<T, const SIZE: usize = 4> {
    data: [T; SIZE],
    head_index: usize,
    count: u8,
}

impl<T, const SIZE: usize> Default for NodeStack<T, SIZE>
where
    T: Default + Copy,
{
    fn default() -> Self {
        Self {
            data: [T::default(); SIZE],
            head_index: 0,
            count: 0,
        }
    }
}

impl<T, const SIZE: usize> NodeStack<T, SIZE>
where
    T: Default + Copy,
{
    pub(crate) fn is_empty(&self) -> bool {
        0 == self.count
    }

    pub(crate) fn push(&mut self, data: T) {
        self.head_index = (self.head_index + 1) % SIZE;
        self.count = (self.count + 1).min(SIZE as u8);
        self.data[self.head_index] = data;
    }

    pub(crate) fn pop(&mut self) -> Option<T> {
        if 0 == self.count {
            None
        } else {
            self.count -= 1;
            let result = self.data[self.head_index];
            if self.head_index == 0 {
                self.head_index = SIZE - 1;
            } else {
                self.head_index -= 1;
            }
            Some(result)
        }
    }

    pub(crate) fn last(&self) -> Option<&T> {
        if 0 == self.count {
            None
        } else {
            Some(&self.data[self.head_index])
        }
    }
    pub(crate) fn last_mut(&mut self) -> Option<&mut T> {
        if 0 == self.count {
            None
        } else {
            Some(&mut self.data[self.head_index])
        }
    }
}

impl<T> Octree<T>
where
    T: Default + Eq + Clone + Copy + VoxelData + Hash,
{
    pub(crate) fn get_dda_scale_factors(ray: &Ray) -> V3c<f32> {
        V3c::new(
            (1. + (ray.direction.z / ray.direction.x).powf(2.)
                + (ray.direction.y / ray.direction.x).powf(2.))
            .sqrt(),
            ((ray.direction.x / ray.direction.y).powf(2.)
                + 1.
                + (ray.direction.z / ray.direction.y).powf(2.))
            .sqrt(),
            (((ray.direction.x / ray.direction.z).powf(2.) + 1.)
                + (ray.direction.y / ray.direction.z).powf(2.))
            .sqrt(),
        )
    }

    /// https://en.wikipedia.org/wiki/Digital_differential_analyzer_(graphics_algorithm)
    /// Calculate the length of the ray in case the iteration is stepped one unit in the [x/y/z] direction.
    /// Changes with minimum ray iteration length shall be applied
    /// inputs: current distances of the 3 components of the ray, unit size, Ray, scale factors of each xyz components
    /// output: the step to the next sibling
    /// The step is also returned in the given unit size ( based on the cell bounds )
    /// * `ray` - The ray to base the step on
    /// * `ray_current_distance` - The distance the ray iteration is currently at
    /// * `current_bounds` - The cell which boundaries the current ray iteration intersects
    /// * `ray_scale_factors` - Pre-computed dda values for the ray
    pub(crate) fn dda_step_to_next_sibling(
        ray: &Ray,
        ray_current_distance: &mut f32,
        current_bounds: &Cube,
        ray_scale_factors: &V3c<f32>,
    ) -> V3c<f32> {
        let p = ray.point_at(*ray_current_distance);
        let diff_from_min = p - current_bounds.min_position;
        let signum_vec = V3c::new(
            ray.direction.x.signum(),
            ray.direction.y.signum(),
            ray.direction.z.signum(),
        );
        let steps_needed = V3c::new(
            current_bounds.size * signum_vec.x.max(0.) - signum_vec.x * diff_from_min.x,
            current_bounds.size * signum_vec.y.max(0.) - signum_vec.y * diff_from_min.y,
            current_bounds.size * signum_vec.z.max(0.) - signum_vec.z * diff_from_min.z,
        );

        let d_x = *ray_current_distance + (steps_needed.x * ray_scale_factors.x).abs();
        let d_y = *ray_current_distance + (steps_needed.y * ray_scale_factors.y).abs();
        let d_z = *ray_current_distance + (steps_needed.z * ray_scale_factors.z).abs();
        *ray_current_distance = d_x.min(d_y).min(d_z);

        V3c::new(
            if (*ray_current_distance - d_x).abs() < FLOAT_ERROR_TOLERANCE {
                signum_vec.x
            } else {
                0.
            },
            if (*ray_current_distance - d_y).abs() < FLOAT_ERROR_TOLERANCE {
                signum_vec.y
            } else {
                0.
            },
            if (*ray_current_distance - d_z).abs() < FLOAT_ERROR_TOLERANCE {
                signum_vec.z
            } else {
                0.
            },
        )
    }

    /// Iterates on the given ray and brick to find a potential intersection in 3D space
    /// Returns with the 3d and flat index values pointing to the voxel hit inside the brick in case there's a hit
    fn traverse_brick(
        &self,
        ray: &Ray,
        ray_current_distance: &mut f32,
        brick: &[PaletteIndexValues],
        brick_bounds: &Cube,
        brick_dim: usize,
        ray_scale_factors: &V3c<f32>,
    ) -> Option<(V3c<usize>, usize)> {
        // Decide the starting index inside the brick
        let position_in_brick = (ray.point_at(*ray_current_distance) - brick_bounds.min_position)
            * brick_dim as f32
            / brick_bounds.size;
        let mut current_index = V3c::new(
            (position_in_brick.x as i32).clamp(0, (brick_dim - 1) as i32),
            (position_in_brick.y as i32).clamp(0, (brick_dim - 1) as i32),
            (position_in_brick.z as i32).clamp(0, (brick_dim - 1) as i32),
        );
        let flat_delta_x = flat_projection(1, 0, 0, brick_dim) as i32;
        let flat_delta_y = flat_projection(0, 1, 0, brick_dim) as i32;
        let flat_delta_z = flat_projection(0, 0, 1, brick_dim) as i32;
        let mut current_flat_index = flat_projection(
            current_index.x as usize,
            current_index.y as usize,
            current_index.z as usize,
            brick_dim,
        ) as i32;

        // Map the current position to index and bitmap spaces
        let brick_unit = brick_bounds.size / brick_dim as f32; // how long is index step in space (set by the bounds)
        let mut current_bounds = Cube {
            min_position: brick_bounds.min_position + V3c::from(current_index) * brick_unit,
            size: brick_unit,
        };

        // Loop through the brick, terminate if no possibility of hit
        let mut step = V3c::unit(0.);
        loop {
            if
            // If index is out of bounds, there's no hit
            current_index.x < 0
                || current_index.x >= brick_dim as i32
                || current_index.y < 0
                || current_index.y >= brick_dim as i32
                || current_index.z < 0
                || current_index.z >= brick_dim as i32
            {
                return None;
            }

            current_flat_index += step.x as i32 * flat_delta_x
                + step.y as i32 * flat_delta_y
                + step.z as i32 * flat_delta_z;
            debug_assert_eq!(
                flat_projection(
                    current_index.x as usize,
                    current_index.y as usize,
                    current_index.z as usize,
                    brick_dim,
                ),
                current_flat_index as usize
            );

            if !NodeContent::pix_points_to_empty(
                &brick[current_flat_index as usize],
                &self.voxel_color_palette,
                &self.voxel_data_palette,
            ) {
                return Some((
                    V3c::<usize>::from(current_index),
                    current_flat_index as usize,
                ));
            }

            step = Self::dda_step_to_next_sibling(
                ray,
                ray_current_distance,
                &current_bounds,
                ray_scale_factors,
            );
            current_bounds.min_position += step * brick_unit;
            current_index += V3c::<i32>::from(step);

            #[cfg(debug_assertions)]
            {
                // Check if the resulting point is inside bounds still
                let relative_point =
                    ray.point_at(*ray_current_distance) - current_bounds.min_position;
                debug_assert!(
                    (relative_point.x < FLOAT_ERROR_TOLERANCE
                        || (relative_point.x - current_bounds.size) < FLOAT_ERROR_TOLERANCE)
                        || (relative_point.y < FLOAT_ERROR_TOLERANCE
                            || (relative_point.y - current_bounds.size) < FLOAT_ERROR_TOLERANCE)
                        || (relative_point.z < FLOAT_ERROR_TOLERANCE
                            || (relative_point.z - current_bounds.size) < FLOAT_ERROR_TOLERANCE)
                );
            }
        }
    }

    fn probe_brick(
        &self,
        ray: &Ray,
        ray_current_distance: &mut f32,
        brick: &BrickData<PaletteIndexValues>,
        brick_bounds: &Cube,
        ray_scale_factors: &V3c<f32>,
    ) -> Option<(OctreeEntry<T>, V3c<f32>, V3c<f32>)> {
        match brick {
            BrickData::Empty => {
                // No need to do anything, iteration continues with "leaf miss"
                None
            }
            BrickData::Solid(voxel) => {
                let impact_point = ray.point_at(*ray_current_distance);
                Some((
                    NodeContent::pix_get_ref(
                        voxel,
                        &self.voxel_color_palette,
                        &self.voxel_data_palette,
                    ),
                    impact_point,
                    cube_impact_normal(brick_bounds, &impact_point),
                ))
            }
            BrickData::Parted(brick) => {
                if let Some((leaf_brick_hit, leaf_brick_hit_flat_index)) = self.traverse_brick(
                    ray,
                    ray_current_distance,
                    brick,
                    brick_bounds,
                    self.brick_dim as usize,
                    ray_scale_factors,
                ) {
                    let hit_bounds = Cube {
                        size: brick_bounds.size / self.brick_dim as f32,
                        min_position: brick_bounds.min_position
                            + V3c::<f32>::from(leaf_brick_hit) * brick_bounds.size
                                / self.brick_dim as f32,
                    };
                    let impact_point = ray.point_at(*ray_current_distance);
                    let impact_normal = cube_impact_normal(&hit_bounds, &impact_point);
                    Some((
                        NodeContent::pix_get_ref(
                            &brick[leaf_brick_hit_flat_index],
                            &self.voxel_color_palette,
                            &self.voxel_data_palette,
                        ),
                        impact_point,
                        impact_normal,
                    ))
                } else {
                    None
                }
            }
        }
    }

    /// provides the collision point of the ray with the contained voxel field
    /// return reference of the data, collision point and normal at impact, should there be any
    pub fn get_by_ray(&self, ray: &Ray) -> Option<(OctreeEntry<T>, V3c<f32>, V3c<f32>)> {
        // Pre-calculated optimization variables
        let ray_scale_factors = Self::get_dda_scale_factors(ray);
        let direction_lut_index = hash_direction(&ray.direction) as usize;

        let mut node_stack: NodeStack<u32> = NodeStack::default();
        let mut current_bounds = Cube::root_bounds(self.octree_size as f32);
        let (mut ray_current_distance, mut target_octant) =
            if let Some(root_hit) = current_bounds.intersect_ray(ray) {
                let ray_current_distance = root_hit.impact_distance.unwrap_or(0.);
                (
                    ray_current_distance,
                    hash_region(
                        &(ray.point_at(ray_current_distance) - current_bounds.min_position),
                        current_bounds.size / 2.,
                    ),
                )
            } else {
                (0., OOB_OCTANT)
            };
        let mut current_node_key: usize;
        let mut step_vec = V3c::unit(0.);

        while target_octant != OOB_OCTANT {
            current_node_key = Self::ROOT_NODE_KEY as usize;
            current_bounds = Cube::root_bounds(self.octree_size as f32);
            node_stack.push(Self::ROOT_NODE_KEY);
            while !node_stack.is_empty() {
                let current_node_occupied_bits =
                    self.stored_occupied_bits(*node_stack.last().unwrap() as usize);
                debug_assert!(self
                    .nodes
                    .key_is_valid(*node_stack.last().unwrap() as usize));

                let mut do_backtrack_after_leaf_miss = false;
                if target_octant != OOB_OCTANT {
                    match self.nodes.get(current_node_key) {
                        NodeContent::UniformLeaf(brick) => {
                            debug_assert!(matches!(
                                self.node_children[current_node_key],
                                NodeChildren::OccupancyBitmap(_)
                            ));
                            if let Some(hit) = self.probe_brick(
                                ray,
                                &mut ray_current_distance,
                                brick,
                                &current_bounds,
                                &ray_scale_factors,
                            ) {
                                return Some(hit);
                            }
                            do_backtrack_after_leaf_miss = true;
                        }
                        NodeContent::Leaf(bricks) => {
                            debug_assert!(matches!(
                                self.node_children[current_node_key],
                                NodeChildren::OccupancyBitmap(_)
                            ));
                            if let Some(hit) = self.probe_brick(
                                ray,
                                &mut ray_current_distance,
                                &bricks[target_octant as usize],
                                &current_bounds.child_bounds_for(target_octant),
                                &ray_scale_factors,
                            ) {
                                return Some(hit);
                            }
                        }
                        NodeContent::Internal(_) | NodeContent::Nothing => {}
                    }
                };

                // the position of the current iteration inside the current bounds in bitmap dimensions
                let mut bitmap_pos_in_node = (ray.point_at(ray_current_distance)
                    - current_bounds.min_position)
                    * BITMAP_DIMENSION as f32
                    / current_bounds.size;
                bitmap_pos_in_node = V3c::new(
                    (bitmap_pos_in_node.x).clamp(FLOAT_ERROR_TOLERANCE, 4. - FLOAT_ERROR_TOLERANCE),
                    (bitmap_pos_in_node.y).clamp(FLOAT_ERROR_TOLERANCE, 4. - FLOAT_ERROR_TOLERANCE),
                    (bitmap_pos_in_node.z).clamp(FLOAT_ERROR_TOLERANCE, 4. - FLOAT_ERROR_TOLERANCE),
                );
                let mut flat_pos_in_bitmap = BITMAP_INDEX_LUT
                    [bitmap_pos_in_node.x.floor() as usize]
                    [bitmap_pos_in_node.y.floor() as usize]
                    [bitmap_pos_in_node.z.floor() as usize];

                if do_backtrack_after_leaf_miss
                    || target_octant == OOB_OCTANT
                    // The current Node is empty
                    || 0 == current_node_occupied_bits
                    // There is no overlap between node occupancy and the area the ray potentially hits
                    || 0 == (current_node_occupied_bits & RAY_TO_NODE_OCCUPANCY_BITMASK_LUT[flat_pos_in_bitmap][direction_lut_index])
                {
                    // POP
                    node_stack.pop();
                    step_vec = Self::dda_step_to_next_sibling(
                        ray,
                        &mut ray_current_distance,
                        &current_bounds,
                        &ray_scale_factors,
                    );
                    if let Some(parent) = node_stack.last_mut() {
                        current_node_key = *parent as usize;
                        let current_bound_center =
                            current_bounds.min_position + V3c::unit(current_bounds.size / 2.);
                        let parent_bound_min_position = current_bounds.min_position
                            - *current_bounds
                                .min_position
                                .clone()
                                .modulo(&(current_bounds.size * 2.));
                        target_octant = step_octant(
                            hash_region(
                                &(current_bound_center - parent_bound_min_position),
                                current_bounds.size,
                            ),
                            step_vec,
                        );
                        current_bounds.size *= 2.;
                        current_bounds.min_position = parent_bound_min_position;
                        debug_assert!(current_bounds.size <= self.octree_size as f32);
                    }
                    continue; // Restart loop with the parent Node
                              // Eliminating this `continue` causes significant slowdown in GPU
                }

                let mut target_bounds = current_bounds.child_bounds_for(target_octant);
                let mut target_child_key =
                    self.node_children[current_node_key].child(target_octant) as u32;
                if self.nodes.key_is_valid(target_child_key as usize)
                    && 0 != (current_node_occupied_bits
                        & BITMAP_MASK_FOR_OCTANT_LUT[target_octant as usize])
                {
                    // PUSH
                    current_node_key = target_child_key as usize;
                    current_bounds = target_bounds;
                    target_octant = hash_region(
                        &(ray.point_at(ray_current_distance) - target_bounds.min_position),
                        target_bounds.size / 2.,
                    );
                    node_stack.push(target_child_key);
                } else {
                    // ADVANCE
                    // target child is invalid, or it does not intersect with the ray,
                    // so advance iteration to the next sibling
                    loop {
                        // step the iteration to the next sibling cell!
                        step_vec = Self::dda_step_to_next_sibling(
                            ray,
                            &mut ray_current_distance,
                            &target_bounds,
                            &ray_scale_factors,
                        );
                        target_octant = step_octant(target_octant, step_vec);
                        if OOB_OCTANT != target_octant {
                            target_bounds = current_bounds.child_bounds_for(target_octant);
                            target_child_key =
                                self.node_children[current_node_key].child(target_octant) as u32;
                            bitmap_pos_in_node += step_vec * 4. / current_bounds.size;
                            flat_pos_in_bitmap = BITMAP_INDEX_LUT
                                [bitmap_pos_in_node.x.floor() as usize]
                                [bitmap_pos_in_node.y.floor() as usize]
                                [bitmap_pos_in_node.z.floor() as usize];
                        }
                        if target_octant == OOB_OCTANT
                        // In case the current internal node has a valid target child
                        || (self.nodes.key_is_valid(target_child_key as usize)
                            // current node is occupied at target octant
                            && 0 != current_node_occupied_bits & BITMAP_MASK_FOR_OCTANT_LUT[target_octant as usize]
                            //  target child is in the area the ray can potentially hit
                            && 0 != (RAY_TO_NODE_OCCUPANCY_BITMASK_LUT[flat_pos_in_bitmap][direction_lut_index]
                                & current_node_occupied_bits)
                            )
                            // In case the current node is leaf
                            || match self.nodes.get(current_node_key) {
                                    // Empty or internal nodes are not evaluated in this condition;
                                    // Basically if there's no hit with a uniform leaf
                                    // | It's either because the leaf is solid empty
                                    // | Or the parted brick did not have any non-empty voxels intersecting with the ray
                                    // --> Both reasons are valid to go forward, so don't break the advancement
                                NodeContent::Nothing | NodeContent::Internal(_) | NodeContent::UniformLeaf(_) => false,
                                NodeContent::Leaf(bricks) => {
                                    // Stop advancement if brick under target octant is not empty
                                    !matches!(bricks[target_octant as usize], BrickData::Empty)
                                }
                            }
                        {
                            // stop advancing because current target is either
                            // - OOB
                            // - or (not empty while inside bounds AND collides with the ray based on its occupancy bitmap)
                            break;
                        }
                    }
                }
            }

            // POP on empty stack happened, which means iteration must continue from root
            // To avoid precision problems the current bound center
            // pushed along the last step is used for reference
            let current_octant_center = current_bounds.min_position
                + V3c::unit(current_bounds.size / 2.)
                + step_vec * current_bounds.size;
            target_octant = if current_octant_center.x < self.octree_size as f32
                && current_octant_center.y < self.octree_size as f32
                && current_octant_center.z < self.octree_size as f32
                && current_octant_center.x > 0.
                && current_octant_center.y > 0.
                && current_octant_center.z > 0.
            {
                hash_region(&current_octant_center, self.octree_size as f32 / 2.)
            } else {
                OOB_OCTANT
            };
        }
        None
    }
}
