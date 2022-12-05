// Copyright © 2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use core::simd::*;
use std::ops::{Add, AddAssign, Div, Index, Mul, MulAssign, Neg, Sub, SubAssign};

use crate::{Color, Quat};

#[derive(Clone, Copy)]
pub enum Axis3 {
    X,
    Y,
    Z,
}

const EPS: f32 = f32::EPSILON * 8192.0;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Vec3 {
    pub simd: f32x4,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Self {
            simd: f32x4::from_array([x, y, z, 0.0]),
        }
    }

    pub fn splat(d: f32) -> Vec3 {
        Self::new(d, d, d)
    }

    pub fn simd(simd: f32x4) -> Self {
        Self { simd }
    }

    pub fn get_x(&self) -> f32 {
        self.simd[0]
    }
    pub fn get_y(&self) -> f32 {
        self.simd[1]
    }
    pub fn get_z(&self) -> f32 {
        self.simd[2]
    }

    pub fn set_x(&mut self, x: f32) {
        self.simd[0] = x;
    }
    pub fn set_y(&mut self, y: f32) {
        self.simd[1] = y;
    }
    pub fn set_z(&mut self, z: f32) {
        self.simd[2] = z;
    }

    pub fn abs(mut self) -> Vec3 {
        self.simd = self.simd.abs();
        self
    }

    pub fn close(&self, b: &Vec3) -> bool {
        let diff = (self - b).abs();
        diff < Vec3::splat(EPS)
    }

    pub fn min(self, other: &Vec3) -> Vec3 {
        Self::simd(self.simd.simd_min(other.simd))
    }

    pub fn max(self, other: &Vec3) -> Vec3 {
        Self::simd(self.simd.simd_max(other.simd))
    }

    pub fn dot(&self, rhs: &Self) -> f32 {
        let mul4 = self.simd * rhs.simd;
        let mask = mask32x4::from_array([true, true, true, false]);
        let mul3 = mask.select(mul4, f32x4::default());
        mul3.reduce_sum()
    }

    /// [SIMD cross-product](https://geometrian.com/programming/tutorials/cross-product/index.php) method 5
    pub fn cross(&self, rhs: &Self) -> Self {
        let tmp0 = simd_swizzle!(self.simd, [1, 2, 0, 3]);
        let tmp1 = simd_swizzle!(rhs.simd, [2, 0, 1, 3]);
        let tmp2 = tmp0 * rhs.simd;
        let tmp3 = tmp0 * tmp1;
        let tmp4 = simd_swizzle!(tmp2, [1, 2, 0, 3]);
        Self::simd(tmp3 - tmp4)
    }

    pub fn scale(&mut self, scale: &Vec3) {
        let mask = mask32x4::from_array([true, true, true, false]);
        let scale = mask.select(scale.simd, f32x4::splat(1.0));
        self.simd *= scale;
    }

    pub fn rotate(&mut self, rotation: &Quat) {
        // Extract the vector part of the quaternion
        let u = Vec3::simd(rotation.simd * f32x4::from_array([1.0, 1.0, 1.0, 0.0]));

        let v = *self;

        // Extract the scalar part of the quaternion
        let s = rotation.get_w();

        // Do the math
        *self = 2.0 * u.dot(&v) * u + (s * s - u.dot(&u)) * v + 2.0 * s * u.cross(&v);
    }

    pub fn translate(&mut self, translation: &Vec3) {
        let mask = mask32x4::from_array([true, true, true, false]);
        let translation = mask.select(translation.simd, f32x4::splat(0.0));
        self.simd += translation;
    }

    pub fn norm(&self) -> f32 {
        self.dot(self)
    }

    pub fn len(&self) -> f32 {
        self.norm().sqrt()
    }

    pub fn normalize(&mut self) {
        let len = self.len();
        self.simd /= f32x4::splat(len);
    }

    pub fn get_normalized(mut self) -> Self {
        self.normalize();
        self
    }

    pub fn get_reciprocal(&self) -> Self {
        let one = f32x4::from_array([1.0, 1.0, 1.0, 0.0]);
        let mask = mask32x4::from_array([true, true, true, false]);
        let den = mask.select(self.simd, f32x4::splat(1.0));
        Self::simd(one / den)
    }

    /// Returns the reflection of this vector around a surface normal
    pub fn reflect(&self, normal: &Vec3) -> Self {
        self - 2.0 * self.dot(normal) * normal
    }
}

impl From<&Color> for Vec3 {
    fn from(c: &Color) -> Self {
        Vec3::new(c.r * c.a, c.g * c.a, c.b * c.a)
    }
}

impl From<Color> for Vec3 {
    fn from(c: Color) -> Self {
        Vec3::new(c.r * c.a, c.g * c.a, c.b * c.a)
    }
}

impl Sub for Vec3 {
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self::Output {
        self.simd -= rhs.simd;
        self
    }
}

impl Sub for &Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: &Vec3) -> Self::Output {
        *self - rhs
    }
}

impl Sub<Vec3> for &Vec3 {
    type Output = Vec3;

    fn sub(self, mut rhs: Vec3) -> Self::Output {
        rhs.simd = self.simd - rhs.simd;
        rhs
    }
}

