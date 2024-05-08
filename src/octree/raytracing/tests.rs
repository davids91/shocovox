#[cfg(test)]
mod wgpu_tests {
    #[test]
    fn test_special_key_values() {
        // assumptions in shader needs to be compared to factual values
        assert!(crate::object_pool::key_none_value() == 4294967295u32);
    }
}

#[cfg(test)]
mod octree_raytracing_tests {
    use crate::octree::{Octree, V3c};
    use crate::spatial::raytracing::Ray;
    use rand::{rngs::ThreadRng, Rng};

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
        let mut tree = Octree::<u32>::new(4).ok().unwrap();
        let mut filled = Vec::new();
        for x in 1..4 {
            for y in 1..4 {
                if 10 > rng.gen_range(0..20) {
                    let pos = V3c::new(x, y, 1);
                    tree.insert(&pos, 5 | 0xFF000000).ok().unwrap();
                    filled.push(pos);
                }
            }
        }

        for p in filled.into_iter() {
            let ray = make_ray_point_to(&V3c::new(p.x as f32, p.y as f32, p.z as f32), &mut rng);
            assert!(tree.get_by_ray(&ray).is_some());
            assert!(*tree.get_by_ray(&ray).unwrap().0 == 5 | 0xFF000000);
        }
    }

    #[test]
    fn test_get_by_ray_from_outside_where_dim_is_2() {
        let mut rng = rand::thread_rng();
        let mut tree = Octree::<u32, 2>::new(4).ok().unwrap();
        let mut filled = Vec::new();
        for x in 1..4 {
            for y in 1..4 {
                if 10 > rng.gen_range(0..20) {
                    let pos = V3c::new(x, y, 1);
                    tree.insert(&pos, 5 | 0xFF000000).ok().unwrap();
                    filled.push(pos);
                }
            }
        }

        for p in filled.into_iter() {
            let ray = make_ray_point_to(&V3c::new(p.x as f32, p.y as f32, p.z as f32), &mut rng);
            assert!(tree.get_by_ray(&ray).is_some());
            assert!(*tree.get_by_ray(&ray).unwrap().0 == 5 | 0xFF000000);
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
        let mut tree = Octree::<u32>::new(8).ok().unwrap();
        let mut filled = Vec::new();
        for x in 1..4 {
            for y in 1..4 {
                for z in 1..4 {
                    if 10 > rng.gen_range(0..20) {
                        let pos = V3c::new(x, y, z);
                        tree.insert(&pos, 5 | 0xFF000000).ok().unwrap();
                        filled.push(pos);
                    }
                }
            }
        }

        for p in filled.into_iter() {
            println!("targeting point: {:?}", p);
            let ray = make_edge_ray_point_to(
                &V3c::new(p.x as f32 + 0.1, p.y as f32 + 0.1, p.z as f32 + 0.1),
                &mut rng,
            );
            assert!(tree.get_by_ray(&ray).is_some());
            assert!(*tree.get_by_ray(&ray).unwrap().0 == 5 | 0xFF000000);
        }
    }

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
                        tree.insert(&pos, 5 | 0xFF000000).ok().unwrap();
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
            assert!(*tree.get_by_ray(&ray).unwrap().0 == 5 | 0xFF000000);
        }
    }

    #[test]
    fn test_edge_case_unreachable() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();
        tree.insert(&V3c::new(3, 0, 0), 0 | 0xFF000000)
            .ok()
            .unwrap();
        tree.insert(&V3c::new(3, 3, 0), 1 | 0xFF000000)
            .ok()
            .unwrap();
        tree.insert(&V3c::new(0, 3, 0), 2 | 0xFF000000)
            .ok()
            .unwrap();

        for y in 0..4 {
            tree.insert(&V3c::new(0, y, y), 3 | 0xFF000000)
                .ok()
                .unwrap();
            tree.insert(&V3c::new(1, y, y), 3 | 0xFF000000)
                .ok()
                .unwrap();
            tree.insert(&V3c::new(2, y, y), 3 | 0xFF000000)
                .ok()
                .unwrap();
            tree.insert(&V3c::new(3, y, y), 3 | 0xFF000000)
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
        let mut tree = Octree::<u32>::new(4).ok().unwrap();
        tree.insert(&V3c::new(2, 1, 1), 3 | 0xFF000000).ok();

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
        let mut tree = Octree::<u32>::new(4).ok().unwrap();
        tree.insert(&V3c::new(3, 0, 0), 0 | 0xFF000000)
            .ok()
            .unwrap();
        tree.insert(&V3c::new(3, 3, 0), 1 | 0xFF000000)
            .ok()
            .unwrap();
        tree.insert(&V3c::new(0, 3, 0), 2 | 0xFF000000)
            .ok()
            .unwrap();

        for y in 0..4 {
            tree.insert(&V3c::new(0, y, y), 3 | 0xFF000000)
                .ok()
                .unwrap();
            tree.insert(&V3c::new(1, y, y), 3 | 0xFF000000)
                .ok()
                .unwrap();
            tree.insert(&V3c::new(2, y, y), 3 | 0xFF000000)
                .ok()
                .unwrap();
            tree.insert(&V3c::new(3, y, y), 3 | 0xFF000000)
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
    fn test_edge_case_cube_edges() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();
        tree.insert(&V3c::new(3, 0, 0), 0 | 0xFF000000)
            .ok()
            .unwrap();
        tree.insert(&V3c::new(3, 3, 0), 1 | 0xFF000000)
            .ok()
            .unwrap();
        tree.insert(&V3c::new(0, 3, 0), 2 | 0xFF000000)
            .ok()
            .unwrap();

        for y in 0..4 {
            tree.insert(&V3c::new(0, y, y), 3 | 0xFF000000)
                .ok()
                .unwrap();
            tree.insert(&V3c::new(1, y, y), 4 | 0xFF000000)
                .ok()
                .unwrap();
            tree.insert(&V3c::new(2, y, y), 5 | 0xFF000000)
                .ok()
                .unwrap();
            tree.insert(&V3c::new(3, y, y), 6 | 0xFF000000)
                .ok()
                .unwrap();
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
        assert!(tree
            .get_by_ray(&ray)
            .is_some_and(|v| *v.0 == 6 | 0xFF000000));
    }

    #[test]
    fn test_edge_case_ray_behind_octree() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();
        tree.insert(&V3c::new(0, 3, 0), 5 | 0xFF000000)
            .ok()
            .unwrap();
        let origin = V3c::new(2., 2., -5.);
        let ray = Ray {
            direction: (V3c::new(0., 3., 0.) - origin).normalized(),
            origin,
        };
        assert!(tree.get(&V3c::new(0, 3, 0)).is_some());
        assert!(*tree.get(&V3c::new(0, 3, 0)).unwrap() == 5 | 0xFF000000);
        assert!(tree.get_by_ray(&ray).is_some());
        assert!(*tree.get_by_ray(&ray).unwrap().0 == 5 | 0xFF000000);
    }

    #[test]
    fn test_edge_case_overlapping_voxels() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();
        tree.insert(&V3c::new(0, 0, 0), 5 | 0xFF000000)
            .ok()
            .unwrap();
        tree.insert(&V3c::new(1, 0, 0), 6 | 0xFF000000)
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
            .is_some_and(|hit| *hit.0 == 6 | 0xFF000000));
    }

    #[test]
    fn test_edge_case_edge_raycast() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();

        for x in 0..4 {
            for z in 0..4 {
                tree.insert(&V3c::new(x, 0, z), 5 | 0xFF000000)
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
        assert!(result.is_none() || *result.unwrap().0 == 5 | 0xFF000000);
    }

    #[test]
    fn test_edge_case_voxel_corner() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();

        for x in 0..4 {
            for z in 0..4 {
                tree.insert(&V3c::new(x, 0, z), 5 | 0xFF000000)
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
        assert!(*tree.get_by_ray(&ray).unwrap().0 == 5 | 0xFF000000);
    }

    #[test]
    fn test_edge_case_bottom_edge() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();

        for x in 0..4 {
            for z in 0..4 {
                tree.insert(&V3c::new(x, 0, z), 5 | 0xFF000000)
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
        assert!(*tree.get_by_ray(&ray).unwrap().0 == 5 | 0xFF000000);
    }

    #[test]
    fn test_edge_case_loop_stuck() {
        let mut tree = Octree::<u32>::new(4).ok().unwrap();
        tree.insert(&V3c::new(3, 0, 0), 0 | 0xFF000000)
            .ok()
            .unwrap();
        tree.insert(&V3c::new(3, 3, 0), 1 | 0xFF000000)
            .ok()
            .unwrap();
        tree.insert(&V3c::new(0, 3, 0), 2 | 0xFF000000)
            .ok()
            .unwrap();

        for y in 0..4 {
            tree.insert(&V3c::new(0, y, y), 3 | 0xFF000000)
                .ok()
                .unwrap();
            tree.insert(&V3c::new(1, y, y), 4 | 0xFF000000)
                .ok()
                .unwrap();
            tree.insert(&V3c::new(2, y, y), 5 | 0xFF000000)
                .ok()
                .unwrap();
            tree.insert(&V3c::new(3, y, y), 6 | 0xFF000000)
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
    fn test_edge_case_matrix_undetected() {
        let mut tree = Octree::<u32, 4>::new(4).ok().unwrap();

        for x in 0..4 {
            for z in 0..4 {
                tree.insert(&V3c::new(x, 0, z), 5 | 0xFF000000)
                    .ok()
                    .unwrap();
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
        assert!(*tree.get_by_ray(&ray).unwrap().0 == 5 | 0xFF000000);
    }

    #[test]
    fn test_edge_case_detailed_matrix_undetected() {
        let tree_size = 8;
        const MATRIX_DIMENSION: usize = 2;
        let mut tree = Octree::<u32, MATRIX_DIMENSION>::new(tree_size)
            .ok()
            .unwrap();

        for x in 0..tree_size {
            for y in 0..tree_size {
                for z in 0..tree_size {
                    tree.insert(&V3c::new(x, y, z), 5 | 0xFF000000)
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
        assert!(*tree.get_by_ray(&ray).unwrap().0 == 5 | 0xFF000000);
    }

    #[test]
    fn test_edge_case_detailed_matrix_z_edge_error() {
        let tree_size = 8;
        const MATRIX_DIMENSION: usize = 2;
        let mut tree = Octree::<u32, MATRIX_DIMENSION>::new(tree_size)
            .ok()
            .unwrap();

        for x in 1..tree_size {
            for y in 1..tree_size {
                for z in 1..tree_size {
                    tree.insert(&V3c::new(x, y, z), z | 0xFF000000)
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
            println!("result is {:?}", v);
            *v.0 == 1 | 0xFF000000 && v.2 == V3c::<f32>::new(0., 0., -1.)
        }));
    }
}
