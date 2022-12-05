// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use crate::{Color, Vec3};

#[repr(C)]
pub struct Point3 {
    pub pos: Vec3,
    pub color: Color,
}

impl Point3 {
    pub fn new(pos: Vec3, color: Color) -> Self {
        Self { pos, color }
    }
}
