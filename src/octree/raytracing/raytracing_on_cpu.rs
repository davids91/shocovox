use crate::octree::{raytracing::types::NodeStackItem, NodeContent};
use crate::octree::{Cube, Octree, V3c, VoxelData};

use crate::spatial::{
    math::{hash_region, offset_region, plane_line_intersection},
    raytracing::{CubeRayIntersection, Ray},
    FLOAT_ERROR_TOLERANCE,
};

impl NodeStackItem {
    pub(crate) fn new(
        bounds: Cube,
        bounds_intersection: CubeRayIntersection,
        node: u32,
        target_octant: u32,
    ) -> Self {
        let child_center = Into::<V3c<f32>>::into(bounds.min_position)
            + V3c::unit(bounds.size as f32 / 4.)
            + Into::<V3c<f32>>::into(offset_region(target_octant)) * (bounds.size as f32 / 2.);
        Self {
            bounds_intersection,
            bounds,
            node,
            target_octant,
            child_center,
        }
    }

    pub(crate) fn add_point(&mut self, p: V3c<f32>) {
        self.child_center = self.child_center + p;
        self.target_octant = hash_region(
            &(self.child_center - self.bounds.min_position.into()),
            self.bounds.size as f32,
        );
    }

    pub(crate) fn target_bounds(&self) -> Cube {
        self.bounds.child_bounds_for(self.target_octant)
    }

    pub(crate) fn contains_target_center(&self) -> bool {
        self.bounds.contains_point(&self.child_center)
    }
}

