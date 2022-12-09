// Copyright © 2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use crate::{Mat4, Quat};

/// Transform
#[repr(C)]
pub struct Trs {
    pub matrix: Mat4,
}

impl Default for Trs {
    fn default() -> Self {
        Self::new()
    }
}

impl Trs {
    pub fn new() -> Self {
        Self {
            matrix: Mat4::identity(),
        }
    }

    pub fn rotate(&mut self, rot: &Quat) {
        self.matrix.rotate(rot)
    }
}
