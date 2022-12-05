// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use crate::Point3;

#[repr(C)]
pub struct Line {
    a: Point3,
    b: Point3,
}

impl Line {
    pub fn new(a: Point3, b: Point3) -> Line {
        Line { a, b }
    }
}
