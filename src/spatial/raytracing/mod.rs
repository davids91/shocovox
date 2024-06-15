use crate::spatial::{math::vector::V3c, Cube};

pub mod lut;

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

#[derive(Debug, Copy, Clone, Default)]
pub struct CubeRayIntersection {
    pub(crate) impact_distance: Option<f32>,
    pub(crate) exit_distance: f32,
}

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

        if tmin < 0.0 {
            return Some(CubeRayIntersection {
                impact_distance: None,
                exit_distance: tmax,
            });
        }

        Some(CubeRayIntersection {
            impact_distance: Some(tmin),
            exit_distance: tmax,
        })
    }
}
