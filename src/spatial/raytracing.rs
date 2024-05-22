#[cfg(feature = "raytracing")]
use crate::spatial::{math::vector::V3c, Cube, FLOAT_ERROR_TOLERANCE};

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
    /// Tells the intersection with the cube of the given ray.
    /// returns the distance from the origin to the direction of the ray until the hit point and the normal of the hit
    /// https://gamedev.stackexchange.com/questions/18436/most-efficient-aabb-vs-ray-collision-algorithms
    #[cfg(feature = "raytracing")]
    pub fn intersect_ray(&self, ray: &Ray) -> Option<CubeRayIntersection> {
        debug_assert!(ray.is_valid());

        let max_position = V3c::<f32>::from(self.min_position) + V3c::unit(self.size as f32);
        let t1 = (self.min_position.x as f32 - ray.origin.x) / ray.direction.x;
        let t2 = (max_position.x - ray.origin.x) / ray.direction.x;
        let t3 = (self.min_position.y as f32 - ray.origin.y) / ray.direction.y;
        let t4 = (max_position.y - ray.origin.y) / ray.direction.y;
        let t5 = (self.min_position.z as f32 - ray.origin.z) / ray.direction.z;
        let t6 = (max_position.z - ray.origin.z) / ray.direction.z;

        let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
        let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

        if tmax < 0. || tmin > tmax {
            // ray is intersecting the cube, but it is behind it
            // OR ray doesn't intersect cube
            return None;
        }

        let p = ray.point_at(tmin);
        let mut impact_normal = V3c::unit(0.);
        if (p.x - self.min_position.x as f32).abs() < FLOAT_ERROR_TOLERANCE {
            impact_normal.x = -1.;
        } else if (p.x - (self.min_position.x + self.size) as f32).abs() < FLOAT_ERROR_TOLERANCE {
            impact_normal.x = 1.;
        } else if (p.y - self.min_position.y as f32).abs() < FLOAT_ERROR_TOLERANCE {
            impact_normal.y = -1.;
        } else if (p.y - (self.min_position.y + self.size) as f32).abs() < FLOAT_ERROR_TOLERANCE {
            impact_normal.y = 1.;
        } else if (p.z - self.min_position.z as f32).abs() < FLOAT_ERROR_TOLERANCE {
            impact_normal.z = -1.;
        } else if (p.z - (self.min_position.z + self.size) as f32).abs() < FLOAT_ERROR_TOLERANCE {
            impact_normal.z = 1.;
        }

        if tmin < 0.0 {
            return Some(CubeRayIntersection {
                impact_distance: None,
                exit_distance: tmax,
                impact_normal,
            });
        }

        Some(CubeRayIntersection {
            impact_distance: Some(tmin),
            exit_distance: tmax,
            impact_normal,
        })
    }
}
