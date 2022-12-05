// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use crate::{Color, Vec3};

#[repr(C)]
pub struct Vertex {
    pub pos: Vec3,
    pub color: Color,
}

impl Vertex {
    pub fn new(x: f32, y: f32, z: f32) -> Vertex {
        Vertex {
            pos: Vec3::new(x, y, z),
            color: Color::new(1.0, 1.0, 1.0, 1.0),
        }
    }
}
