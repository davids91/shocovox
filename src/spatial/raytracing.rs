#[cfg(feature = "raytracing")]
use crate::spatial::{
    math::{plane_line_intersection, vector::V3c},
    Cube,
};

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
pub struct CubeRayIntersection {
    pub(crate) impact_distance: Option<f32>,
    pub(crate) exit_distance: f32,
    pub(crate) impact_normal: V3c<f32>,
}

#[cfg(feature = "raytracing")]
impl Cube {
    pub fn face(&self, face: CubeFaces) -> Ray {
        let midpoint = self.midpoint();
        let direction = match face {
            CubeFaces::FRONT => V3c::new(0., 0., -1.),
            CubeFaces::LEFT => V3c::new(-1., 0., 0.),
            CubeFaces::REAR => V3c::new(0., 0., 1.),
            CubeFaces::RIGHT => V3c::new(1., 0., 0.),
            CubeFaces::TOP => V3c::new(0., 1., 0.),
            CubeFaces::BOTTOM => V3c::new(0., -1., 0.),
        };
        Ray {
            origin: midpoint + direction * (self.size as f32 / 2.),
            direction,
        }
    }

    /// Tells the intersection with the cube of the given ray.
    /// returns the distance from the origin to the direction of the ray until the hit point and the normal of the hit
    #[cfg(feature = "raytracing")]
    pub fn intersect_ray(&self, ray: &Ray) -> Option<CubeRayIntersection> {
        assert!(ray.is_valid());
        let mut distances: Vec<f32> = Vec::new();
        let mut impact_normal = V3c::default();

        if self.contains_point(&ray.origin) {
            distances.push(0.);
        }

        for f in CubeFaces::into_iter() {
            let face = &self.face(f);
            if let Some(d) =
                plane_line_intersection(&face.origin, &face.direction, &ray.origin, &ray.direction)
            {
                if 0. <= d && self.contains_point(&ray.point_at(d)) {
                    // ray hits the plane only when the resulting distance is at least positive,
                    // and the point is contained inside the cube
                    if 1 < distances.len()
                        && ((distances[0] - distances[1]).abs()
                            < crate::spatial::FLOAT_ERROR_TOLERANCE
                            || (d < distances[0] - crate::spatial::FLOAT_ERROR_TOLERANCE
                                && d < distances[1] - crate::spatial::FLOAT_ERROR_TOLERANCE))
                    {
                        // the first 2 hits were of an edge or the corner of the cube, so one of them can be discarded
                        distances[1] = d;
                    } else if distances.len() < 2 {
                        // not enough hits are gathered yet
                        distances.push(d);
                    } else {
                        // enough hits are gathered, exit the loop
                        break;
                    }
                    if distances.is_empty() || d <= distances[0] {
                        impact_normal = face.direction;
                    }
                }
            }
        }
        if 1 < distances.len() {
            Some(CubeRayIntersection {
                impact_distance: Some(distances[0].min(distances[1])),
                exit_distance: distances[0].max(distances[1]),
                impact_normal,
            })
        } else if !distances.is_empty() {
            Some(CubeRayIntersection {
                impact_distance: None,
                exit_distance: distances[0],
                impact_normal,
            })
        } else {
            None
        }
    }
}
