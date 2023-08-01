///####################################################################################
/// V3C
///####################################################################################
#[derive(Default, Clone, Copy, Debug)]
pub struct V3c<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

impl<T> V3c<T>
where
    T: Copy + Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Div<Output = T>,
{
    pub fn new(x: T, y: T, z: T) -> Self {
        Self { x, y, z }
    }
    pub fn unit(scale: T) -> Self {
        Self {
            x: scale,
            y: scale,
            z: scale,
        }
    }
}

impl V3c<f32> {
    pub fn length(&self) -> f32 {
        (self.x.powf(2.0) + self.y.powf(2.0) + self.z.powf(2.0)).sqrt()
    }

    pub fn normalized(self) -> V3c<f32> {
        self / self.length()
    }

    pub fn dot(&self, other: &V3c<f32>) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: V3c<f32>) -> V3c<f32> {
        V3c {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }
}

use std::ops::{Add, Div, Mul, Sub};
impl<T: Add<Output = T>> Add for V3c<T> {
    type Output = V3c<T>;

    fn add(self, other: V3c<T>) -> V3c<T> {
        V3c {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl<T> Sub for V3c<T>
where
    T: Copy + Sub<Output = T>,
{
    type Output = V3c<T>;

    fn sub(self, other: V3c<T>) -> V3c<T> {
        V3c {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl<T: Mul<Output = T> + Copy> Mul<T> for V3c<T> {
    type Output = V3c<T>;

    fn mul(self, scalar: T) -> V3c<T> {
        V3c {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

impl<T: Div<Output = T> + Copy> Div<T> for V3c<T> {
    type Output = V3c<T>;

    fn div(self, scalar: T) -> V3c<T> {
        V3c {
            x: self.x / scalar,
            y: self.y / scalar,
            z: self.z / scalar,
        }
    }
}

impl<T> PartialEq for V3c<T>
where
    T: Default + Add<Output = T> + Mul<Output = T> + Copy + PartialEq + PartialEq + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z
    }
}
impl<T> Eq for V3c<T> where T: Default + Add<Output = T> + Mul<Output = T> + Copy + PartialEq {}

impl From<V3c<u32>> for V3c<f32> {
    fn from(vec: V3c<u32>) -> V3c<f32> {
        {
            V3c::new(vec.x as f32, vec.y as f32, vec.z as f32)
        }
    }
}

impl From<V3c<f32>> for V3c<u32> {
    fn from(vec: V3c<f32>) -> V3c<u32> {
        {
            V3c::new(
                vec.x.round() as u32,
                vec.y.round() as u32,
                vec.z.round() as u32,
            )
        }
    }
}

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

///####################################################################################
/// Raytracing stuff
///####################################################################################
#[derive(Debug)]
pub struct Ray {
    pub origin: V3c<f32>,
    pub direction: V3c<f32>,
}

impl Ray {
    pub fn is_valid(&self) -> bool {
        (1. - self.direction.length()).abs() < 0.000001
    }
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

#[derive(Default, Debug)]
pub struct Cube {
    pub min_position: V3c<u32>,
    pub size: u32,
}

impl Cube {
    pub fn face(&self, face: CubeFaces) -> Ray {
        let midpoint: V3c<f32> = V3c::unit(self.size as f32 / 2.) + self.min_position.into();
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
    pub fn intersect_ray(&self, ray: &Ray) -> Option<(f32, V3c<f32>)> {
        assert!(ray.is_valid());
        let mut faces_hit = 0;
        let mut min_distance = f32::MAX;
        let mut normal = V3c::default();
        let mut intersect_fn = |face| -> u32 {
            //returns with 1 if the given face is hit, otherwise zero
            let face = &self.face(face);
            if let Some(d) = plane_line_intersection_distance(face, ray) {
                // d hits the plane
                if 0. < d && self.contains_point(&(ray.origin + (ray.direction * d))) {
                    if d < min_distance {
                        min_distance = d;
                        normal = face.direction;
                    }
                    1
                } else {
                    0
                }
            } else {
                0
            }
        };

        faces_hit += intersect_fn(CubeFaces::FRONT);
        faces_hit += intersect_fn(CubeFaces::LEFT);
        if 2 == faces_hit {
            return Some((min_distance, normal));
        }
        faces_hit += intersect_fn(CubeFaces::REAR);
        if 2 == faces_hit {
            return Some((min_distance, normal));
        }
        faces_hit += intersect_fn(CubeFaces::RIGHT);
        if 2 == faces_hit {
            return Some((min_distance, normal));
        }
        faces_hit += intersect_fn(CubeFaces::TOP);
        if 2 == faces_hit {
            return Some((min_distance, normal));
        }
        faces_hit += intersect_fn(CubeFaces::BOTTOM);

        assert!(faces_hit <= 2);
        if 0 < faces_hit {
            return Some((min_distance, normal));
        } else {
            None
        }
    }

    pub fn contains_ray(&self, ray: &Ray) -> bool {
        self.intersect_ray(ray).is_some()
    }

    pub fn contains_point(&self, point: &V3c<f32>) -> bool {
        (point.x >= self.min_position.x as f32)
            && (point.x <= (self.min_position.x + self.size) as f32)
            && (point.y >= self.min_position.y as f32)
            && (point.y <= (self.min_position.y + self.size) as f32)
            && (point.z >= self.min_position.z as f32)
            && (point.z <= (self.min_position.z + self.size) as f32)
    }
}

/// calculates the distance between the line, and the plane both described by a ray
/// plane: normal, and a point on plane, line: origin and direction
/// return the distance from the line origin to the direction of it, if they have an intersection
pub fn plane_line_intersection_distance(plane: &Ray, line: &Ray) -> Option<f32> {
    let origins_diff = plane.origin - line.origin;
    let plane_line_dot_to_plane = origins_diff.dot(&plane.direction);
    let directions_dot = line.direction.dot(&plane.direction);
    if 0. == directions_dot {
        // line and plane is paralell
        if 0. == origins_diff.dot(&plane.direction) {
            // The distance is zero because the origin is already on the plane
            return Some(0.);
        }
        return None;
    }
    Some(plane_line_dot_to_plane / directions_dot)
}

#[cfg(test)]
mod raytracing_tests {
    use crate::spatial::plane_line_intersection_distance;

    use super::Cube;
    use super::Ray;
    use super::V3c;

    #[test]
    fn test_plane_line_intersection() {
        assert!(
            plane_line_intersection_distance(
                &Ray {
                    // plane
                    origin: V3c::new(0., 0., 0.),
                    direction: V3c::new(0., 1., 0.)
                },
                &Ray {
                    // line
                    origin: V3c::new(0., 1., 0.),
                    direction: V3c::new(1., 0., 0.)
                }
            ) == None
        );

        assert!(
            plane_line_intersection_distance(
                &Ray {
                    // plane
                    origin: V3c::new(0., 0., 0.),
                    direction: V3c::new(0., 1., 0.)
                },
                &Ray {
                    // line
                    origin: V3c::new(0., 1., 0.),
                    direction: V3c::new(0., -1., 0.)
                }
            ) == Some(1.)
        );

        assert!(
            plane_line_intersection_distance(
                &Ray {
                    // plane
                    origin: V3c::new(0., 0., 0.),
                    direction: V3c::new(0., 1., 0.)
                },
                &Ray {
                    // line
                    origin: V3c::new(0., 0., 0.),
                    direction: V3c::new(1., 0., 0.)
                }
            ) == Some(0.)
        );
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
                x: 4.001,
                y: 4.001,
                z: 4.001,
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

///####################################################################################
/// Octant
///####################################################################################
pub(crate) fn offset_region(octant: usize) -> V3c<u32> {
    match octant {
        0 => V3c::new(0, 0, 0),
        1 => V3c::new(1, 0, 0),
        2 => V3c::new(0, 0, 1),
        3 => V3c::new(1, 0, 1),
        4 => V3c::new(0, 1, 0),
        5 => V3c::new(1, 1, 0),
        6 => V3c::new(0, 1, 1),
        7 => V3c::new(1, 1, 1),
        _ => panic!("Invalid region hash provided for spatial reference!"),
    }
}

/// Each Node is separated to 8 Octants based on their relative position inside the Nodes occupying space.
/// The hash function assigns an index for each octant, so every child Node can be indexed in a well defined manner
pub fn hash_region(offset: &V3c<u32>, size: u32) -> usize {
    let midpoint = V3c::unit(size / 2);

    // The below is rewritten to be branchless
    // (if offset.x < midpoint.x { 0 } else { 1 })
    //     + if offset.z < midpoint.z { 0 } else { 2 }
    //     + if offset.y < midpoint.y { 0 } else { 4 }
    (offset.x >= midpoint.x) as usize
        + (offset.z >= midpoint.z) as usize * 2
        + (offset.y >= midpoint.y) as usize * 4
}

#[cfg(test)]
mod octant_tests {
    use crate::spatial::hash_region;
    use crate::spatial::offset_region;
    use crate::spatial::V3c;

    #[test]
    fn test_hash_region() {
        assert!(hash_region(&V3c::new(0, 0, 0), 10) == 0);
        assert!(hash_region(&V3c::new(6, 0, 0), 10) == 1);
        assert!(hash_region(&V3c::new(0, 0, 6), 10) == 2);
        assert!(hash_region(&V3c::new(6, 0, 6), 10) == 3);
        assert!(hash_region(&V3c::new(0, 6, 0), 10) == 4);
        assert!(hash_region(&V3c::new(6, 6, 0), 10) == 5);
        assert!(hash_region(&V3c::new(0, 6, 6), 10) == 6);
        assert!(hash_region(&V3c::new(6, 6, 6), 10) == 7);
    }

    #[test]
    fn test_offset_region() {
        assert!(V3c::new(0, 0, 0) == offset_region(0));
        assert!(V3c::new(1, 0, 0) == offset_region(1));
        assert!(V3c::new(0, 0, 1) == offset_region(2));
        assert!(V3c::new(1, 0, 1) == offset_region(3));
        assert!(V3c::new(0, 1, 0) == offset_region(4));
        assert!(V3c::new(1, 1, 0) == offset_region(5));
        assert!(V3c::new(0, 1, 1) == offset_region(6));
        assert!(V3c::new(1, 1, 1) == offset_region(7));
    }
}
