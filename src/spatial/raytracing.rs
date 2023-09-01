#[cfg(feature = "raytracing")]
use crate::spatial::{
    math::{plane_line_intersection_distance, V3c},
    Cube,
};

#[cfg(feature = "raytracing")]
pub(crate) const FLOAT_ERROR_TOLERANCE: f32 = 0.005;

#[cfg(feature = "raytracing")]
#[derive(Debug)]
pub(crate) enum CubeFaces {
    FRONT,
    LEFT,
    REAR,
    RIGHT,
    TOP,
    BOTTOM,
}

#[cfg(feature = "raytracing")]
impl CubeFaces {
    pub(crate) fn into_iter() -> core::array::IntoIter<CubeFaces, 6> {
        [
            CubeFaces::FRONT,
            CubeFaces::LEFT,
            CubeFaces::REAR,
            CubeFaces::RIGHT,
            CubeFaces::TOP,
            CubeFaces::BOTTOM,
        ]
        .into_iter()
    }
}

#[cfg(feature = "raytracing")]
#[derive(Debug)]
pub struct Ray {
    pub origin: V3c<f32>,
    pub direction: V3c<f32>,
}

#[cfg(feature = "raytracing")]
impl Ray {
    pub fn is_valid(&self) -> bool {
        (1. - self.direction.length()).abs() < 0.000001
    }

    pub fn point_at(&self, d: f32) -> V3c<f32> {
        self.origin + self.direction * d
    }
}

#[cfg(feature = "raytracing")]
#[derive(Debug, Copy, Clone, Default)]
pub struct CubeHit {
    pub(crate) impact_distance: Option<f32>,
    pub(crate) exit_distance: f32,
    pub(crate) impact_normal: V3c<f32>,
}

impl Cube {
    #[cfg(feature = "raytracing")]
    pub fn face(&self, face: CubeFaces) -> Ray {
        let midpoint = self.midpoint();
        match face {
            CubeFaces::FRONT => {
                let direction = V3c::new(0., 0., -1.);
                Ray {
                    origin: midpoint + direction * (self.size as f32 / 2.),
                    direction,
                }
            }
            CubeFaces::LEFT => {
                let direction = V3c::new(-1., 0., 0.);
                Ray {
                    origin: midpoint + direction * (self.size as f32 / 2.),
                    direction,
                }
            }
            CubeFaces::REAR => {
                let direction = V3c::new(0., 0., 1.);
                Ray {
                    origin: midpoint + direction * (self.size as f32 / 2.),
                    direction,
                }
            }
            CubeFaces::RIGHT => {
                let direction = V3c::new(1., 0., 0.);
                Ray {
                    origin: midpoint + direction * (self.size as f32 / 2.),
                    direction,
                }
            }
            CubeFaces::TOP => {
                let direction = V3c::new(0., 1., 0.);
                Ray {
                    origin: midpoint + direction * (self.size as f32 / 2.),
                    direction,
                }
            }
            CubeFaces::BOTTOM => {
                let direction = V3c::new(0., -1., 0.);
                Ray {
                    origin: midpoint + direction * (self.size as f32 / 2.),
                    direction,
                }
            }
        }
    }

