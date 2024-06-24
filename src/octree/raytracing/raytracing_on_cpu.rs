use crate::{
    octree::{
        types::{NodeChildrenArray, NodeContent},
        Cube, Octree, V3c, VoxelData,
    },
    spatial::math::step_octant,
};

use crate::spatial::{
    math::{
        cube_impact_normal, flat_projection, hash_direction, hash_region, octant_bitmask,
        position_in_bitmap_64bits,
    },
    raytracing::{
        lut::{OOB_OCTANT, RAY_TO_LEAF_OCCUPANCY_BITMASK_LUT, RAY_TO_NODE_OCCUPANCY_BITMASK_LUT},
        Ray,
    },
    FLOAT_ERROR_TOLERANCE,
};

#[derive(Debug)]
struct NodeStackItem {
    pub(crate) bounds: Cube,
    node: u32,
    target_octant: u8,
}

impl NodeStackItem {
    pub(crate) fn new(bounds: Cube, node: u32, target_octant: u8) -> Self {
        Self {
            bounds,
            node,
            target_octant,
        }
    }
}

impl<T: Default + PartialEq + Clone + std::fmt::Debug + VoxelData, const DIM: usize>
    Octree<T, DIM>
{
    pub(in crate::octree) fn get_dda_scale_factors(ray: &Ray) -> V3c<f32> {
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
    /// Calculate the length of the ray should its iteration be stepped one unit in the [x/y/z] direction.
    /// Changes with minimum ray iteration length shall be applied
    /// The step is also returned in the given unit size ( based on the cell bounds )
    /// * `ray` - The ray to base the step on
    /// * `ray_current_distance` - The distance the ray iteration is currently at
    /// * `current_bounds` - The cell which boundaries the current ray iteration intersects
    /// * `ray_scale_factors` - Pre-computed dda values for the ray
    /// inputs: current distances of the 3 components of the ray, unit size, Ray, scale factors of each xyz components
    /// output: the step to the next sibling
    pub(in crate::octree) fn dda_step_to_next_sibling(
        ray: &Ray,
        ray_current_distance: &mut f32,
        current_bounds: &Cube,
        ray_scale_factors: &V3c<f32>,
    ) -> V3c<f32> {
        let p = ray.point_at(*ray_current_distance);
        let steps_needed = V3c::new(
            p.x - current_bounds.min_position.x as f32
                - (current_bounds.size as f32 * ray.direction.x.signum().max(0.)),
            p.y - current_bounds.min_position.y as f32
                - (current_bounds.size as f32 * ray.direction.y.signum().max(0.)),
            p.z - current_bounds.min_position.z as f32
                - (current_bounds.size as f32 * ray.direction.z.signum().max(0.)),
        );

        let d_x = *ray_current_distance + (steps_needed.x * ray_scale_factors.x).abs();
        let d_y = *ray_current_distance + (steps_needed.y * ray_scale_factors.y).abs();
        let d_z = *ray_current_distance + (steps_needed.z * ray_scale_factors.z).abs();
        *ray_current_distance = d_x.min(d_y).min(d_z);

        V3c::new(
            if (*ray_current_distance - d_x).abs() < FLOAT_ERROR_TOLERANCE {
                (current_bounds.size as f32).copysign(ray.direction.x)
            } else {
                0.
            },
            if (*ray_current_distance - d_y).abs() < FLOAT_ERROR_TOLERANCE {
                (current_bounds.size as f32).copysign(ray.direction.y)
            } else {
                0.
            },
            if (*ray_current_distance - d_z).abs() < FLOAT_ERROR_TOLERANCE {
                (current_bounds.size as f32).copysign(ray.direction.z)
            } else {
                0.
            },
        )
    }

    const UNIT_IN_BITMAP_SPACE: f32 = 4. / DIM as f32;
    /// Iterates on the given ray and brick to find a potential intersection in 3D space
    fn traverse_brick(
        ray: &Ray,
        ray_current_distance: &mut f32,
        brick: &[[[T; DIM]; DIM]; DIM],
        brick_occupied_bits: u64,
        bounds: &Cube,
        ray_scale_factors: &V3c<f32>,
        direction_lut_index: usize,
    ) -> Option<V3c<usize>> {
        let mut current_index = {
            let pos = ray.point_at(*ray_current_distance) - bounds.min_position;
            V3c::new(
                (pos.x as i32).clamp(0, (DIM - 1) as i32),
                (pos.y as i32).clamp(0, (DIM - 1) as i32),
                (pos.z as i32).clamp(0, (DIM - 1) as i32),
            )
        };
        let brick_unit = bounds.size / DIM as f32;
        let mut current_bounds = Cube {
            min_position: bounds.min_position + V3c::from(current_index) * brick_unit,
            size: brick_unit,
        };
        let start_pos_in_bitmap = position_in_bitmap_64bits(
            current_index.x as usize,
            current_index.y as usize,
            current_index.z as usize,
            DIM,
        );

        if 0 == (RAY_TO_LEAF_OCCUPANCY_BITMASK_LUT[start_pos_in_bitmap][direction_lut_index]
            & brick_occupied_bits)
        {
            return None;
        }

        let mut prev_bitmap_position_full_resolution = V3c::new(
            (current_index.x as f32 * Self::UNIT_IN_BITMAP_SPACE) as usize,
            (current_index.y as f32 * Self::UNIT_IN_BITMAP_SPACE) as usize,
            (current_index.z as f32 * Self::UNIT_IN_BITMAP_SPACE) as usize,
        );
        loop {
            if current_index.x < 0
                || current_index.x >= DIM as i32
                || current_index.y < 0
                || current_index.y >= DIM as i32
                || current_index.z < 0
                || current_index.z >= DIM as i32
            {
                return None;
            }

            let bitmap_position_full_resolution = V3c::new(
                (current_index.x as f32 * Self::UNIT_IN_BITMAP_SPACE) as usize,
                (current_index.y as f32 * Self::UNIT_IN_BITMAP_SPACE) as usize,
                (current_index.z as f32 * Self::UNIT_IN_BITMAP_SPACE) as usize,
            );
            if bitmap_position_full_resolution != prev_bitmap_position_full_resolution {
                prev_bitmap_position_full_resolution = bitmap_position_full_resolution;
                let start_pos_in_bitmap = flat_projection(
                    bitmap_position_full_resolution.x as usize,
                    bitmap_position_full_resolution.y as usize,
                    bitmap_position_full_resolution.z as usize,
                    4,
                );
                if 0 == (RAY_TO_LEAF_OCCUPANCY_BITMASK_LUT[start_pos_in_bitmap]
                    [direction_lut_index]
                    & brick_occupied_bits)
                {
                    return None;
                }
            }

            if !brick[current_index.x as usize][current_index.y as usize][current_index.z as usize]
                .is_empty()
            {
                return Some(V3c::<usize>::from(current_index));
            }

            let step = Self::dda_step_to_next_sibling(
                ray,
                ray_current_distance,
                &current_bounds,
                ray_scale_factors,
            );
            current_bounds.min_position = current_bounds.min_position + step;
            current_index = current_index + V3c::<i32>::from(step);
            #[cfg(debug_assertions)]
            {
                let relative_point =
                    ray.point_at(*ray_current_distance) - V3c::from(current_bounds.min_position);
                debug_assert!(
                    (relative_point.x < FLOAT_ERROR_TOLERANCE
                        || (relative_point.x - current_bounds.size as f32) < FLOAT_ERROR_TOLERANCE)
                        || (relative_point.y < FLOAT_ERROR_TOLERANCE
                            || (relative_point.y - current_bounds.size as f32)
                                < FLOAT_ERROR_TOLERANCE)
                        || (relative_point.z < FLOAT_ERROR_TOLERANCE
                            || (relative_point.z - current_bounds.size as f32)
                                < FLOAT_ERROR_TOLERANCE)
                );
            }
        }
    }

    /// provides the collision point of the ray with the contained voxel field
    /// return reference of the data, collision point and normal at impact, should there be any
    pub fn get_by_ray(&self, ray: &Ray) -> Option<(&T, V3c<f32>, V3c<f32>)> {
        let ray = Ray {
            origin: ray.origin,
            direction: V3c::new(
                if 0. != ray.direction.x {
                    ray.direction.x
                } else {
                    FLOAT_ERROR_TOLERANCE
                },
                if 0. != ray.direction.y {
                    ray.direction.y
                } else {
                    FLOAT_ERROR_TOLERANCE
                },
                if 0. != ray.direction.z {
                    ray.direction.z
                } else {
                    FLOAT_ERROR_TOLERANCE
                },
            ),
        };

        // Pre-calculated optimization variables
        let ray_scale_factors = Self::get_dda_scale_factors(&ray);
        let direction_lut_index = hash_direction(&ray.direction) as usize;

        let root_bounds = Cube::root_bounds(self.octree_size as f32);
        let mut ray_current_distance = 0.0; // No need to initialize, but it will shut the compiler
        let mut node_stack = Vec::new();
        if let Some(root_hit) = root_bounds.intersect_ray(&ray) {
            ray_current_distance = root_hit.impact_distance.unwrap_or(0.);
            let target_octant = hash_region(
                &(ray.point_at(ray_current_distance) - root_bounds.min_position.into()),
                root_bounds.size as f32,
            );
            node_stack.push(NodeStackItem::new(
                root_bounds,
                Octree::<T, DIM>::ROOT_NODE_KEY,
                target_octant,
            ));
        }

        while !node_stack.is_empty() {
            let node_stack_top = node_stack.last().unwrap();
            let mut current_bounds = node_stack_top.bounds;
            let current_node_key = node_stack_top.node as usize;
            let current_node = self.nodes.get(current_node_key);
            let mut target_octant = node_stack_top.target_octant;
            let current_node_occupied_bits = self.occupied_8bit(node_stack_top.node);

            debug_assert!(self
                .nodes
                .key_is_valid(node_stack.last().unwrap().node as usize));

            let mut leaf_miss = false;
            if current_node.is_leaf() {
                debug_assert!(matches!(
                    self.node_children[current_node_key].content,
                    NodeChildrenArray::OccupancyBitmap(_)
                ));
                if let Some(leaf_brick_hit) = Self::traverse_brick(
                    &ray,
                    &mut ray_current_distance,
                    current_node.leaf_data(),
                    match self.node_children[current_node_key].content {
                        NodeChildrenArray::OccupancyBitmap(bitmap) => bitmap,
                        _ => {
                            debug_assert!(false);
                            0
                        }
                    },
                    &current_bounds,
                    &ray_scale_factors,
                    direction_lut_index,
                ) {
                    current_bounds.size /= DIM as f32;
                    current_bounds.min_position = current_bounds.min_position
                        + V3c::<f32>::from(leaf_brick_hit) * current_bounds.size;
                    let impact_point = ray.point_at(ray_current_distance);
                    let impact_normal = cube_impact_normal(&current_bounds, &impact_point);
                    return Some((
                        &current_node.leaf_data()[leaf_brick_hit.x][leaf_brick_hit.y]
                            [leaf_brick_hit.z],
                        impact_point,
                        impact_normal,
                    ));
                }
                leaf_miss = true;
            }

            if leaf_miss
                || node_stack_top.target_octant == OOB_OCTANT
                // In case the current Node is empty
                || 0 == current_node_occupied_bits
                // In case there is no overlap between the node occupancy and the potential slots the ray would hit
                || 0 == (current_node_occupied_bits & RAY_TO_NODE_OCCUPANCY_BITMASK_LUT[target_octant as usize][direction_lut_index as usize])
            {
                // POP
                let popped_target = node_stack.pop().unwrap();
                if let Some(parent) = node_stack.last_mut() {
                    let step_vec = Self::dda_step_to_next_sibling(
                        &ray,
                        &mut ray_current_distance,
                        &popped_target.bounds,
                        &ray_scale_factors,
                    );
                    parent.target_octant = step_octant(parent.target_octant, step_vec);
                }
                continue; // Re-calculate current_bounds
            }

            let mut target_bounds = current_bounds.child_bounds_for(target_octant);
            let mut target_child_key = self.node_children[current_node_key][target_octant as u32];
            let target_is_empty = !self.nodes.key_is_valid(target_child_key as usize)
                || 0 == current_node_occupied_bits & octant_bitmask(target_octant);
            if !target_is_empty {
                // PUSH
                let child_target_octant = hash_region(
                    &(ray.point_at(ray_current_distance) - target_bounds.min_position.into()),
                    target_bounds.size as f32,
                );
                node_stack.push(NodeStackItem::new(
                    target_bounds,
                    target_child_key,
                    child_target_octant,
                ));
            } else {
                // ADVANCE
                // target child is invalid, or it does not intersect with the ray
                // Advance iteration to the next sibling
                loop {
                    // step the iteration to the next sibling cell!
                    let step_vec = Self::dda_step_to_next_sibling(
                        &ray,
                        &mut ray_current_distance,
                        &target_bounds,
                        &ray_scale_factors,
                    );
                    target_octant = step_octant(target_octant, step_vec);
                    if OOB_OCTANT != target_octant {
                        target_bounds = current_bounds.child_bounds_for(target_octant);
                        target_child_key =
                            self.node_children[current_node_key][target_octant as u32];
                    }

                    if target_octant == OOB_OCTANT
                        || (self.nodes.key_is_valid(target_child_key as usize)
                            // current node is occupied at target octant
                            && 0 != current_node_occupied_bits & octant_bitmask(target_octant)
                            // target child collides with the ray
                            && 0 != match self.nodes.get(target_child_key as usize) {
                                NodeContent::Nothing => 0,
                                NodeContent::Internal(_) | NodeContent::Leaf(_)=> self.occupied_8bit(target_child_key)
                                    & RAY_TO_NODE_OCCUPANCY_BITMASK_LUT[hash_region(
                                        &(ray.point_at(ray_current_distance) - target_bounds.min_position),
                                        target_bounds.size,
                                    ) as usize]
                                    [direction_lut_index as usize],
                            })
                    {
                        // stop advancing because current target is either
                        // - OOB
                        // - or (not empty while inside bounds AND collides with the ray based on its occupancy bitmap)
                        node_stack.last_mut().unwrap().target_octant = target_octant;
                        break;
                    }
                }
            }
        }
        None
    }
}
