#[cfg(feature = "raytracing")]
use crate::octree::{Cube, Octree, V3c};

use crate::spatial::raytracing::CubeHit;
#[cfg(feature = "raytracing")]
use crate::spatial::{
    math::{hash_region, offset_region},
    raytracing::Ray,
};

#[cfg(feature = "raytracing")]
pub(crate) struct NodeStackItem {
    pub(crate) bounds: Cube,
    pub(crate) node: usize,
    pub(crate) target_octant: usize,
    pub(crate) child_center: V3c<f32>,
    pub(crate) ray_intersection: CubeHit,
}

#[cfg(feature = "raytracing")]
impl NodeStackItem {
    pub(crate) fn new(
        bounds: Cube,
        ray_intersection: CubeHit,
        node: usize,
        target_octant: usize,
    ) -> Self {
        let quarter_size = bounds.size as f32 / 4.;
        let half_size = bounds.size as f32 / 2.;
        let child_center = Into::<V3c<f32>>::into(bounds.min_position)
            + V3c::unit(quarter_size)
            + Into::<V3c<f32>>::into(offset_region(target_octant)) * half_size;
        Self {
            bounds,
            node,
            target_octant,
            child_center,
            ray_intersection,
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

#[cfg(feature = "raytracing")]
use bendy::{decoding::FromBencode, encoding::ToBencode};

impl<
        #[cfg(feature = "serialization")] T: Default + ToBencode + FromBencode + Serialize + DeserializeOwned,
        #[cfg(not(feature = "serialization"))] T: Default + ToBencode + FromBencode,
    > Octree<T>
where
    T: Default + PartialEq + Clone + std::fmt::Debug,
{
    fn get_step_to_next_sibling(
        exit_point: V3c<f32>,
        midpoint: V3c<f32>,
        bounds_size: f32,
        ray: &Ray,
    ) -> V3c<f32> {
        use crate::spatial::raytracing::FLOAT_ERROR_TOLERANCE;

        let x_d = exit_point.x - midpoint.x;
        let y_d = exit_point.y - midpoint.y;
        let z_d = exit_point.z - midpoint.z;
        let max_d = x_d.abs().max(y_d.abs()).max(z_d.abs());
        let step_vec = V3c::new(
            if (x_d.abs() - max_d).abs() < FLOAT_ERROR_TOLERANCE
                && 1f32.copysign(x_d) == 1f32.copysign(ray.direction.x)
            {
                (bounds_size).copysign(ray.direction.x)
            } else {
                0.
            },
            if (y_d.abs() - max_d).abs() < FLOAT_ERROR_TOLERANCE
                && 1f32.copysign(y_d) == 1f32.copysign(ray.direction.y)
            {
                (bounds_size).copysign(ray.direction.y)
            } else {
                0.
            },
            if (z_d.abs() - max_d).abs() < FLOAT_ERROR_TOLERANCE
                && 1f32.copysign(z_d) == 1f32.copysign(ray.direction.z)
            {
                (bounds_size).copysign(ray.direction.z)
            } else {
                0.
            },
        );
        // assert!(0. < step_vec.length());
        if 0. == step_vec.length() {
            ray.direction * bounds_size
        } else {
            step_vec
        }
    }

    //TODO: Get by ray at LOD
    #[cfg(feature = "raytracing")]
    /// provides the collision point of the ray with the contained voxel field
    /// return reference of the data, collision point and normal at impact, should there be any
    pub fn get_by_ray(&self, ray: &Ray) -> Option<(&T, V3c<f32>, V3c<f32>)> {
        use crate::object_pool::key_might_be_valid;
        let root_bounds = Cube::root_bounds(self.root_size);
        let mut current_d;
        let mut node_stack = Vec::new();

        if let Some(hit) = root_bounds.intersect_ray(&ray) {
            if self.nodes.get(self.root_node).is_leaf() {
                return Some((
                    self.nodes.get(self.root_node).leaf_data(),
                    ray.point_at(if let Some(entry_distance) = hit.impact_distance {
                        entry_distance
                    } else {
                        0.
                    }),
                    hit.impact_normal,
                ));
            }
            current_d = hit.impact_distance.unwrap_or(0.);
            let target_octant = hash_region(
                &(ray.point_at(current_d) - root_bounds.min_position.into()),
                self.root_size as f32,
            );
            node_stack.push(NodeStackItem::new(
                root_bounds,
                hit,
                self.root_node,
                target_octant,
            ));
        } else {
            return None;
        }

        while !node_stack.is_empty() {
            let current_bounds = node_stack.last().unwrap().bounds;
            let current_bounds_ray_intersection = node_stack.last().unwrap().ray_intersection;
            let current_target_bounds = node_stack.last().unwrap().target_bounds();
            let current_node = node_stack.last().unwrap().node;
            let current_target_octant = node_stack.last().unwrap().target_octant;
            assert!(key_might_be_valid(current_node));

            if !node_stack.last().unwrap().contains_target_center() {
                current_d = current_bounds_ray_intersection.exit_distance;
                let popped_target = node_stack.pop().unwrap();
                if let Some(parent) = node_stack.last_mut() {
                    let step_vec = Self::get_step_to_next_sibling(
                        ray.point_at(popped_target.ray_intersection.exit_distance),
                        popped_target.bounds.midpoint(),
                        popped_target.bounds.size as f32,
                        &ray,
                    );
                    parent.add_point(step_vec);
                }
                continue;
            }

            if self.nodes.get(current_node).is_leaf() {
                return Some((
                    self.nodes.get(current_node).leaf_data(),
                    ray.point_at(
                        if let Some(entry_distance) =
                            current_bounds_ray_intersection.impact_distance
                        {
                            entry_distance
                        } else {
                            0.
                        },
                    ),
                    current_bounds_ray_intersection.impact_normal,
                ));
            }

            let target_child_ray_intersection = current_target_bounds.intersect_ray(&ray);
            if key_might_be_valid(self.node_children[current_node][current_target_octant])
                && target_child_ray_intersection.is_some()
            {
                let target_child_ray_intersection = target_child_ray_intersection.unwrap();
                current_d =
                    if let Some(impact_distance) = target_child_ray_intersection.impact_distance {
                        impact_distance
                    } else {
                        0.
                    };
                let child_bounds = current_bounds.child_bounds_for(current_target_octant);
                let child_target_octant = hash_region(
                    &(ray.point_at(current_d) - child_bounds.min_position.into()),
                    child_bounds.size as f32,
                );
                node_stack.push(NodeStackItem::new(
                    child_bounds,
                    target_child_ray_intersection,
                    self.node_children[current_node][current_target_octant],
                    child_target_octant,
                ));
            } else {
                // target child is invalid, advance iterator on the same level to the direction of the ray
                let step_vec = Self::get_step_to_next_sibling(
                    ray.point_at(
                        if let Some(target_child_ray_intersection) = target_child_ray_intersection {
                            target_child_ray_intersection.exit_distance
                        } else {
                            current_d
                        },
                    ),
                    current_target_bounds.midpoint(),
                    current_target_bounds.size as f32,
                    &ray,
                );
                node_stack.last_mut().unwrap().add_point(step_vec);
            }
        }
        None
    }
}

///####################################################################################
/// Tests
///####################################################################################
#[cfg(test)]
mod octree_raytracing_tests {

    #[cfg(feature = "raytracing")]
    use crate::octree::{raytracing::Ray, Octree, V3c};

    #[cfg(feature = "raytracing")]
    use rand::{rngs::ThreadRng, Rng};

    #[cfg(feature = "raytracing")]
    fn make_ray_point_to(target: &V3c<f32>, rng: &mut ThreadRng) -> Ray {
        let origin = V3c {
            x: rng.gen_range(8..16) as f32,
            y: rng.gen_range(8..16) as f32,
            z: rng.gen_range(8..16) as f32,
        };
        Ray {
            direction: (*target - origin).normalized(),
            origin,
        }
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_get_by_ray_from_outside() {
        let mut rng = rand::thread_rng();
        let mut tree = Octree::<u32>::new(4).ok().unwrap();
        let mut filled = Vec::new();
        for x in 1..4 {
            for y in 1..4 {
                if 10 > rng.gen_range(0..20) {
                    let pos = V3c::new(x, y, 1);
                    tree.insert(&pos, 5).ok();
                    filled.push(pos);
                }
            }
        }

        for p in filled.into_iter() {
            let ray = make_ray_point_to(&V3c::new(p.x as f32, p.y as f32, p.z as f32), &mut rng);
            assert!(tree.get_by_ray(&ray).is_some());
            assert!(*tree.get_by_ray(&ray).unwrap().0 == 5);
        }
    }

    #[cfg(feature = "raytracing")]
    fn make_edge_ray_point_to(target: &V3c<f32>, rng: &mut ThreadRng) -> Ray {
        let origin = V3c {
            x: rng.gen_range(0..8) as f32,
            y: rng.gen_range(0..8) as f32,
            z: 8.,
        };
        Ray {
            direction: (*target - origin).normalized(),
            origin,
        }
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_get_by_ray_from_edge() {
        let mut rng = rand::thread_rng();
        let mut tree = Octree::<u32>::new(8).ok().unwrap();
        let mut filled = Vec::new();
        for x in 1..4 {
            for y in 1..4 {
                for z in 1..4 {
                    if 10 > rng.gen_range(0..20) {
                        let pos = V3c::new(x, y, z);
                        tree.insert(&pos, 5).ok();
                        filled.push(pos);
                    }
                }
            }
        }

        for p in filled.into_iter() {
            let ray = make_edge_ray_point_to(
                &V3c::new(p.x as f32 + 0.1, p.y as f32 + 0.1, p.z as f32 + 0.1),
                &mut rng,
            );
            assert!(tree.get_by_ray(&ray).is_some());
            assert!(*tree.get_by_ray(&ray).unwrap().0 == 5);
        }
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_get_by_ray_from_inside() {
        let mut rng = rand::thread_rng();
        let mut tree = Octree::<u32>::new(16).ok().unwrap();
        let mut filled = Vec::new();
        for x in 1..4 {
            for y in 1..4 {
                for z in 1..4 {
                    if 10 > rng.gen_range(0..20) {
                        let pos = V3c::new(x, y, z);
                        tree.insert(&pos, 5).ok();
                        filled.push(pos);
                    }
                }
            }
        }

        for p in filled.into_iter() {
            let pos = V3c::new(p.x as f32, p.y as f32, p.z as f32);
            let ray = make_ray_point_to(&pos, &mut rng);
            assert!(tree.get(&pos.into()).is_some());
            assert!(tree.get_by_ray(&ray).is_some());
            assert!(*tree.get_by_ray(&ray).unwrap().0 == 5);
        }
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_edge_case_unreachable() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();
        tree.insert(&V3c::new(3, 0, 0), 0).ok();
        tree.insert(&V3c::new(3, 3, 0), 1).ok();
        tree.insert(&V3c::new(0, 3, 0), 2).ok();

        for y in 0..4 {
            tree.insert(&V3c::new(0, y, y), 3).ok();
            tree.insert(&V3c::new(1, y, y), 3).ok();
            tree.insert(&V3c::new(2, y, y), 3).ok();
            tree.insert(&V3c::new(3, y, y), 3).ok();
        }

        let ray = Ray {
            origin: V3c {
                x: 10.0,
                y: 10.0,
                z: -5.,
            },
            direction: V3c {
                x: -0.66739213,
                y: -0.6657588,
                z: 0.333696,
            },
        };
        let _ = tree.get_by_ray(&ray); //Should not fail with unreachable code panic
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_edge_case_empty_line_in_middle() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();
        tree.insert(&V3c::new(3, 0, 0), 0).ok();
        tree.insert(&V3c::new(3, 3, 0), 1).ok();
        tree.insert(&V3c::new(0, 3, 0), 2).ok();

        for y in 0..4 {
            tree.insert(&V3c::new(0, y, y), 3).ok();
            tree.insert(&V3c::new(1, y, y), 3).ok();
            tree.insert(&V3c::new(2, y, y), 3).ok();
            tree.insert(&V3c::new(3, y, y), 3).ok();
        }

        let ray = Ray {
            origin: V3c {
                x: 8.965594,
                y: 10.0,
                z: -4.4292345,
            },
            direction: V3c {
                x: -0.5082971,
                y: -0.72216684,
                z: 0.46915793,
            },
        };
        assert!(tree.get_by_ray(&ray).is_some());
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_edge_case_zero_advance() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();
        tree.insert(&V3c::new(3, 0, 0), 0).ok();
        tree.insert(&V3c::new(3, 3, 0), 1).ok();
        tree.insert(&V3c::new(0, 3, 0), 2).ok();

        for y in 0..4 {
            tree.insert(&V3c::new(0, y, y), 3).ok();
            tree.insert(&V3c::new(1, y, y), 3).ok();
            tree.insert(&V3c::new(2, y, y), 3).ok();
            tree.insert(&V3c::new(3, y, y), 3).ok();
        }

        let ray = Ray {
            origin: V3c {
                x: 8.930992,
                y: 10.0,
                z: -4.498597,
            },
            direction: V3c {
                x: -0.4687217,
                y: -0.772969,
                z: 0.42757326,
            },
        };
        assert!(tree.get_by_ray(&ray).is_some());
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_edge_case_cube_edges() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();
        tree.insert(&V3c::new(3, 0, 0), 0).ok();
        tree.insert(&V3c::new(3, 3, 0), 1).ok();
        tree.insert(&V3c::new(0, 3, 0), 2).ok();

        for y in 0..4 {
            tree.insert(&V3c::new(0, y, y), 3).ok();
            tree.insert(&V3c::new(1, y, y), 4).ok();
            tree.insert(&V3c::new(2, y, y), 5).ok();
            tree.insert(&V3c::new(3, y, y), 6).ok();
        }

        let ray = Ray {
            origin: V3c {
                x: 10.0,
                y: 10.0,
                z: -5.,
            },
            direction: (V3c {
                x: 3.0,
                y: 1.9,
                z: 2.0,
            } - V3c {
                x: 10.0,
                y: 10.0,
                z: -5.,
            })
            .normalized(),
        };

        //Should reach position 3, 2, 2
        assert!(tree.get_by_ray(&ray).is_some_and(|v| *v.0 == 6));
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_edge_case_ray_behind_octree() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();
        tree.insert(&V3c::new(0, 3, 0), 5).ok();
        let origin = V3c::new(2., 2., -5.);
        let ray = Ray {
            direction: (V3c::new(0., 3., 0.) - origin).normalized(),
            origin,
        };
        assert!(tree.get(&V3c::new(0, 3, 0)).is_some());
        assert!(*tree.get(&V3c::new(0, 3, 0)).unwrap() == 5);
        assert!(tree.get_by_ray(&ray).is_some());
        assert!(*tree.get_by_ray(&ray).unwrap().0 == 5);
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_edge_case_overlapping_voxels() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 0), 5).ok();
        tree.insert(&V3c::new(1, 0, 0), 6).ok();

        let test_ray = Ray {
            origin: V3c {
                x: 2.0,
                y: 4.0,
                z: -2.0,
            },
            direction: V3c {
                x: -0.23184556,
                y: -0.79392403,
                z: 0.5620785,
            },
        };
        assert!(tree.get_by_ray(&test_ray).is_some_and(|hit| *hit.0 == 6));
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_edge_case_edge_raycast() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();

        for x in 0..4 {
            for z in 0..4 {
                tree.insert(&V3c::new(x, 0, z), 5).ok();
            }
        }
        let ray = Ray {
            origin: V3c {
                x: 2.0,
                y: 4.0,
                z: -2.0,
            },
            direction: V3c {
                x: -0.47839317,
                y: -0.71670955,
                z: 0.50741255,
            },
        };
        let result = tree.get_by_ray(&ray);
        assert!(result.is_none() || *result.unwrap().0 == 5);
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_edge_case_voxel_corner() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();

        for x in 0..4 {
            for z in 0..4 {
                tree.insert(&V3c::new(x, 0, z), 5).ok();
            }
        }

        let ray = Ray {
            origin: V3c {
                x: 2.0,
                y: 4.0,
                z: -2.0,
            },
            direction: V3c {
                x: -0.27100056,
                y: -0.7961219,
                z: 0.54106253,
            },
        };
        assert!(tree.get_by_ray(&ray).is_some());
        assert!(*tree.get_by_ray(&ray).unwrap().0 == 5);
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_edge_case_bottom_edge() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();

        for x in 0..4 {
            for z in 0..4 {
                tree.insert(&V3c::new(x, 0, z), 5).ok();
            }
        }

        let ray = Ray {
            origin: V3c {
                x: 2.0,
                y: 4.0,
                z: -2.0,
            },
            direction: V3c {
                x: -0.379010856,
                y: -0.822795153,
                z: 0.423507959,
            },
        };
        assert!(tree.get_by_ray(&ray).is_some());
        assert!(*tree.get_by_ray(&ray).unwrap().0 == 5);
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_edge_case_loop_stuck() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();
        tree.insert(&V3c::new(3, 0, 0), 0).ok();
        tree.insert(&V3c::new(3, 3, 0), 1).ok();
        tree.insert(&V3c::new(0, 3, 0), 2).ok();

        for y in 0..4 {
            tree.insert(&V3c::new(0, y, y), 3).ok();
            tree.insert(&V3c::new(1, y, y), 4).ok();
            tree.insert(&V3c::new(2, y, y), 5).ok();
            tree.insert(&V3c::new(3, y, y), 6).ok();
        }

        let ray = Ray {
            origin: V3c {
                x: 0.024999974,
                y: 10.0,
                z: 0.0,
            },
            direction: V3c {
                x: -0.0030831057,
                y: -0.98595166,
                z: 0.16700225,
            },
        };
        let _ = tree.get_by_ray(&ray); //should not cause infinite loop
    }
}