impl Sub<&Vec3> for Vec3 {
    type Output = Vec3;

    fn sub(mut self, rhs: &Vec3) -> Self::Output {
        self.simd -= rhs.simd;
        self
    }
}

impl Sub<f32> for Vec3 {
    type Output = Vec3;

    fn sub(mut self, rhs: f32) -> Self::Output {
        self.simd -= f32x4::splat(rhs);
        self
    }
}

impl SubAssign<&Vec3> for Vec3 {
    fn sub_assign(&mut self, rhs: &Vec3) {
        self.simd -= rhs.simd;
    }
}

impl Default for Vec3 {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }
}

impl Add for Vec3 {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.simd += rhs.simd;
        self
    }
}

impl Add<Vec3> for f32 {
    type Output = Vec3;

    fn add(self, mut rhs: Vec3) -> Self::Output {
        rhs += self;
        rhs
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        self.simd += rhs.simd;
    }
}

impl AddAssign<&Vec3> for Vec3 {
    fn add_assign(&mut self, rhs: &Self) {
        self.simd += rhs.simd;
    }
}

impl AddAssign<f32> for Vec3 {
    fn add_assign(&mut self, rhs: f32) {
        self.simd += f32x4::splat(rhs)
    }
}

impl Mul for Vec3 {
    type Output = Self;

    fn mul(mut self, rhs: Self) -> Self::Output {
        self.simd *= rhs.simd;
        self
    }
}

impl Mul<Vec3> for &Vec3 {
    type Output = Vec3;

    fn mul(self, mut rhs: Vec3) -> Self::Output {
        rhs.simd *= self.simd;
        rhs
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;

    fn mul(mut self, rhs: f32) -> Self::Output {
        self.simd *= f32x4::splat(rhs);
        self
    }
}
impl Mul<f32> for &Vec3 {
    type Output = Vec3;

    fn mul(self, rhs: f32) -> Self::Output {
        self * Vec3::splat(rhs)
    }
}

impl Mul<Vec3> for f32 {
    type Output = Vec3;

    fn mul(self, mut rhs: Vec3) -> Self::Output {
        rhs.simd *= f32x4::splat(self);
        rhs
    }
}

impl Mul<&Vec3> for f32 {
    type Output = Vec3;

    fn mul(self, rhs: &Vec3) -> Self::Output {
        *rhs * self
    }
}

impl MulAssign<f32> for Vec3 {
    fn mul_assign(&mut self, rhs: f32) {
        self.simd *= f32x4::splat(rhs);
    }
}

impl Div<f32> for Vec3 {
    type Output = Vec3;

    fn div(mut self, rhs: f32) -> Self::Output {
        self.simd /= f32x4::splat(rhs);
        self
    }
}

impl Div<Vec3> for f32 {
    type Output = Vec3;

    fn div(self, mut rhs: Vec3) -> Self::Output {
        rhs.simd = f32x4::splat(self) / rhs.simd;
        rhs
    }
}

impl Neg for Vec3 {
    type Output = Vec3;

    fn neg(mut self) -> Self::Output {
        self.simd = -self.simd;
        self
    }
}

impl Neg for &Vec3 {
    type Output = Vec3;

    fn neg(self) -> Self::Output {
        -*self
    }
}

impl Index<Axis3> for Vec3 {
    type Output = f32;

    fn index(&self, index: Axis3) -> &Self::Output {
        match index {
            Axis3::X => &self.simd[0],
            Axis3::Y => &self.simd[1],
            Axis3::Z => &self.simd[2],
        }
    }
}

#[cfg(test)]
mod test {
    mod vec3 {
        use crate::*;

        #[test]
        fn normalize() {
            let mut v = Vec3::new(2.0, 0.0, 0.0);
            v.normalize();
            assert!(v.close(&Vec3::new(1.0, 0.0, 0.0)));
        }

        #[test]
        fn rotate() {
            let mut v = Vec3::new(1.0, 0.0, 0.0);
            let y180 = Quat::new(0.0, 1.0, 0.0, 0.0);
            v.rotate(&y180);
            assert!(v.close(&Vec3::new(-1.0, 0.0, 0.0)));

            let mut v = Vec3::new(1.0, 0.0, 0.0);
            let y90 = Quat::new(0.0, 0.707, 0.0, 0.707);
            v.rotate(&y90);
            assert!(v.close(&Vec3::new(0.0, 0.0, -1.0)));

            let mut v = Vec3::new(1.0, 0.0, 0.0);
            let z180 = Quat::new(0.0, 0.0, 1.0, 0.0);
            v.rotate(&z180);
            assert!(v.close(&Vec3::new(-1.0, 0.0, 0.0)));

            let mut v = Vec3::new(1.0, 0.0, 0.0);
            let z90 = Quat::new(0.0, 0.0, 0.707, 0.707);
            v.rotate(&z90);
            assert!(v.close(&Vec3::new(0.0, 1.0, 0.0)));

            let mut v = Vec3::new(0.0, 0.0, 1.0);
            // x: -45 degrees
            let rot = Quat::new(-0.383, 0.0, 0.0, 0.924);
            v.rotate(&rot);
            assert!(v.close(&Vec3::new(0.0, 0.707, 0.707)));
        }
    }
}
