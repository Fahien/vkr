// Copyright © 2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{
    ops::{Index, IndexMut, Mul},
    simd::f32x4,
};

use super::*;

#[derive(Default, PartialEq)]
pub struct Mat3 {
    /// Row-major
    values: [[f32; 3]; 3],
}

impl From<[[f32; 3]; 3]> for Mat3 {
    fn from(values: [[f32; 3]; 3]) -> Self {
        let mut ret = Self::new();
        ret.values = values;
        ret
    }
}

impl Mat3 {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn from_rotation(rotation: &Quat) -> Self {
        let mut ret = Mat3::identity();
        ret.rotate(rotation);
        ret
    }

    pub fn from_scale(scale: &Vec3) -> Self {
        let mut ret = Mat3::identity();
        ret.scale(scale);
        ret
    }

    pub fn identity() -> Self {
        Self {
            values: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    /// Tangent, bitangent, normal
    pub fn tbn(t: &Vec3, b: &Vec3, n: &Vec3) -> Self {
        Self {
            values: [
                [t.simd[0], b.simd[0], n.simd[0]],
                [t.simd[1], b.simd[1], n.simd[1]],
                [t.simd[2], b.simd[2], n.simd[2]],
            ],
        }
    }

    pub fn scale(&mut self, scale: &Vec3) {
        self[0][0] *= scale.get_x();
        self[1][1] *= scale.get_y();
        self[2][2] *= scale.get_z();
    }

    pub fn rotate(&mut self, q: &Quat) {
        *self = Mat3::from(q) * self as &Mat3;
    }

    pub fn get_scale(&self) -> Vec3 {
        Vec3::new(self[0][0], self[1][1], self[2][2])
    }

    pub fn get_rotation(&self) -> Quat {
        Quat::from(self)
    }

    pub fn get_transpose(&self) -> Self {
        let mut ret = Self::new();
        for i in 0..3 {
            for j in 0..3 {
                ret[i][j] = self[j][i]
            }
        }
        ret
    }
}

impl From<&Mat4> for Mat3 {
    fn from(mat4: &Mat4) -> Self {
        let mut ret = Self::default();
        for i in 0..3 {
            for j in 0..3 {
                ret[i][j] = mat4[i][j]
            }
        }
        ret
    }
}

impl Index<usize> for Mat3 {
    type Output = [f32; 3];
    fn index(&self, i: usize) -> &Self::Output {
        &self.values[i]
    }
}

impl IndexMut<usize> for Mat3 {
    fn index_mut(&mut self, i: usize) -> &mut [f32; 3] {
        &mut self.values[i]
    }
}

impl Mul<&Mat3> for Mat3 {
    type Output = Mat3;

    fn mul(self, rhs: &Mat3) -> Self::Output {
        let mut ret = Mat3::new();

        for i in 0..3 {
            let a = self.values[i][0];
            let b = self.values[i][1];
            let c = self.values[i][2];

            for j in 0..3 {
                let e = a * rhs.values[0][j];
                let f = b * rhs.values[1][j];
                let g = c * rhs.values[2][j];
                ret[i][j] = e + f + g;
            }
        }

        ret
    }
}

impl Mul<Mat3> for Mat3 {
    type Output = Mat3;

    fn mul(self, rhs: Mat3) -> Self::Output {
        self * &rhs
    }
}

impl From<&Quat> for Mat3 {
    fn from(q: &Quat) -> Self {
        let xq = f32x4::splat(q.simd[0]) * q.simd;
        let yq = f32x4::splat(q.simd[1]) * q.simd;
        let zq = f32x4::splat(q.simd[2]) * q.simd;

        Mat3::from([
            [
                1.0 - 2.0 * (yq[1] + zq[2]),
                2.0 * (xq[1] - zq[3]),
                2.0 * (xq[2] + yq[3]),
            ],
            [
                2.0 * (xq[1] + zq[3]),
                1.0 - 2.0 * (xq[0] + zq[2]),
                2.0 * (yq[2] - xq[3]),
            ],
            [
                2.0 * (xq[2] - yq[3]),
                2.0 * (yq[2] + xq[3]),
                1.0 - 2.0 * (xq[0] + yq[1]),
            ],
        ])
    }
}

impl Mul<Vec3> for &Mat3 {
    type Output = Vec3;

    fn mul(self, rhs: Vec3) -> Self::Output {
        let mut ret = [0.0, 0.0, 0.0];

        for i in 0..3 {
            for j in 0..3 {
                let vv = rhs.simd[j];
                let mv = self[i][j];
                ret[i] += mv * vv;
            }
        }

        Vec3::new(ret[0], ret[1], ret[2])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn mul() {
        // TODO: Further test multiplication
        let a = Mat3::identity();
        let mut b = Mat3::identity();
        b.scale(&Vec3::new(2.0, 2.0, 2.0));
        assert!(b.values[0][0] == 2.0);
        assert!(b.values[1][1] == 2.0);
        assert!(b.values[2][2] == 2.0);

        let c = a * b;
        assert!(c != Mat3::identity());
        assert!(c.values[0][0] == 2.0);
        assert!(c.values[1][1] == 2.0);
        assert!(c.values[2][2] == 2.0);
    }
}
