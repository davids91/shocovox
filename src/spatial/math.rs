///####################################################################################
/// V3c
///####################################################################################
#[derive(Default, Clone, Copy, Debug)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
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
        ((self.x * self.x) + (self.y * self.y) + (self.z * self.z)).sqrt()
    }
    pub fn normalized(self) -> V3c<f32> {
        self / self.length()
    }
}

impl V3c<u32> {
    pub fn length(&self) -> f32 {
        (((self.x * self.x) + (self.y * self.y) + (self.z * self.z)) as f32).sqrt()
    }
    pub fn normalized(self) -> V3c<f32> {
        let result: V3c<f32> = self.into();
        result / self.length()
    }
}

impl<T> V3c<T>
where
    T: std::ops::Mul<Output = T>
        + std::ops::Div<Output = T>
        + std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + std::marker::Copy,
{
    pub fn dot(&self, other: &V3c<T>) -> T {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: V3c<T>) -> V3c<T> {
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

///####################################################################################
/// Octant
///####################################################################################
pub(crate) fn offset_region(octant: u32) -> V3c<u32> {
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
pub fn hash_region(offset: &V3c<f32>, size: f32) -> u32 {
    let midpoint = V3c::unit(size / 2.);
    // The below is rewritten to be branchless
    // (if offset.x < midpoint.x { 0 } else { 1 })
    //     + if offset.z < midpoint.z { 0 } else { 2 }
    //     + if offset.y < midpoint.y { 0 } else { 4 }
    (offset.x >= midpoint.x) as u32
        + (offset.z >= midpoint.z) as u32 * 2
        + (offset.y >= midpoint.y) as u32 * 4
}

#[cfg(feature = "raytracing")]
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