impl<T: Default + PartialEq + Clone + std::fmt::Debug + VoxelData, const DIM: usize>
    Octree<T, DIM>
{
    pub(in crate::octree) fn get_dda_scale_factors(ray: &Ray) -> V3c<f32> {
        let angle_corrected_direction = V3c::new(
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
        );
        V3c::new(
            (1. + (ray.direction.z / angle_corrected_direction.x).powf(2.)
                + (ray.direction.y / angle_corrected_direction.x).powf(2.))
            .sqrt(),
            ((ray.direction.x / angle_corrected_direction.y).powf(2.)
                + 1.
                + (ray.direction.z / angle_corrected_direction.y).powf(2.))
            .sqrt(),
            (((ray.direction.x / angle_corrected_direction.z).powf(2.) + 1.)
                + (ray.direction.y / angle_corrected_direction.z).powf(2.))
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

    /// Iterates on the given ray and matrix to find a potential intersection in 3D space
    fn traverse_matrix(
        ray: &Ray,
        ray_current_distance: &mut f32,
        ray_scale_factors: &V3c<f32>,
        matrix: &[[[T; DIM]; DIM]; DIM],
        bounds: &Cube,
        intersection: &CubeRayIntersection,
    ) -> Option<V3c<usize>> {
        let mut current_index = {
            let pos = ray.point_at(
                intersection
                    .impact_distance
                    .unwrap_or(*ray_current_distance),
            ) - V3c::<f32>::from(bounds.min_position);
            V3c::new(
                (pos.x as i32).clamp(0, (DIM - 1) as i32),
                (pos.y as i32).clamp(0, (DIM - 1) as i32),
                (pos.z as i32).clamp(0, (DIM - 1) as i32),
            )
        };
        let matrix_unit = bounds.size / DIM as u32;
        let mut current_bounds = Cube {
            min_position: bounds.min_position + V3c::<u32>::from(current_index) * matrix_unit,
            size: matrix_unit,
        };
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

            if !matrix[current_index.x as usize][current_index.y as usize][current_index.z as usize]
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
            current_bounds.min_position =
                V3c::<u32>::from(V3c::<f32>::from(current_bounds.min_position) + step);
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
        use crate::object_pool::key_might_be_valid;
        let root_bounds = Cube::root_bounds(self.octree_size);
        let mut current_d = 0.0; // No need to initialize, but it will shut the compiler
        let mut node_stack = Vec::new();
        let ray_scale_factors = Self::get_dda_scale_factors(&ray);
        if let Some(root_hit) = root_bounds.intersect_ray(ray) {
            current_d = root_hit.impact_distance.unwrap_or(0.);
            if self
                .nodes
                .get(Octree::<T, DIM>::ROOT_NODE_KEY as usize)
                .is_leaf()
            {
                if let Some(root_matrix_hit) = Self::traverse_matrix(
                    ray,
                    &mut current_d,
                    &ray_scale_factors,
                    self.nodes
                        .get(Octree::<T, DIM>::ROOT_NODE_KEY as usize)
                        .leaf_data(),
                    &root_bounds,
                    &root_hit,
                ) {
                    let matrix_unit = root_bounds.size / DIM as u32;
                    let result_raycast = Cube {
                        min_position: root_bounds.min_position
                            + V3c::<u32>::from(root_matrix_hit * matrix_unit as usize),
                        size: matrix_unit,
                    }
                    .intersect_ray(ray)
                    .unwrap_or(root_hit);
                    return Some((
                        &self
                            .nodes
                            .get(Octree::<T, DIM>::ROOT_NODE_KEY as usize)
                            .leaf_data()[root_matrix_hit.x][root_matrix_hit.y][root_matrix_hit.z],
                        ray.point_at(result_raycast.impact_distance.unwrap_or(current_d)),
                        result_raycast.impact_normal,
                    ));
                } else {
                    // If the root if a leaf already and there's no hit in it, then there is no hit at all.
                    return None;
                }
            }
            let target_octant = hash_region(
                &(ray.point_at(current_d) - root_bounds.min_position.into()),
                root_bounds.size as f32,
            );
            node_stack.push(NodeStackItem::new(
                root_bounds,
                root_hit,
                Octree::<T, DIM>::ROOT_NODE_KEY,
                target_octant,
            ));
        }

        while !node_stack.is_empty() {
            let current_bounds = node_stack.last().unwrap().bounds;
            let current_bounds_ray_intersection = node_stack.last().unwrap().bounds_intersection;
            if !node_stack.last().unwrap().contains_target_center() // If current target is OOB
                // No need to go into the Node if it's empty
                || match self.nodes.get(node_stack.last().unwrap().node as usize) {
                    NodeContent::Nothing => true,
                    NodeContent::Internal(count) => 0 == *count,
                    _ => false,
                }
            {
                // POP
                let popped_target = node_stack.pop().unwrap();
                if let Some(parent) = node_stack.last_mut() {
                    let step_vec = Self::dda_step_to_next_sibling(
                        ray,
                        &mut current_d,
                        &popped_target.bounds,
                        &ray_scale_factors,
                    );
                    parent.add_point(step_vec);
                }
                current_d = current_bounds_ray_intersection.exit_distance;
                continue; // Re-calculate current_bounds and ray intersection
            }

            let current_node = node_stack.last().unwrap().node as usize;
            assert!(key_might_be_valid(current_node as u32));

            if self.nodes.get(current_node).is_leaf() {
                if let Some(leaf_matrix_hit) = Self::traverse_matrix(
                    ray,
                    &mut current_d,
                    &ray_scale_factors,
                    self.nodes.get(current_node).leaf_data(),
                    &current_bounds,
                    &current_bounds_ray_intersection,
                ) {
                    let matrix_unit = current_bounds.size / DIM as u32;
                    let result_raycast = Cube {
                        min_position: current_bounds.min_position
                            + V3c::<u32>::from(leaf_matrix_hit * matrix_unit as usize),
                        size: matrix_unit,
                    }
                    .intersect_ray(ray)
                    .unwrap_or(current_bounds_ray_intersection);
                    return Some((
                        &self.nodes.get(current_node).leaf_data()[leaf_matrix_hit.x]
                            [leaf_matrix_hit.y][leaf_matrix_hit.z],
                        ray.point_at(result_raycast.impact_distance.unwrap_or(current_d)),
                        result_raycast.impact_normal,
                    ));
                } else {
                    // POP
                    let popped_target = node_stack.pop().unwrap();
                    if let Some(parent) = node_stack.last_mut() {
                        let step_vec = Self::dda_step_to_next_sibling(
                            ray,
                            &mut current_d,
                            &popped_target.bounds,
                            &ray_scale_factors,
                        );
                        parent.add_point(step_vec);
                    }
                    current_d = current_bounds_ray_intersection.exit_distance;
                    continue; // Re-calculate current_bounds and ray intersection
                }
            }
            current_d = current_bounds_ray_intersection
                .impact_distance
                .unwrap_or(current_d);

            let target_octant = node_stack.last().unwrap().target_octant;
            let target_child = self.node_children[current_node][target_octant];
            let target_bounds = current_bounds.child_bounds_for(target_octant);
            let target_is_empty = !key_might_be_valid(target_child)
                || match self.nodes.get(target_child as usize) {
                    NodeContent::Internal(count) => 0 == *count,
                    NodeContent::Leaf(_) => false,
                    _ => true,
                };
            let target_hit = target_bounds.intersect_ray(ray);
            if !target_is_empty && target_hit.is_some() {
                // PUSH
                current_d = target_hit.unwrap().impact_distance.unwrap_or(current_d);
                let child_target_octant = hash_region(
                    &(ray.point_at(current_d) - target_bounds.min_position.into()),
                    target_bounds.size as f32,
                );
                node_stack.push(NodeStackItem::new(
                    target_bounds,
                    target_hit.unwrap(),
                    target_child,
                    child_target_octant,
                ));
            } else {
                // ADVANCE
                // target child is invalid, or it does not intersect with the ray
                // Advance iteration to the next sibling
                let current_target_bounds = node_stack.last().unwrap().target_bounds();
                let step_vec = Self::dda_step_to_next_sibling(
                    ray,
                    &mut current_d,
                    &current_target_bounds,
                    &ray_scale_factors,
                );
                node_stack.last_mut().unwrap().add_point(step_vec);
                if let Some(hit) = target_hit {
                    current_d = hit.exit_distance;
                }
            }
        }
        None
    }
}
