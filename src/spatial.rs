pub mod math;

///####################################################################################
/// Raytracing stuff
///####################################################################################
use crate::spatial::math::{offset_region, plane_line_intersection_distance, V3c};

#[derive(Debug)]
pub struct Ray {
    pub origin: V3c<f32>,
    pub direction: V3c<f32>,
}

impl Ray {
    pub fn is_valid(&self) -> bool {
        (1. - self.direction.length()).abs() < 0.000001
    }

    pub fn point_at(&self, d: f32) -> V3c<f32> {
        self.origin + self.direction * d
    }
}

#[derive(Debug)]
pub struct CubeHit {
    pub(crate) impact_distance: Option<f32>,
    pub(crate) exit_distance: f32,
    pub(crate) impact_normal: V3c<f32>,
}

#[derive(Debug)]
pub enum CubeFaces {
    FRONT,
    LEFT,
    REAR,
    RIGHT,
    TOP,
    BOTTOM,
}

impl CubeFaces {
    pub fn into_iter() -> core::array::IntoIter<CubeFaces, 6> {
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

#[derive(Default, Debug)]
pub struct Cube {
    pub min_position: V3c<u32>,
    pub size: u32,
}

impl Cube {
    pub fn midpoint(&self) -> V3c<f32> {
        V3c::unit(self.size as f32 / 2.) + self.min_position.into()
    }

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

    /// Creates a bounding box within an area described by the min_position and size, for the given octant
    pub(crate) fn child_bounds_for(min_position: V3c<u32>, size: u32, octant: usize) -> Cube {
        let child_size = size / 2;
        Cube {
            min_position: (min_position + (offset_region(octant) * child_size).into()).into(),
            size: child_size,
        }
    }

    /// Tells the intersection with the cube of the given ray.
    /// returns the distance from the origin to the direction of the ray until the hit point and the normal of the hit
    pub fn intersect_ray(&self, ray: &Ray) -> Option<CubeHit> {
        assert!(ray.is_valid());
        let mut faces_hit = 0;
        let mut distances = Vec::new();
        let mut impact_normal = V3c::default();
        for f in CubeFaces::into_iter() {
            let face = &self.face(f);
            if let Some(d) = plane_line_intersection_distance(
                &face.origin,
                &face.direction,
                &ray.origin,
                &ray.direction,
            ) {
                // d hits the plane
                if 0. < d && self.contains_point(&ray.point_at(d)) {
                    distances.push(d);
                    if 0 < distances.len() && d <= distances[0] {
                        impact_normal = face.direction;
                    }
                    faces_hit += 1;
                }
            }
            if 2 == faces_hit {
                break;
            }
        }
        if 1 < faces_hit {
            return Some(CubeHit {
                impact_distance: Some(distances[0].min(distances[1])),
                exit_distance: distances[0].max(distances[1]),
                impact_normal,
            });
        } else if 0 < faces_hit{
            return Some(CubeHit {
                impact_distance: None,
                exit_distance: distances[0],
                impact_normal,
            });
        }else {None}
    }

    pub fn contains_ray(&self, ray: &Ray) -> bool {
        self.intersect_ray(ray).is_some()
    }

    pub fn contains_point(&self, point: &V3c<f32>) -> bool {
        let edges_epsilon = 0.000001;
        (point.x >= self.min_position.x as f32 - edges_epsilon)
            && (point.x <= (self.min_position.x + self.size) as f32 + edges_epsilon)
            && (point.y >= self.min_position.y as f32 - edges_epsilon)
            && (point.y <= (self.min_position.y + self.size) as f32 + edges_epsilon)
            && (point.z >= self.min_position.z as f32 - edges_epsilon)
            && (point.z <= (self.min_position.z + self.size) as f32 + edges_epsilon)
    }
}

#[cfg(test)]
mod raytracing_tests {
    use crate::spatial::math::plane_line_intersection_distance;

    use super::Cube;
    use super::Ray;
    use super::V3c;

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
        let cube = Cube{
            min_position: V3c::default(),
            size: 10
        };

        // Test front bottom left
        let bound_fbl = Cube::child_bounds_for(cube.min_position, cube.size, 0);
        assert!(bound_fbl.min_position == V3c::unit(0));
        assert!(bound_fbl.size == 5);

        // Test front bottom right
        let bound_fbl = Cube::child_bounds_for(cube.min_position, cube.size, 1);
        assert!(bound_fbl.min_position == V3c::new(5,0,0));
        assert!(bound_fbl.size == 5);

        // Test back bottom left
        let bound_fbl = Cube::child_bounds_for(cube.min_position, cube.size, 2);
        assert!(bound_fbl.min_position == V3c::new(0,0,5));
        assert!(bound_fbl.size == 5);

        // Test back bottom right
        let bound_fbl = Cube::child_bounds_for(cube.min_position, cube.size, 3);
        assert!(bound_fbl.min_position == V3c::new(5,0,5));
        assert!(bound_fbl.size == 5);

        // Test front top left
        let bound_fbl = Cube::child_bounds_for(cube.min_position, cube.size, 4);
        assert!(bound_fbl.min_position == V3c::new(0,5,0));
        assert!(bound_fbl.size == 5);

        // Test front top right
        let bound_fbl = Cube::child_bounds_for(cube.min_position, cube.size, 5);
        assert!(bound_fbl.min_position == V3c::new(5,5,0));
        assert!(bound_fbl.size == 5);

        // Test back top left
        let bound_fbl = Cube::child_bounds_for(cube.min_position, cube.size, 6);
        assert!(bound_fbl.min_position == V3c::new(0,5,5));
        assert!(bound_fbl.size == 5);

        // Test back top right
        let bound_fbl = Cube::child_bounds_for(cube.min_position, cube.size, 7);
        assert!(bound_fbl.min_position == V3c::new(5,5,5));
        assert!(bound_fbl.size == 5);
    }


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
}
