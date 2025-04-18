use crate::{
    octree::V3c,
    spatial::{
        math::FLOAT_ERROR_TOLERANCE,
        raytracing::{plane_line_intersection, Ray},
        Cube,
    },
};

/// Reference implementation to decide step to sibling boundary
#[allow(dead_code)]
pub(crate) fn get_step_to_next_sibling(current: &Cube, ray: &Ray) -> V3c<f32> {
    //Find the point furthest from the ray
    let midpoint = V3c::unit(current.size / 2.0) + current.min_position;
    let ref_point = midpoint
        + V3c::new(
            (current.size / 2.).copysign(ray.direction.x),
            (current.size / 2.).copysign(ray.direction.y),
            (current.size / 2.).copysign(ray.direction.z),
        );

    // Find the min of the 3 plane intersections
    let x_plane_distance = plane_line_intersection(
        &ref_point,
        &V3c::new(1., 0., 0.),
        &ray.origin,
        &ray.direction,
    )
    .unwrap_or(f32::MAX);
    let y_plane_distance = plane_line_intersection(
        &ref_point,
        &V3c::new(0., 1., 0.),
        &ray.origin,
        &ray.direction,
    )
    .unwrap_or(f32::MAX);
    let z_plane_distance = plane_line_intersection(
        &ref_point,
        &V3c::new(0., 0., 1.),
        &ray.origin,
        &ray.direction,
    )
    .unwrap_or(f32::MAX);
    let min_d = x_plane_distance.min(y_plane_distance).min(z_plane_distance);

    // Step along the axes with the minimum distances
    V3c::new(
        if (min_d - x_plane_distance).abs() < FLOAT_ERROR_TOLERANCE {
            (current.size).copysign(ray.direction.x)
        } else {
            0.
        },
        if (min_d - y_plane_distance).abs() < FLOAT_ERROR_TOLERANCE {
            (current.size).copysign(ray.direction.y)
        } else {
            0.
        },
        if (min_d - z_plane_distance).abs() < FLOAT_ERROR_TOLERANCE {
            (current.size).copysign(ray.direction.z)
        } else {
            0.
        },
    )
}

#[cfg(test)]
mod wgpu_tests {
    #[test]
    fn test_special_key_values() {
        // assumptions in shader needs to be compared to factual values
        assert!(crate::object_pool::empty_marker::<u32>() == 4294967295u32);
    }
}

#[cfg(test)]
mod octree_raytracing_tests {
    use crate::{
        octree::{Albedo, BoxTree, BoxTreeEntry, V3c},
        raytracing::tests::get_step_to_next_sibling,
        spatial::{math::FLOAT_ERROR_TOLERANCE, raytracing::Ray, Cube},
        voxel_data,
    };

    use rand::{rngs::ThreadRng, Rng};

