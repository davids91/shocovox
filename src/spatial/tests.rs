#[cfg(test)]
mod vector_tests {

    use crate::spatial::V3c;

    #[test]
    fn test_cross_product() {
        let a = V3c::new(3., 0., 2.);
        let b = V3c::new(-1., 4., 2.);
        let cross = a.cross(b);
        assert!(cross.x == -8.);
        assert!(cross.y == -8.);
        assert!(cross.z == 12.);
    }
}

#[cfg(test)]
mod octant_tests {

    use crate::spatial::math::hash_region;
    use crate::spatial::math::offset_region;
    use crate::spatial::V3c;

    #[test]
    fn test_hash_region() {
        assert!(hash_region(&V3c::new(0.0, 0.0, 0.0), 10.0) == 0);
        assert!(hash_region(&V3c::new(6.0, 0.0, 0.0), 10.0) == 1);
        assert!(hash_region(&V3c::new(0.0, 0.0, 6.0), 10.0) == 2);
        assert!(hash_region(&V3c::new(6.0, 0.0, 6.0), 10.0) == 3);
        assert!(hash_region(&V3c::new(0.0, 6.0, 0.0), 10.0) == 4);
        assert!(hash_region(&V3c::new(6.0, 6.0, 0.0), 10.0) == 5);
        assert!(hash_region(&V3c::new(0.0, 6.0, 6.0), 10.0) == 6);
        assert!(hash_region(&V3c::new(6.0, 6.0, 6.0), 10.0) == 7);
    }

    #[test]
    fn test_offset_region() {
        assert!(V3c::new(0., 0., 0.) == offset_region(0));
        assert!(V3c::new(1., 0., 0.) == offset_region(1));
        assert!(V3c::new(0., 0., 1.) == offset_region(2));
        assert!(V3c::new(1., 0., 1.) == offset_region(3));
        assert!(V3c::new(0., 1., 0.) == offset_region(4));
        assert!(V3c::new(1., 1., 0.) == offset_region(5));
        assert!(V3c::new(0., 1., 1.) == offset_region(6));
        assert!(V3c::new(1., 1., 1.) == offset_region(7));
    }
}

#[cfg(feature = "raytracing")]
#[cfg(test)]
mod intersection_tests {

    use crate::spatial::{math::plane_line_intersection, raytracing::Ray, Cube, V3c};

    #[test]
    fn test_negative_intersection() {
        let plane_point = V3c::new(0., 0., 0.);
        let plane_normal = V3c::new(0., 1., 0.);
        let line_origin = V3c::new(0., 1., 0.);
        let line_direction = V3c::new(0., 1., 0.);
        assert!(plane_line_intersection(
            &plane_point,
            &plane_normal,
            &line_origin,
            &line_direction
        )
        .is_some_and(|v| v == -1.));
    }

    #[test]
    fn test_edge_case_cube_top_hit() {
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
        let t_hit = (Cube {
            min_position: V3c::new(2.0, 0.0, 0.0),
            size: 2.0,
        })
        .intersect_ray(&ray)
        .unwrap();

        assert!(t_hit
            .impact_distance
            .is_some_and(|v| (v - 11.077772).abs() < 0.001));
        assert!((ray.point_at(t_hit.impact_distance.unwrap()).y - 2.).abs() < 0.001);
    }
}

#[cfg(feature = "raytracing")]
#[cfg(test)]
mod raytracing_tests {
    use crate::spatial::{math::plane_line_intersection, raytracing::Ray, Cube, V3c};

    #[test]
    fn test_plane_line_intersection() {
        assert!(
            plane_line_intersection(
                // plane
                &V3c::new(0., 0., 0.),
                &V3c::new(0., 1., 0.),
                // line
                &V3c::new(0., 1., 0.),
                &V3c::new(1., 0., 0.),
            ) == None
        );

        assert!(
            plane_line_intersection(
                // plane
                &V3c::new(0., 0., 0.),
                &V3c::new(0., 1., 0.),
                // line
                &V3c::new(0., 1., 0.),
                &V3c::new(0., -1., 0.),
            ) == Some(1.)
        );

        assert!(
            plane_line_intersection(
                // plane
                &V3c::new(0., 0., 0.),
                &V3c::new(0., 1., 0.),
                // line
                &V3c::new(0., 0., 0.),
                &V3c::new(1., 0., 0.),
            ) == Some(0.)
        );
    }

