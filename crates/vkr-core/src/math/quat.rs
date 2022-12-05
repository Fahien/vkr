// Copyright © 2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{
    ops::{Mul, MulAssign},
    simd::{f32x4, SimdFloat},
};

use crate::*;

/// Quaternion structure
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Quat {
    pub simd: f32x4,
}

impl Quat {
    pub fn simd(simd: f32x4) -> Self {
        Self { simd }
    }
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Quat {
            simd: f32x4::from_array([x, y, z, w]),
        }
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
    pub fn get_w(&self) -> f32 {
        self.simd[3]
    }

    pub fn axis_angle(axis: Vec3, angle_radians: f32) -> Self {
        let factor = (angle_radians / 2.0).sin();

        let simd = axis.simd * f32x4::splat(factor)
            + f32x4::from_array([0.0, 0.0, 0.0, (angle_radians / 2.0).cos()]);

        let mut ret = Quat::simd(simd);
        ret.normalize();
        ret
    }

    /// Standard euclidean for product in 4D
    pub fn dot(&self, rhs: &Quat) -> f32 {
        (self.simd * rhs.simd).reduce_sum()
    }

    pub fn len(&self) -> f32 {
        self.dot(self).sqrt()
    }

    pub fn normalize(&mut self) {
        let len = self.len();
        self.simd /= f32x4::splat(len);
    }

    pub fn is_normalized(&self) -> bool {
        (self.len() - 1.0).abs() < 0.001
    }

    pub fn get_conjugate(&self) -> Self {
        Self::simd(self.simd * f32x4::from_array([-1.0, -1.0, -1.0, 1.0]))
    }

    pub fn get_inverse(&self) -> Self {
        // The inverse of a unit quaternion is its conjugate
        assert!(self.is_normalized());
        self.get_conjugate()
    }
}

impl Default for Quat {
    fn default() -> Self {
        Quat::new(0.0, 0.0, 0.0, 1.0)
    }
}

impl From<&Mat3> for Quat {
    fn from(matrix: &Mat3) -> Self {
        let mut ret;

        let t = matrix[0][0] + matrix[1][1] + matrix[2][2];
        if t > 0.0 {
            let s = 0.5 / (t + 1.0).sqrt();
            ret = Quat::new(
                (matrix[2][1] - matrix[1][2]) * s,
                (matrix[0][2] - matrix[2][0]) * s,
                (matrix[1][0] - matrix[0][1]) * s,
                0.25 / s,
            );
        } else if matrix[0][0] > matrix[1][1] && matrix[0][0] > matrix[2][2] {
            let s = 2.0 * (1.0 + matrix[0][0] - matrix[1][1] - matrix[2][2]).sqrt();
            ret = Quat::new(
                0.25 * s,
                (matrix[0][1] + matrix[1][0]) / s,
                (matrix[0][2] + matrix[2][0]) / s,
                (matrix[2][1] - matrix[1][2]) / s,
            );
        } else if matrix[1][1] > matrix[2][2] {
            let s = 2.0 * (1.0 + matrix[1][1] - matrix[0][0] - matrix[2][2]).sqrt();
            ret = Quat::new(
                (matrix[0][1] + matrix[1][0]) / s,
                0.25 * s,
                (matrix[1][2] + matrix[2][1]) / s,
                (matrix[0][2] - matrix[2][0]) / s,
            );
        } else {
            let s = 2.0 * (1.0 + matrix[2][2] - matrix[0][0] - matrix[1][1]).sqrt();
            ret = Quat::new(
                (matrix[0][2] + matrix[2][0]) / s,
                (matrix[1][2] + matrix[2][1]) / s,
                0.25 * s,
                (matrix[1][0] - matrix[0][1]) / s,
            );
        }

        ret.normalize();

        ret
    }
}

impl From<&Mat4> for Quat {
    fn from(matrix: &Mat4) -> Self {
        let mut ret;

        let t = matrix[0][0] + matrix[1][1] + matrix[2][2];
        if t > 0.0 {
            let s = 0.5 / (t + 1.0).sqrt();
            ret = Quat::new(
                (matrix[2][1] - matrix[1][2]) * s,
                (matrix[0][2] - matrix[2][0]) * s,
                (matrix[1][0] - matrix[0][1]) * s,
                0.25 / s,
            )
        } else if matrix[0][0] > matrix[1][1] && matrix[0][0] > matrix[2][2] {
            let s = 2.0 * (1.0 + matrix[0][0] - matrix[1][1] - matrix[2][2]).sqrt();
            ret = Quat::new(
                0.25 * s,
                (matrix[0][1] + matrix[1][0]) / s,
                (matrix[0][2] + matrix[2][0]) / s,
                (matrix[2][1] - matrix[1][2]) / s,
            );
        } else if matrix[1][1] > matrix[2][2] {
            let s = 2.0 * (1.0 + matrix[1][1] - matrix[0][0] - matrix[2][2]).sqrt();
            ret = Quat::new(
                (matrix[0][1] + matrix[1][0]) / s,
                0.25 * s,
                (matrix[1][2] + matrix[2][1]) / s,
                (matrix[0][2] - matrix[2][0]) / s,
            );
        } else {
            let s = 2.0 * (1.0 + matrix[2][2] - matrix[0][0] - matrix[1][1]).sqrt();
            ret = Quat::new(
                (matrix[0][2] + matrix[2][0]) / s,
                (matrix[1][2] + matrix[2][1]) / s,
                0.25 * s,
                (matrix[1][0] - matrix[0][1]) / s,
            );
        }

        ret.normalize();
        ret
    }
}

impl From<Mat4> for Quat {
    fn from(matrix: Mat4) -> Self {
        Quat::from(&matrix)
    }
}

impl Mul<Quat> for Quat {
    type Output = Quat;

    fn mul(self, rhs: Quat) -> Self::Output {
        Self::new(
            self.get_x() * rhs.get_w() + self.get_y() * rhs.get_z() - self.get_z() * rhs.get_y()
                + self.get_w() * rhs.get_x(),
            -self.get_x() * rhs.get_z()
                + self.get_y() * rhs.get_w()
                + self.get_z() * rhs.get_x()
                + self.get_w() * rhs.get_y(),
            self.get_x() * rhs.get_y() - self.get_y() * rhs.get_x()
                + self.get_z() * rhs.get_w()
                + self.get_w() * rhs.get_z(),
            -self.get_x() * rhs.get_x() - self.get_y() * rhs.get_y() - self.get_z() * rhs.get_z()
                + self.get_w() * rhs.get_w(),
        )
    }
}

impl MulAssign<Quat> for Quat {
    fn mul_assign(&mut self, rhs: Quat) {
        *self = *self * rhs;
    }
}

impl Mul<Vec3> for Quat {
    type Output = Vec3;

    fn mul(self, mut rhs: Vec3) -> Self::Output {
        rhs.rotate(&self);
        rhs
    }
}

#[cfg(test)]
mod test {
    use std::f32::consts::FRAC_PI_4;

    use super::*;

    #[test]
    fn invert() {
        // Rotation of PI/2 around Y axis (notice we use angle/2.0)
        let a = Quat::new(0.0, FRAC_PI_4.sin(), 0.0, FRAC_PI_4.cos());
        let b = a.get_inverse();
        assert!(a.get_x() == b.get_x());
        assert!(a.get_y() == -b.get_y());
        assert!(a.get_z() == b.get_z());
        assert!(a.get_w() == b.get_w());
        assert!(b.is_normalized());
    }
}