    #[test]
    #[ignore = "May fail in edge cases"]
    fn compare_sibling_step_functions() {
        // Sometimes this test fails because the optimized implementation
        // does not consider points at the exact edges of the cube to be part of it
        // and axis aligned ray directions at the cube boundaries also behave differently
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let cube = Cube {
                min_position: V3c::new(
                    rng.gen_range(0..100) as f32,
                    rng.gen_range(0..100) as f32,
                    rng.gen_range(0..100) as f32,
                ),
                size: rng.gen_range(1..1000) as f32,
            };
            let ray = make_ray_point_to(
                &(V3c::from(cube.min_position) + V3c::unit(cube.size as f32) * 0.5),
                &mut rng,
            );
            let scale_factors = BoxTree::<Albedo>::get_dda_scale_factors(&ray);
            let mut current_point = ray.point_at(
                cube.intersect_ray(&ray)
                    .unwrap()
                    .impact_distance
                    .unwrap_or(0.),
            );

            assert!(
                FLOAT_ERROR_TOLERANCE
                    > (get_step_to_next_sibling(&cube, &ray)
                        - BoxTree::<Albedo>::dda_step_to_next_sibling(
                            &ray,
                            &mut current_point,
                            &cube,
                            &scale_factors
                        ))
                    .length()
            );
        }
    }

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

    #[test]
    fn test_get_by_ray_from_outside() {
        let mut rng = rand::thread_rng();
        let mut tree: BoxTree = BoxTree::new(4, 1).ok().unwrap();
        let mut filled = Vec::new();
        for x in 1..4 {
            for y in 1..4 {
                if 10 > rng.gen_range(0..20) {
                    let pos = V3c::new(x, y, 1);
                    tree.insert(&pos, voxel_data!(&5)).ok().unwrap();
                    filled.push(pos);
                }
            }
        }

        for p in filled.into_iter() {
            let ray = make_ray_point_to(&V3c::new(p.x as f32, p.y as f32, p.z as f32), &mut rng);
            assert!(tree.get_by_ray(&ray).is_some());
            assert!(tree.get_by_ray(&ray).unwrap().0 == voxel_data!(&5));
        }
    }

    #[test]
    fn test_get_by_ray_from_outside_where_dim_is_2() {
        let mut rng = rand::thread_rng();
        let mut tree: BoxTree = BoxTree::new(8, 2).ok().unwrap();
        let mut filled = Vec::new();
        for x in 1..4 {
            for y in 1..4 {
                if 10 > rng.gen_range(0..20) {
                    let pos = V3c::new(x, y, 1);
                    tree.insert(&pos, voxel_data!(&5)).ok().unwrap();
                    filled.push(pos);
                }
            }
        }

        for p in filled.into_iter() {
            let ray = make_ray_point_to(&V3c::new(p.x as f32, p.y as f32, p.z as f32), &mut rng);
            assert!(tree.get_by_ray(&ray).is_some());
            assert!(tree.get_by_ray(&ray).unwrap().0 == voxel_data!(&5));
        }
    }

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

    #[test]
    fn test_get_by_ray_from_edge() {
        let mut rng = rand::thread_rng();
        let mut tree: BoxTree = BoxTree::new(16, 1).ok().unwrap();
        let mut filled = Vec::new();
        for x in 1..4 {
            for y in 1..4 {
                for z in 1..4 {
                    if 10 > rng.gen_range(0..20) {
                        let pos = V3c::new(x, y, z);
                        tree.insert(&pos, voxel_data!(&5)).ok().unwrap();
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
            assert!(tree.get_by_ray(&ray).unwrap().0 == voxel_data!(&5));
        }
    }

    #[test]
    fn test_get_by_ray_from_inside() {
        let mut rng = rand::thread_rng();
        let mut tree: BoxTree = BoxTree::new(16, 1).ok().unwrap();
        let mut filled = Vec::new();
        for x in 1..4 {
            for y in 1..4 {
                for z in 1..4 {
                    if 10 > rng.gen_range(0..20) {
                        let pos = V3c::new(x, y, z);
                        tree.insert(&pos, voxel_data!(&5)).ok().unwrap();
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
            assert!(tree.get_by_ray(&ray).unwrap().0 == voxel_data!(&5));
        }
    }

    #[test]
    fn test_edge_case_unreachable() {
        let mut tree: BoxTree = BoxTree::new(4, 1).ok().unwrap();
        tree.insert(&V3c::new(3, 0, 0), &Albedo::from(0).into())
            .ok()
            .unwrap();
        tree.insert(&V3c::new(3, 3, 0), &Albedo::from(1).into())
            .ok()
            .unwrap();
        tree.insert(&V3c::new(0, 3, 0), &Albedo::from(2).into())
            .ok()
            .unwrap();

        for y in 0..4 {
            tree.insert(&V3c::new(0, y, y), &Albedo::from(3))
                .ok()
                .unwrap();
            tree.insert(&V3c::new(1, y, y), &Albedo::from(3))
                .ok()
                .unwrap();
            tree.insert(&V3c::new(2, y, y), &Albedo::from(3))
                .ok()
                .unwrap();
            tree.insert(&V3c::new(3, y, y), &Albedo::from(3))
                .ok()
                .unwrap();
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

    #[test]
    fn test_edge_case_empty_line_in_middle() {
        let mut tree: BoxTree = BoxTree::new(4, 1).ok().unwrap();
        tree.insert(&V3c::new(2, 1, 1), &Albedo::from(3).into())
            .ok();

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

    #[test]
    fn test_edge_case_zero_advance() {
        let mut tree: BoxTree = BoxTree::new(4, 1).ok().unwrap();
        tree.insert(&V3c::new(3, 0, 0), &Albedo::from(0).into())
            .ok()
            .unwrap();
        tree.insert(&V3c::new(3, 3, 0), &Albedo::from(1).into())
            .ok()
            .unwrap();
        tree.insert(&V3c::new(0, 3, 0), &Albedo::from(2).into())
            .ok()
            .unwrap();

        for y in 0..4 {
            tree.insert(&V3c::new(0, y, y), &Albedo::from(3))
                .ok()
                .unwrap();
            tree.insert(&V3c::new(1, y, y), &Albedo::from(3))
                .ok()
                .unwrap();
            tree.insert(&V3c::new(2, y, y), &Albedo::from(3))
                .ok()
                .unwrap();
            tree.insert(&V3c::new(3, y, y), &Albedo::from(3))
                .ok()
                .unwrap();
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

    #[test]
    fn test_edge_case_ray_behind_octree() {
        let mut tree: BoxTree = BoxTree::new(4, 1).ok().unwrap();
        tree.insert(&V3c::new(0, 3, 0), voxel_data!(&5))
            .ok()
            .unwrap();
        let origin = V3c::new(2., 2., -5.);
        let ray = Ray {
            direction: (V3c::new(0., 3., 0.) - origin).normalized(),
            origin,
        };
        assert!(tree.get(&V3c::new(0, 3, 0)).is_some());
        assert!(tree.get(&V3c::new(0, 3, 0)) == voxel_data!(&5));
        assert!(tree.get_by_ray(&ray).is_some());
        assert!(tree.get_by_ray(&ray).unwrap().0 == voxel_data!(&5));
    }

    #[test]
    fn test_edge_case_overlapping_voxels() {
        let mut tree: BoxTree = BoxTree::new(4, 1).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 0), voxel_data!(&5))
            .ok()
            .unwrap();
        tree.insert(&V3c::new(1, 0, 0), &Albedo::from(6).into())
            .ok()
            .unwrap();

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
        assert!(tree
            .get_by_ray(&test_ray)
            .is_some_and(|hit| hit.0 == (&Albedo::from(6)).into()));
    }

    #[test]
    fn test_edge_case_edge_raycast() {
        let mut tree: BoxTree = BoxTree::new(4, 1).ok().unwrap();

        for x in 0..4 {
            for z in 0..4 {
                tree.insert(&V3c::new(x, 0, z), voxel_data!(&5))
                    .ok()
                    .unwrap();
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
        assert!(result.is_none() || result.unwrap().0 == voxel_data!(&5));
    }

    #[test]
    fn test_edge_case_voxel_corner() {
        let mut tree: BoxTree = BoxTree::new(4, 1).ok().unwrap();

        for x in 0..4 {
            for z in 0..4 {
                tree.insert(&V3c::new(x, 0, z), voxel_data!(&5))
                    .ok()
                    .unwrap();
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
        assert!(tree.get_by_ray(&ray).unwrap().0 == voxel_data!(&5));
    }

    #[test]
    fn test_edge_case_bottom_edge() {
        let mut tree: BoxTree = BoxTree::new(4, 1).ok().unwrap();

        for x in 0..4 {
            for z in 0..4 {
                tree.insert(&V3c::new(x, 0, z), voxel_data!(&5))
                    .ok()
                    .unwrap();
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
        assert!(tree.get_by_ray(&ray).unwrap().0 == voxel_data!(&5));
    }

    #[test]
    fn test_edge_case_loop_stuck() {
        let mut tree: BoxTree = BoxTree::new(4, 1).ok().unwrap();
        tree.insert(&V3c::new(3, 0, 0), &Albedo::from(0).into())
            .ok()
            .unwrap();
        tree.insert(&V3c::new(3, 3, 0), &Albedo::from(1).into())
            .ok()
            .unwrap();
        tree.insert(&V3c::new(0, 3, 0), &Albedo::from(2).into())
            .ok()
            .unwrap();

        for y in 0..4 {
            tree.insert(&V3c::new(0, y, y), &Albedo::from(3))
                .ok()
                .unwrap();
            tree.insert(&V3c::new(1, y, y), &Albedo::from(4).into())
                .ok()
                .unwrap();
            tree.insert(&V3c::new(2, y, y), voxel_data!(&5))
                .ok()
                .unwrap();
            tree.insert(&V3c::new(3, y, y), &Albedo::from(6).into())
                .ok()
                .unwrap();
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

    #[test]
    fn test_edge_case_brick_undetected() {
        let mut tree: BoxTree = BoxTree::new(16, 4).ok().unwrap();

        for x in 0..4 {
            for z in 0..4 {
                tree.insert(&V3c::new(x, 0, z), voxel_data!(&5))
                    .ok()
                    .unwrap();
            }
        }

        for x in 0..4 {
            for z in 0..4 {
                assert!(tree.get(&V3c::new(x, 0, z)).is_some());
                assert!(tree.get(&V3c::new(x, 0, z)) == voxel_data!(&5));
            }
        }

        let ray = Ray {
            origin: V3c {
                x: -1.0716193,
                y: 8.0,
                z: -7.927902,
            },
            direction: V3c {
                x: 0.18699232,
                y: -0.6052176,
                z: 0.7737865,
            },
        };
        assert!(tree.get_by_ray(&ray).is_some());
        assert!(tree.get_by_ray(&ray).unwrap().0 == voxel_data!(&5));
    }

    #[test]
    fn test_edge_case_detailed_brick_undetected() {
        let tree_size = 8;
        const BRICK_DIMENSION: u32 = 2;
        let mut tree: BoxTree = BoxTree::new(tree_size, BRICK_DIMENSION).ok().unwrap();

        for x in 0..tree_size {
            for y in 0..tree_size {
                for z in 0..tree_size {
                    tree.insert(&V3c::new(x, y, z), voxel_data!(&5))
                        .ok()
                        .unwrap();
                }
            }
        }

        let ray = Ray {
            origin: V3c {
                x: 15.8443775,
                y: 16.0,
                z: 2.226141,
            },
            direction: V3c {
                x: -0.7984906,
                y: -0.60134345,
                z: 0.028264323,
            },
        };
        assert!(tree.get_by_ray(&ray).is_some());
        assert!(tree.get_by_ray(&ray).unwrap().0 == voxel_data!(&5));
    }

    #[test]
    fn test_edge_case_detailed_brick_z_edge_error() {
        let tree_size = 8;
        const BRICK_DIMENSION: u32 = 2;
        let mut tree: BoxTree = BoxTree::new(tree_size, BRICK_DIMENSION).ok().unwrap();

        for x in 1..tree_size {
            for y in 1..tree_size {
                for z in 1..tree_size {
                    tree.insert(&V3c::new(x, y, z), &Albedo::from(z))
                        .ok()
                        .unwrap();
                }
            }
        }

        let ray = Ray {
            origin: V3c {
                x: 11.92238,
                y: 16.0,
                z: -10.670372,
            },
            direction: V3c {
                x: -0.30062392,
                y: -0.6361918,
                z: 0.7105529,
            },
        };
        assert!(tree.get_by_ray(&ray).is_some_and(|v| {
            v.0 == (&Albedo::from(1)).into() && v.2 == V3c::<f32>::new(0., 0., -1.)
        }));
    }

    #[test]
    fn test_edge_case_deep_stack() {
        let tree_size = 1024;
        const BRICK_DIMENSION: u32 = 1;
        let mut tree: BoxTree = BoxTree::new(tree_size, BRICK_DIMENSION).ok().unwrap();

        let target = V3c::new(tree_size - 1, tree_size - 1, tree_size - 1);

        tree.insert(&V3c::new(0, 0, 0), &Albedo::from(0x000000EE))
            .ok()
            .unwrap();
        tree.insert(&target, &Albedo::from(0x000000FF))
            .ok()
            .unwrap();

        let origin = V3c {
            x: 0.,
            y: 5.,
            z: -1.,
        };
        let direction = (V3c::from(target) + V3c::unit(0.5) - origin).normalized();
        let ray = Ray { origin, direction };
        assert!(tree
            .get_by_ray(&ray)
            .is_some_and(|v| { v.0 == (&Albedo::from(0x000000FF)).into() }));
    }

    #[test]
    fn test_edge_case_brick_traversal_error() {
        let tree_size = 8;
        const BRICK_DIMENSION: u32 = 2;
        let mut tree: BoxTree = BoxTree::new(tree_size, BRICK_DIMENSION).ok().unwrap();

        tree.insert(&V3c::new(0, 0, 0), &Albedo::from(0x000000FF))
            .ok()
            .unwrap();

        let ray = Ray {
            origin: V3c {
                x: 23.84362,
                y: 32.0,
                z: -21.342018,
            },
            direction: V3c {
                x: -0.51286834,
                y: -0.70695364,
                z: 0.48701409,
            },
        };
        let hit = tree.get_by_ray(&ray);
        assert!(hit.is_some());

        let hit = hit.unwrap();
        assert_eq!(hit.0, (&Albedo::from(0x000000FF)).into());
        assert!((hit.2 - V3c::<f32>::new(0., 0., 0.)).length() < 1.1);
    }

    #[test]
    fn test_edge_case_brick_boundary_error() {
        const BRICK_DIMENSION: u32 = 8;
        const TREE_SIZE: u32 = 128;
        let mut tree: BoxTree = BoxTree::new(TREE_SIZE, BRICK_DIMENSION).ok().unwrap();

        for x in 0..TREE_SIZE {
            for y in 0..TREE_SIZE {
                for z in 0..TREE_SIZE {
                    if ((x < (TREE_SIZE / 4) || y < (TREE_SIZE / 4) || z < (TREE_SIZE / 4))
                        && (0 == x % 2 && 0 == y % 4 && 0 == z % 2))
                        || ((TREE_SIZE / 2) <= x && (TREE_SIZE / 2) <= y && (TREE_SIZE / 2) <= z)
                    {
                        tree.insert(
                            &V3c::new(x, y, z),
                            &Albedo::default()
                                .with_red((255 as f32 * (x % 6) as f32 / 6.0) as u8)
                                .with_green((255 as f32 * (y % 6) as f32 / 6.0) as u8)
                                .with_blue((255 as f32 * (z % 6) as f32 / 6.0) as u8)
                                .with_alpha(255),
                        )
                        .ok()
                        .unwrap();
                    }
                }
            }
        }

        let ray = Ray {
            origin: V3c {
                x: 191.60886,
                y: 256.0,
                z: -169.77057,
            },
            direction: V3c {
                x: -0.38838777,
                y: -0.49688956,
                z: 0.7760514,
            },
        };
        let hit = tree.get_by_ray(&ray);
        assert!(hit.is_some());
    }

    #[test]
    fn test_edge_case_cube_flaps() {
        const TREE_SIZE: u32 = 64;
        const BRICK_DIMENSION: u32 = 1;
        let mut tree: BoxTree = BoxTree::new(TREE_SIZE, BRICK_DIMENSION).ok().unwrap();

        for x in 0..TREE_SIZE {
            for y in 0..TREE_SIZE {
                for z in 0..TREE_SIZE {
                    if (TREE_SIZE / 2) <= x && (TREE_SIZE / 2) <= y && (TREE_SIZE / 2) <= z {
                        tree.insert(
                            &V3c::new(x, y, z),
                            &Albedo::default()
                                .with_red((255 as f32 * x as f32 / TREE_SIZE as f32) as u8)
                                .with_green((255 as f32 * y as f32 / TREE_SIZE as f32) as u8)
                                .with_blue((255 as f32 * z as f32 / TREE_SIZE as f32) as u8)
                                .with_alpha(255),
                        )
                        .ok()
                        .unwrap();
                    }
                }
            }
        }

        let ray = Ray {
            origin: V3c {
                x: 47.898006,
                y: 64.0,
                z: -42.44739,
            },
            direction: V3c {
                x: -0.42279032,
                y: -0.4016629,
                z: 0.8123516,
            },
        };
        let hit = tree.get_by_ray(&ray);
        assert!(hit.is_none());
    }

    #[test]
    fn test_edge_case_context_bleed() {
        const TREE_SIZE: u32 = 64;
        const BRICK_DIMENSION: u32 = 1;
        let mut tree: BoxTree = BoxTree::new(TREE_SIZE, BRICK_DIMENSION).ok().unwrap();

        for x in 0..TREE_SIZE {
            for y in 0..TREE_SIZE {
                for z in 0..TREE_SIZE {
                    if (x < (TREE_SIZE / 4) || y < (TREE_SIZE / 4) || z < (TREE_SIZE / 4))
                        && (0 == x % 2 && 0 == y % 4 && 0 == z % 2)
                    {
                        tree.insert(
                            &V3c::new(x, y, z),
                            &Albedo::default()
                                .with_red((255 as f32 * x as f32 / TREE_SIZE as f32) as u8)
                                .with_green((255 as f32 * y as f32 / TREE_SIZE as f32) as u8)
                                .with_blue((255 as f32 * z as f32 / TREE_SIZE as f32) as u8)
                                .with_alpha(255),
                        )
                        .ok()
                        .unwrap();
                    }
                }
            }
        }

        let ray = Ray {
            origin: V3c {
                x: 47.898006,
                y: 64.0,
                z: -42.44739,
            },
            direction: V3c {
                x: -0.49263135,
                y: -0.49703234,
                z: 0.714334,
            },
        };
        let hit = tree.get_by_ray(&ray);
        assert!(hit.is_some());
    }
}

#[cfg(test)]
mod node_stack_tests {
    use crate::raytracing::raytracing_on_cpu::NodeStack;

    #[test]
    fn test_stack_is_empty() {
        let stack: NodeStack<i32> = NodeStack::default();
        assert!(stack.is_empty(), "Stack should be empty on initialization");
    }

    #[test]
    fn test_stack_push_and_wrap_around() {
        let mut stack: NodeStack<i32, 3> = NodeStack::default();

        stack.push(1);
        stack.push(2);
        stack.push(3);
        stack.push(4); // This should overwrite the first element (1)

        assert_eq!(
            stack.last(),
            Some(&4),
            "Last element should be 4 after push"
        );
        assert_eq!(stack.pop(), Some(4), "Popped element should be 4");

        assert_eq!(
            stack.last(),
            Some(&3),
            "Last element should be 3 after popping 4"
        );
        assert_eq!(stack.pop(), Some(3), "Popped element should be 3");

        assert_eq!(
            stack.last(),
            Some(&2),
            "Last element should be 2 after popping 3"
        );
        assert_eq!(stack.pop(), Some(2), "Popped element should be 2");

        assert_eq!(
            stack.pop(),
            None,
            "Popping again should return None since stack is empty"
        );
    }

    #[test]
    fn test_stack_last() {
        let mut stack: NodeStack<i32, 3> = NodeStack::default();
        stack.push(10);
        stack.push(20);
        stack.push(30);
        assert_eq!(stack.last(), Some(&30), "Last element should be 30");

        stack.push(40); // This should overwrite the first element (10)
        assert_eq!(
            stack.last(),
            Some(&40),
            "Last element should be 40 after pushing 40"
        );
    }

    #[test]
    fn test_stack_last_mut() {
        let mut stack: NodeStack<i32, 3> = NodeStack::default();
        stack.push(100);
        stack.push(200);
        stack.push(300);

        if let Some(last_mut) = stack.last_mut() {
            *last_mut += 50;
        }
        assert_eq!(
            stack.last(),
            Some(&350),
            "Last element should be 350 after mutation"
        );
    }

    #[test]
    fn test_stack_pop_until_empty() {
        let mut stack: NodeStack<i32, 3> = NodeStack::default();
        stack.push(5);
        stack.push(15);
        stack.push(25);

        assert_eq!(stack.pop(), Some(25), "Popped element should be 25");
        assert_eq!(stack.pop(), Some(15), "Popped element should be 15");
        assert_eq!(stack.pop(), Some(5), "Popped element should be 5");
        assert_eq!(
            stack.pop(),
            None,
            "Popping from an empty stack should return None"
        );
    }
}
