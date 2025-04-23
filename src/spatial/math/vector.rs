use std::ops::{Add, AddAssign, Div, Mul, Rem, Sub, SubAssign};

#[derive(Default, Clone, Copy, Debug, Eq, PartialEq, PartialOrd)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
#[repr(C)]
pub struct V3c<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

pub type V3cf32 = V3c<f32>;

impl<T: Copy> V3c<T> {
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

impl<T: Copy + Rem<N, Output = T>, N: Copy> Rem<N> for V3c<T> {
    type Output = V3c<T>;
    fn rem(self, rem: N) -> V3c<T> {
        V3c::new(self.x % rem, self.y % rem, self.z % rem)
    }
}

impl<T> SubAssign for V3c<T>
where
    T: Copy + Sub<Output = T>,
{
    fn sub_assign(&mut self, other: V3c<T>) {
        *self = *self - other;
    }
}

impl<T> AddAssign for V3c<T>
where
    T: Copy + Add<Output = T>,
{
    fn add_assign(&mut self, other: V3c<T>) {
        *self = *self + other;
    }
}

impl<T> V3c<T>
where
    T: num_traits::Signed + Clone,
{
    pub fn abs(&mut self) -> &mut Self {
        self.x = self.x.abs();
        self.y = self.y.abs();
        self.z = self.z.abs();
        self
    }

    pub fn modulo(&mut self, operand: &T) -> &mut Self {
        self.x = self.x.clone() % operand.clone();
        self.y = self.y.clone() % operand.clone();
        self.z = self.z.clone() % operand.clone();
        self
    }
}

impl V3c<f32> {
    pub fn length(&self) -> f32 {
        ((self.x * self.x) + (self.y * self.y) + (self.z * self.z)).sqrt()
    }

    pub fn normalized(self) -> V3c<f32> {
        self / self.length()
    }

    pub fn signum(&self) -> V3c<f32> {
        V3c {
            x: self.x.signum(),
            y: self.y.signum(),
            z: self.z.signum(),
        }
    }

    pub fn floor(&self) -> V3c<f32> {
        V3c {
            x: self.x.floor(),
            y: self.y.floor(),
            z: self.z.floor(),
        }
    }

    pub fn ceil(&self) -> V3c<f32> {
        V3c {
            x: self.x.ceil(),
            y: self.y.ceil(),
            z: self.z.ceil(),
        }
    }

    pub fn round(&mut self) -> Self {
        self.x = self.x.round();
        self.y = self.y.round();
        self.z = self.z.round();
        *self
    }

    pub fn cut_each_component(mut self, value: f32) -> Self {
        self.x = self.x.min(value);
        self.y = self.y.min(value);
        self.z = self.z.min(value);
        self
    }
}

impl V3c<i32> {
    pub fn length(&self) -> f32 {
        (((self.x * self.x) + (self.y * self.y) + (self.z * self.z)) as f32).sqrt()
    }
    pub fn sign(&self) -> V3c<i32> {
        V3c::new(self.x.signum(), self.y.signum(), self.z.signum())
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

    pub fn cut_each_component(&mut self, value: &u32) -> Self {
        self.x = self.x.min(*value);
        self.y = self.y.min(*value);
        self.z = self.z.min(*value);
        *self
    }

    pub fn cut_by(&mut self, value: V3c<u32>) -> Self {
        self.x = self.x.min(value.x);
        self.y = self.y.min(value.y);
        self.z = self.z.min(value.z);
        *self
    }
}

impl V3c<usize> {
    pub fn length(&self) -> f32 {
        (((self.x * self.x) + (self.y * self.y) + (self.z * self.z)) as f32).sqrt()
    }

    pub fn normalized(self) -> V3c<f32> {
        let result: V3c<f32> = self.into();
        result / self.length()
    }

    pub fn cut_each_component(mut self, value: usize) -> Self {
        self.x = self.x.min(value);
        self.y = self.y.min(value);
        self.z = self.z.min(value);
        self
    }