    /// Tells the intersection with the cube of the given ray.
    /// returns the distance from the origin to the direction of the ray until the hit point and the normal of the hit
    #[cfg(feature = "raytracing")]
    pub fn intersect_ray(&self, ray: &Ray) -> Option<CubeHit> {
        assert!(ray.is_valid());
        let mut distances: Vec<f32> = Vec::new();
        let mut impact_normal = V3c::default();

        if self.includes_point(&ray.origin) {
            distances.push(0.);
        }

        for f in CubeFaces::into_iter() {
            let face = &self.face(f);
            if let Some(d) = plane_line_intersection_distance(
                &face.origin,
                &face.direction,
                &ray.origin,
                &ray.direction,
            ) {
                if 0. <= d && self.contains_point(&ray.point_at(d)) {
                    // ray hits the plane only when the resulting distance is at least positive,
                    // and the point is contained inside the cube
                    if distances.len() == 2 && (distances[0] - distances[1]).abs() < FLOAT_ERROR_TOLERANCE {
                        distances[1] = d; // the first 2 hits were of an edge or the corner of the cube, so one of them can be discarded
                    } else if distances.len() < 2 {
                        distances.push(d); // not enough hits are gathered
                    } else {
                        break; // enough hits are gathered, exit the loop
                    }
                    if !distances.is_empty() && d <= distances[0] {
                        impact_normal = face.direction;
                    }
                }
            }
        }
        if 1 < distances.len() {
            Some(CubeHit {
                impact_distance: Some(distances[0].min(distances[1])),
                exit_distance: distances[0].max(distances[1]),
                impact_normal,
            })
        } else if !distances.is_empty() {
            Some(CubeHit {
                impact_distance: None,
                exit_distance: distances[0],
                impact_normal,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod raytracing_tests {
    #[cfg(feature = "raytracing")]
    use crate::spatial::math::plane_line_intersection_distance;

    use super::Cube;
    #[cfg(feature = "raytracing")]
    use super::Ray;
    use super::V3c;

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_plane_line_intersection() {
        assert!(
            plane_line_intersection_distance(
                // plane
                &V3c::new(0., 0., 0.),
                &V3c::new(0., 1., 0.),
                // line
                &V3c::new(0., 1., 0.),
                &V3c::new(1., 0., 0.),
            ) == None
        );

        assert!(
            plane_line_intersection_distance(
                // plane
                &V3c::new(0., 0., 0.),
                &V3c::new(0., 1., 0.),
                // line
                &V3c::new(0., 1., 0.),
                &V3c::new(0., -1., 0.),
            ) == Some(1.)
        );

        assert!(
            plane_line_intersection_distance(
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
            size: 10,
        };

        // Test front bottom left
        let bound_fbl = Cube::child_bounds_for(&cube, 0);
        assert!(bound_fbl.min_position == V3c::unit(0));
        assert!(bound_fbl.size == 5);

        // Test front bottom right
        let bound_fbl = Cube::child_bounds_for(&cube, 1);
        assert!(bound_fbl.min_position == V3c::new(5, 0, 0));
        assert!(bound_fbl.size == 5);

        // Test back bottom left
        let bound_fbl = Cube::child_bounds_for(&cube, 2);
        assert!(bound_fbl.min_position == V3c::new(0, 0, 5));
        assert!(bound_fbl.size == 5);

        // Test back bottom right
        let bound_fbl = Cube::child_bounds_for(&cube, 3);
        assert!(bound_fbl.min_position == V3c::new(5, 0, 5));
        assert!(bound_fbl.size == 5);

        // Test front top left
        let bound_fbl = Cube::child_bounds_for(&cube, 4);
        assert!(bound_fbl.min_position == V3c::new(0, 5, 0));
        assert!(bound_fbl.size == 5);

        // Test front top right
        let bound_fbl = Cube::child_bounds_for(&cube, 5);
        assert!(bound_fbl.min_position == V3c::new(5, 5, 0));
        assert!(bound_fbl.size == 5);

        // Test back top left
        let bound_fbl = Cube::child_bounds_for(&cube, 6);
        assert!(bound_fbl.min_position == V3c::new(0, 5, 5));
        assert!(bound_fbl.size == 5);

        // Test back top right
        let bound_fbl = Cube::child_bounds_for(&cube, 7);
        assert!(bound_fbl.min_position == V3c::new(5, 5, 5));
        assert!(bound_fbl.size == 5);
    }

    #[cfg(feature = "raytracing")]
    #[test]
    fn test_cube_contains_ray() {
        let cube = Cube {
            min_position: V3c::unit(0),
            size: 4,
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

        let ray_on_edge = Ray {
            origin: V3c {
                x: 2.,
                y: 5.,
                z: 4.0,
            },
            direction: V3c {
                x: 0.,
                y: -1.,
                z: 0.,
            },
        };
        assert!(cube.intersect_ray(&ray_on_edge).is_some());

        let ray_on_corner = Ray {
            origin: V3c {
                x: 4.0,
                y: 5.,
                z: 4.0,
            },
            direction: V3c {
                x: 0.,
                y: -1.,
                z: 0.,
            },
        };
        assert!(cube.intersect_ray(&ray_on_corner).is_some());

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

        let corner_just_hit = Ray {
            direction: (V3c {
                x: 4.0,
                y: 4.0,
                z: 4.0,
            } - origin)
                .normalized(),
            origin,
        };
        assert!(cube.intersect_ray(&corner_just_hit).is_some());

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
            min_position: V3c { x: 0, y: 0, z: 0 },
            size: 8,
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
            min_position: V3c { x: 0, y: 0, z: 0 },
            size: 16,
        };
        let hit = cube.intersect_ray(&ray).unwrap();
        assert!(hit.impact_distance.is_some_and(|d| d == 0.));
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
            min_position: V3c { x: 0, y: 2, z: 0 },
            size: 2,
        };
        let hit = cube.intersect_ray(&ray).unwrap();
        assert!(hit
            .impact_distance
            .is_some_and(|d| (d > 0.0) && d < hit.exit_distance));
    }
}
