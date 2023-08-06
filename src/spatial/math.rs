///####################################################################################
/// V3c
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
pub fn hash_region(offset: &V3c<f32>, size: f32) -> usize {
    let midpoint = V3c::unit(size / 2.);
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

/// calculates the distance between the line, and the plane both described by a ray
/// plane: normal, and a point on plane, line: origin and direction
/// return the distance from the line origin to the direction of it, if they have an intersection
pub fn plane_line_intersection_distance(
    plane_point: &V3c<f32>,
    plane_normal: &V3c<f32>,
    line_origin: &V3c<f32>,
    line_direction: &V3c<f32>,
) -> Option<f32> {
    let origins_diff = *plane_point - *line_origin;
    let plane_line_dot_to_plane = origins_diff.dot(&plane_normal);
    let directions_dot = line_direction.dot(&plane_normal);
    if 0. == directions_dot {
        // line and plane is paralell
        if 0. == origins_diff.dot(&plane_normal) {
            // The distance is zero because the origin is already on the plane
            return Some(0.);
        }
        return None;
    }
    Some(plane_line_dot_to_plane / directions_dot)
}