    #[test]
    fn test_cube_bounds() {
        let cube = Cube {
            min_position: V3c::default(),
            size: 10.0,
        };

        // Test front bottom left
        let bound_fbl = Cube::child_bounds_for(&cube, 0);
        assert!(bound_fbl.min_position == V3c::unit(0.0));
        assert!(bound_fbl.size == 5.0);

        // Test front bottom right
        let bound_fbl = Cube::child_bounds_for(&cube, 1);
        assert!(bound_fbl.min_position == V3c::new(5.0, 0.0, 0.0));
        assert!(bound_fbl.size == 5.0);

        // Test back bottom left
        let bound_fbl = Cube::child_bounds_for(&cube, 2);
        assert!(bound_fbl.min_position == V3c::new(0.0, 0.0, 5.0));
        assert!(bound_fbl.size == 5.0);

        // Test back bottom right
        let bound_fbl = Cube::child_bounds_for(&cube, 3);
        assert!(bound_fbl.min_position == V3c::new(5.0, 0.0, 5.0));
        assert!(bound_fbl.size == 5.0);

        // Test front top left
        let bound_fbl = Cube::child_bounds_for(&cube, 4);
        assert!(bound_fbl.min_position == V3c::new(0.0, 5.0, 0.0));
        assert!(bound_fbl.size == 5.0);

        // Test front top right
        let bound_fbl = Cube::child_bounds_for(&cube, 5);
        assert!(bound_fbl.min_position == V3c::new(5.0, 5.0, 0.0));
        assert!(bound_fbl.size == 5.0);

        // Test back top left
        let bound_fbl = Cube::child_bounds_for(&cube, 6);
        assert!(bound_fbl.min_position == V3c::new(0.0, 5.0, 5.0));
        assert!(bound_fbl.size == 5.0);

        // Test back top right
        let bound_fbl = Cube::child_bounds_for(&cube, 7);
        assert!(bound_fbl.min_position == V3c::new(5.0, 5.0, 5.0));
        assert!(bound_fbl.size == 5.0);
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_cube_contains_ray() {
        let cube = Cube {
            min_position: V3c::unit(0.0),
            size: 4.0,
        };

        let ray_above = Ray {
            origin: V3c {
                x: 2.,
                y: 5.,
                z: 2.,
            },
            direction: V3c {
                x: 0.,
                y: -1.,
                z: 0.,
            },
        };
        assert!(cube.intersect_ray(&ray_above).is_some());

        let ray_below = Ray {
            origin: V3c {
                x: 2.,
                y: -5.,
                z: 2.,
            },
            direction: V3c {
                x: 0.,
                y: 1.,
                z: 0.,
            },
        };
        assert!(cube.intersect_ray(&ray_below).is_some());

        let ray_miss = Ray {
            origin: V3c {
                x: 2.,
                y: 5.,
                z: 2.,
            },
            direction: V3c {
                x: 0.,
                y: 1.,
                z: 0.,
            },
        };
        assert!(cube.intersect_ray(&ray_miss).is_none());

        let ray_hit = Ray {
            origin: V3c {
                x: -1.,
                y: -1.,
                z: -1.,
            },
            direction: V3c {
                x: 1.,
                y: 1.,
                z: 1.,
            }
            .normalized(),
        };

        assert!(cube.intersect_ray(&ray_hit).is_some());

        let corner_hit = Ray {
            origin: V3c {
                x: -1.,
                y: -1.,
                z: -1.,
            },
            direction: V3c {
                x: 1.,
                y: 1.,
                z: 1.,
            }
            .normalized(),
        };

        assert!(cube.intersect_ray(&corner_hit).is_some());

        let origin = V3c {
            x: 4.,
            y: -1.,
            z: 4.,
        };
        let corner_miss = Ray {
            direction: (V3c {
                x: 4.055,
                y: 4.055,
                z: 4.055,
            } - origin)
                .normalized(),
            origin,
        };
        assert!(!cube.intersect_ray(&corner_miss).is_some());

        let ray_still_miss = Ray {
            origin: V3c {
                x: -1.,
                y: -1.,
                z: -1.,
            },
            direction: V3c {
                x: 1.,
                y: 100.,
                z: 1.,
            }
            .normalized(),
        };
        assert!(cube.intersect_ray(&ray_still_miss).is_none());
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_edge_case_cube_intersect_inwards_pointing_vector() {
        let ray = Ray {
            origin: V3c {
                x: 8.0,
                y: 4.0,
                z: 5.0,
            },
            direction: V3c {
                x: -0.842701,
                y: -0.24077171,
                z: -0.48154342,
            },
        };
        let cube = Cube {
            min_position: V3c {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            size: 8.0,
        };
        let hit = cube.intersect_ray(&ray).unwrap();
        assert!(hit.impact_distance.is_some() && hit.impact_distance.unwrap() == 0.);
        assert!(hit.exit_distance > 0.);
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_edge_case_cube_internal_ray_targeting_corners() {
        let ray = Ray {
            origin: V3c {
                x: 5.0,
                y: 8.0,
                z: 5.0,
            },
            direction: V3c {
                x: -0.48507127,
                y: -0.7276069,
                z: -0.48507127,
            },
        };
        let cube = Cube {
            min_position: V3c {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            size: 16.0,
        };
        let hit = cube.intersect_ray(&ray).unwrap();
        assert!(hit.impact_distance.is_none());
        assert!(hit.exit_distance > 0.);
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_edge_case_cube_bottom_edge() {
        let ray = Ray {
            origin: V3c {
                x: 6.0,
                y: 7.0,
                z: 6.0,
            },
            direction: V3c {
                x: -0.6154574,
                y: -0.49236596,
                z: -0.6154574,
            },
        };
        let cube = Cube {
            min_position: V3c {
                x: 0.0,
                y: 2.0,
                z: 0.0,
            },
            size: 2.0,
        };
        let hit = cube.intersect_ray(&ray).unwrap();
        assert!(hit
            .impact_distance
            .is_some_and(|d| (d > 0.0) && d < hit.exit_distance));
    }
}
