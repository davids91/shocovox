#[derive(Default, Debug)]
pub struct V3c<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

impl<T> V3c<T>
where
    T: Copy,
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

impl<T> Sub for &V3c<T>
where
    T: Copy + Sub<Output = T>,
{
    type Output = V3c<T>;

    fn sub(self, other: &V3c<T>) -> V3c<T> {
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
mod tests {
    use crate::spatial::hash_region;
    use crate::spatial::V3c;

    #[test]
    fn test_unit_v3ctor() {
        todo!()
    }

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
}
