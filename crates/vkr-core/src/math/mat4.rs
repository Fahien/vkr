// Copyright © 2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{
    ops::{Index, IndexMut, Mul},
    simd::{f32x4, SimdFloat},
};

use crate::{Quat, Vec3};

#[derive(Default, PartialEq)]
/// Row-major 4x4 Matrix
pub struct Mat4 {
    values: [f32x4; 4],
}

impl From<[[f32; 4]; 4]> for Mat4 {
    fn from(values: [[f32; 4]; 4]) -> Self {
        Self::new([
            f32x4::from_array(values[0]),
            f32x4::from_array(values[1]),
            f32x4::from_array(values[2]),
            f32x4::from_array(values[3]),
        ])
    }
}

impl Mat4 {
    pub fn new(values: [f32x4; 4]) -> Self {
        Self { values }
    }

    pub fn from_translation(translation: &Vec3) -> Self {
        let mut ret = Mat4::identity();
        ret.translate(translation);
        ret
    }

    pub fn from_rotation(rotation: &Quat) -> Self {
        let mut ret = Mat4::identity();
        ret.rotate(rotation);
        ret
    }

    pub fn from_scale(scale: &Vec3) -> Self {
        let mut ret = Mat4::identity();
        ret.scale(scale);
        ret
    }

    pub fn identity() -> Self {
        Self {
            values: [
                f32x4::from_array([1.0, 0.0, 0.0, 0.0]),
                f32x4::from_array([0.0, 1.0, 0.0, 0.0]),
                f32x4::from_array([0.0, 0.0, 1.0, 0.0]),
                f32x4::from_array([0.0, 0.0, 0.0, 1.0]),
            ],
        }
    }

    pub fn look_at(target: Vec3, eye: Vec3, up: Vec3) -> Self {
        // Z axis points towards the eye!
        let z_axis = (eye - target).get_normalized();
        let x_axis = up.cross(&z_axis).get_normalized();
        let y_axis = z_axis.cross(&x_axis);

        Self {
            values: [
                x_axis.simd + f32x4::from_array([0.0, 0.0, 0.0, x_axis.dot(&-eye)]),
                y_axis.simd + f32x4::from_array([0.0, 0.0, 0.0, y_axis.dot(&-eye)]),
                z_axis.simd + f32x4::from_array([0.0, 0.0, 0.0, z_axis.dot(&-eye)]),
                f32x4::from_array([0.0, 0.0, 0.0, 1.0]),
            ],
        }
    }

    pub fn scale(&mut self, scale: &Vec3) {
        self[0][0] *= scale.get_x();
        self[1][1] *= scale.get_y();
        self[2][2] *= scale.get_z();
    }

    pub fn rotate(&mut self, q: &Quat) {
        *self = Mat4::from(q) * self as &Mat4;
    }

    pub fn translate(&mut self, translation: &Vec3) {
        self[0][3] += translation.get_x();
        self[1][3] += translation.get_y();
        self[2][3] += translation.get_z();
    }

    pub fn get_scale(&self) -> Vec3 {
        Vec3::new(self[0][0], self[1][1], self[2][2])
    }

    pub fn get_rotation(&self) -> Quat {
        Quat::from(self)
    }

    pub fn get_translation(&self) -> Vec3 {
        Vec3::new(self[0][3], self[1][3], self[2][3])
    }

    pub fn set_translation(&mut self, translation: &Vec3) {
        self[0][3] = translation.get_x();
        self[1][3] = translation.get_y();
        self[2][3] = translation.get_z();
    }

    pub fn get_transpose(&self) -> Self {
        let mut ret = Self::default();
        for i in 0..4 {
            for j in 0..4 {
                ret[i][j] = self[j][i]
            }
        }
        ret
    }
}

impl Index<usize> for Mat4 {
    type Output = f32x4;
    fn index(&self, i: usize) -> &Self::Output {
        &self.values[i]
    }
}

impl IndexMut<usize> for Mat4 {
    fn index_mut(&mut self, i: usize) -> &mut f32x4 {
        &mut self.values[i]
    }
}

impl Mul<&Mat4> for Mat4 {
    type Output = Mat4;

    fn mul(self, rhs: &Mat4) -> Self::Output {
        let mut ret = Mat4::default();

        for i in 0..4 {
            let row = &self.values[i];

            for j in 0..4 {
                let column = f32x4::from_array([
                    rhs.values[0][j],
                    rhs.values[1][j],
                    rhs.values[2][j],
                    rhs.values[3][j],
                ]);

                ret[i][j] = (row * column).reduce_sum();
            }
        }

        ret
    }
}

impl Mul<Mat4> for Mat4 {
    type Output = Mat4;

    fn mul(self, rhs: Mat4) -> Self::Output {
        self * &rhs
    }
}

impl From<&Quat> for Mat4 {
    fn from(q: &Quat) -> Self {
        let xq = f32x4::splat(q.simd[0]) * q.simd;
        let yq = f32x4::splat(q.simd[1]) * q.simd;
        let zq = f32x4::splat(q.simd[2]) * q.simd;

        Mat4::from([
            [
                1.0 - 2.0 * (yq[1] + zq[2]),
                2.0 * (xq[1] - zq[3]),
                2.0 * (xq[2] + yq[3]),
                0.0,
            ],
            [
                2.0 * (xq[1] + zq[3]),
                1.0 - 2.0 * (xq[0] + zq[2]),
                2.0 * (yq[2] - xq[3]),
                0.0,
            ],
            [
                2.0 * (xq[2] - yq[3]),
                2.0 * (yq[2] + xq[3]),
                1.0 - 2.0 * (xq[0] + yq[1]),
                0.0,
            ],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }
}

impl Mul<Vec3> for &Mat4 {
    type Output = Vec3;

    fn mul(self, rhs: Vec3) -> Self::Output {
        let mut ret = [0.0, 0.0, 0.0, 0.0];

        for i in 0..4 {
            let row = self[i];
            // Use last element
            let column = rhs.simd + f32x4::from_array([0.0, 0.0, 0.0, 1.0]);
            ret[i] = (row * column).reduce_sum();
        }

        Vec3::new(ret[0] / ret[3], ret[1] / ret[3], ret[2] / ret[3])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn mul() {
        // TODO: Further test multiplication
        let a = Mat4::identity();
        let mut b = Mat4::identity();
        b.scale(&Vec3::new(2.0, 2.0, 2.0));
        assert!(b.values[0][0] == 2.0);
        assert!(b.values[1][1] == 2.0);
        assert!(b.values[2][2] == 2.0);

        let c = a * b;
        assert!(c != Mat4::identity());
        assert!(c.values[0][0] == 2.0);
        assert!(c.values[1][1] == 2.0);
        assert!(c.values[2][2] == 2.0);
    }

    #[test]
    fn look_at() {
        let target = Vec3::default();
        let eye = Vec3::new(0.0, 0.0, 4.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let mat = Mat4::look_at(target, eye, up);
        assert_eq!(-mat.get_translation(), eye);
        assert_eq!(mat.get_rotation(), Quat::default());
    }
}
