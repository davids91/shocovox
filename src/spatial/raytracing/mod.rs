use crate::spatial::{lut::SECTANT_STEP_RESULT_LUT, math::vector::V3c, Cube};

mod tests;

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
}

impl Cube {
    /// Tells the intersection with the cube of the given ray.
    /// returns the distance from the origin to the direction of the ray until the hit point and the normal of the hit
    /// https://gamedev.stackexchange.com/questions/18436/most-efficient-aabb-vs-ray-collision-algorithms
    pub fn intersect_ray(&self, ray: &Ray) -> Option<CubeRayIntersection> {
        debug_assert!(ray.is_valid());

        let max_position = self.min_position + V3c::unit(self.size);
        let t1 = (self.min_position.x - ray.origin.x) / ray.direction.x;
        let t2 = (max_position.x - ray.origin.x) / ray.direction.x;
        let t3 = (self.min_position.y - ray.origin.y) / ray.direction.y;
        let t4 = (max_position.y - ray.origin.y) / ray.direction.y;
        let t5 = (self.min_position.z - ray.origin.z) / ray.direction.z;
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
            });
        }

        Some(CubeRayIntersection {
            impact_distance: Some(tmin),
        })
    }
}

/// Provides the resulting sectant based on the given sectant
/// It returns with OOB_SECTANT if the result is out of bounds.
/// Important note: the specs of `signum` behvaes differently for f32 and i32
/// So the conversion to i32 is absolutely required
pub(crate) fn step_sectant(sectant: u8, step: V3c<f32>) -> u8 {
    SECTANT_STEP_RESULT_LUT[sectant as usize][((step.x as i32).signum() + 1) as usize]
        [((step.y as i32).signum() + 1) as usize][((step.z as i32).signum() + 1) as usize]
}

/// calculates the distance between the line, and the plane both described by a ray
/// plane: normal, and a point on plane, line: origin and direction
/// returns the distance from the line origin to the direction of it, if they have an intersection
#[allow(dead_code)] // Could be useful either for debugging or new implementations
pub fn plane_line_intersection(
    plane_point: &V3c<f32>,
    plane_normal: &V3c<f32>,
    line_origin: &V3c<f32>,
    line_direction: &V3c<f32>,
) -> Option<f32> {
    let origins_diff = *plane_point - *line_origin;
    let plane_line_dot_to_plane = origins_diff.dot(plane_normal);
    let directions_dot = line_direction.dot(plane_normal);
    if 0. == directions_dot {
        // line and plane is paralell
        if 0. == origins_diff.dot(plane_normal) {
            // The distance is zero because the origin is already on the plane
            return Some(0.);
        }
        return None;
    }
    Some(plane_line_dot_to_plane / directions_dot)
}

pub fn cube_impact_normal(cube: &Cube, impact_point: &V3c<f32>) -> V3c<f32> {
    let mid_to_impact = cube.min_position + V3c::unit(cube.size / 2.) - *impact_point;
    let max_component = mid_to_impact
        .x
        .abs()
        .max(mid_to_impact.y.abs())
        .max(mid_to_impact.z.abs());

    let impact_normal = V3c::new(
        if mid_to_impact.x.abs() == max_component {
            -mid_to_impact.x
        } else {
            0.
        },
        if mid_to_impact.y.abs() == max_component {
            -mid_to_impact.y
        } else {
            0.
        },
        if mid_to_impact.z.abs() == max_component {
            -mid_to_impact.z
        } else {
            0.
        },
    );

    debug_assert!(0. < impact_normal.length());
    impact_normal.normalized()
}
