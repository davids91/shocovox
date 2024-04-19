use crate::octree::{raytracing::types::NodeStackItem, NodeContent};
use crate::octree::{Cube, Octree, V3c, VoxelData};

use crate::spatial::{
    math::{hash_region, offset_region, plane_line_intersection},
    raytracing::{CubeHit, Ray},
};

impl NodeStackItem {
    pub(crate) fn new(
        bounds: Cube,
        bounds_intersection: CubeHit,
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

impl<T> Octree<T>
where
    T: Default + PartialEq + Clone + std::fmt::Debug + VoxelData,
{
    fn get_step_to_next_sibling(current: &Cube, ray: &Ray) -> V3c<f32> {
        use crate::spatial::FLOAT_ERROR_TOLERANCE;
        //Find the point furthest from the ray
        let midpoint = current.midpoint();
        let ref_point = midpoint
            + V3c::new(
                (current.size as f32 / 2.).copysign(ray.direction.x),
                (current.size as f32 / 2.).copysign(ray.direction.y),
                (current.size as f32 / 2.).copysign(ray.direction.z),
            );

        // Find the min of the 3 plane intersections
        let x_plane_distance = plane_line_intersection(
            &ref_point,
            &V3c::new(1., 0., 0.),
            &ray.origin,
            &ray.direction,
        )
        .unwrap();
        let y_plane_distance = plane_line_intersection(
            &ref_point,
            &V3c::new(0., 1., 0.),
            &ray.origin,
            &ray.direction,
        )
        .unwrap();
        let z_plane_distance = plane_line_intersection(
            &ref_point,
            &V3c::new(0., 0., 1.),
            &ray.origin,
            &ray.direction,
        )
        .unwrap();
        let min_d = x_plane_distance.min(y_plane_distance).min(z_plane_distance);

        // Step along the axes with the minimum distances
        V3c::new(
            if (min_d - x_plane_distance).abs() < FLOAT_ERROR_TOLERANCE {
                (current.size as f32).copysign(ray.direction.x)
            } else {
                0.
            },
            if (min_d - y_plane_distance).abs() < FLOAT_ERROR_TOLERANCE {
                (current.size as f32).copysign(ray.direction.y)
            } else {
                0.
            },
            if (min_d - z_plane_distance).abs() < FLOAT_ERROR_TOLERANCE {
                (current.size as f32).copysign(ray.direction.z)
            } else {
                0.
            },
        )
    }

    /// provides the collision point of the ray with the contained voxel field
    /// return reference of the data, collision point and normal at impact, should there be any
    pub fn get_by_ray(&self, ray: &Ray) -> Option<(&T, V3c<f32>, V3c<f32>)> {
        use crate::object_pool::key_might_be_valid;
        let root_bounds = Cube::root_bounds(self.root_size);
        let mut current_d = 0.0; // No need to initialize, but it will shut the compiler
        let mut node_stack = Vec::new();

        if let Some(root_hit) = root_bounds.intersect_ray(ray) {
            current_d = root_hit.impact_distance.unwrap_or(0.);
            if self
                .nodes
                .get(Octree::<T>::ROOT_NODE_KEY as usize)
                .is_leaf()
            {
                return Some((
                    self.nodes
                        .get(Octree::<T>::ROOT_NODE_KEY as usize)
                        .leaf_data(),
                    ray.point_at(current_d),
                    root_hit.impact_normal,
                ));
            }
            let target_octant = hash_region(
                &(ray.point_at(current_d) - root_bounds.min_position.into()),
                self.root_size as f32,
            );
            node_stack.push(NodeStackItem::new(
                root_bounds,
                root_hit,
                Octree::<T>::ROOT_NODE_KEY,
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
                    let step_vec = Self::get_step_to_next_sibling(&popped_target.bounds, ray);
                    parent.add_point(step_vec);
                }
                current_d = current_bounds_ray_intersection.exit_distance;
                continue;
            }

            let current_node = node_stack.last().unwrap().node as usize;
            assert!(key_might_be_valid(current_node as u32));

            if self.nodes.get(current_node).is_leaf() {
                return Some((
                    self.nodes.get(current_node).leaf_data(),
                    ray.point_at(
                        current_bounds_ray_intersection
                            .impact_distance
                            .unwrap_or(0.),
                    ),
                    current_bounds_ray_intersection.impact_normal,
                ));
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
            let target_hit = if target_is_empty {
                None
            } else {
                target_bounds.intersect_ray(ray)
            };
            if !target_is_empty && target_hit.is_some() {
                // PUSH
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
                let step_vec = Self::get_step_to_next_sibling(&current_target_bounds, ray);
                node_stack.last_mut().unwrap().add_point(step_vec);
            }
        }
        None
    }
}