    pub fn cut_by(&mut self, value: V3c<usize>) -> Self {
        self.x = self.x.min(value.x);
        self.y = self.y.min(value.y);
        self.z = self.z.min(value.z);
        *self
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

impl<T: Mul<Output = T> + Copy> Mul<V3c<T>> for V3c<T> {
    type Output = V3c<T>;

    fn mul(self, other: V3c<T>) -> V3c<T> {
        V3c {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
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

impl From<V3c<usize>> for V3c<f32> {
    fn from(vec: V3c<usize>) -> V3c<f32> {
        {
            V3c::new(vec.x as f32, vec.y as f32, vec.z as f32)
        }
    }
}

impl From<V3c<i32>> for V3c<f32> {
    fn from(vec: V3c<i32>) -> V3c<f32> {
        {
            V3c::new(vec.x as f32, vec.y as f32, vec.z as f32)
        }
    }
}

impl From<V3c<u32>> for V3c<f32> {
    fn from(vec: V3c<u32>) -> V3c<f32> {
        {
            V3c::new(vec.x as f32, vec.y as f32, vec.z as f32)
        }
    }
}

impl From<[f32; 3]> for V3c<f32> {
    fn from(vec: [f32; 3]) -> V3c<f32> {
        {
            V3c::new(vec[0], vec[1], vec[2])
        }
    }
}

impl From<V3c<u32>> for V3c<usize> {
    fn from(vec: V3c<u32>) -> V3c<usize> {
        {
            V3c::new(vec.x as usize, vec.y as usize, vec.z as usize)
        }
    }
}

impl From<V3c<i32>> for V3c<usize> {
    fn from(vec: V3c<i32>) -> V3c<usize> {
        {
            V3c::new(vec.x as usize, vec.y as usize, vec.z as usize)
        }
    }
}

impl From<V3c<f32>> for V3c<usize> {
    fn from(vec: V3c<f32>) -> V3c<usize> {
        {
            V3c::new(
                vec.x.round() as usize,
                vec.y.round() as usize,
                vec.z.round() as usize,
            )
        }
    }
}

impl From<V3c<usize>> for V3c<u32> {
    fn from(vec: V3c<usize>) -> V3c<u32> {
        {
            V3c::new(vec.x as u32, vec.y as u32, vec.z as u32)
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

impl From<V3c<i32>> for V3c<u32> {
    fn from(vec: V3c<i32>) -> V3c<u32> {
        {
            V3c::new(vec.x as u32, vec.y as u32, vec.z as u32)
        }
    }
}

impl From<V3c<u8>> for V3c<u32> {
    fn from(vec: V3c<u8>) -> V3c<u32> {
        {
            V3c::new(vec.x as u32, vec.y as u32, vec.z as u32)
        }
    }
}

impl From<V3c<f32>> for V3c<i32> {
    fn from(vec: V3c<f32>) -> V3c<i32> {
        {
            V3c::new(
                vec.x.round() as i32,
                vec.y.round() as i32,
                vec.z.round() as i32,
            )
        }
    }
}

impl From<Vec<i32>> for V3c<i32> {
    fn from(vec: Vec<i32>) -> V3c<i32> {
        {
            V3c::new(vec[0], vec[1], vec[2])
        }
    }
}

impl From<V3c<u32>> for V3c<i32> {
    fn from(vec: V3c<u32>) -> V3c<i32> {
        {
            V3c::new(vec.x as i32, vec.y as i32, vec.z as i32)
        }
    }
}

#[cfg(feature = "bevy_wgpu")]
use bevy::render::render_resource::encase::{
    impl_vector, vector::AsMutVectorParts, vector::AsRefVectorParts,
};

#[cfg(feature = "bevy_wgpu")]
impl_vector!(3, V3cf32, f32; using From);

#[cfg(feature = "bevy_wgpu")]
impl AsRefVectorParts<f32, 3> for V3cf32 {
    fn as_ref_parts(&self) -> &[f32; 3] {
        unsafe { &*(self as *const V3cf32 as *const [f32; 3]) }
    }
}

#[cfg(feature = "bevy_wgpu")]
impl AsMutVectorParts<f32, 3> for V3cf32 {
    fn as_mut_parts(&mut self) -> &mut [f32; 3] {
        unsafe { &mut *(self as *mut V3cf32 as *mut [f32; 3]) }
    }
}
