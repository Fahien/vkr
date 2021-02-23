// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

#![no_std]
#![feature(register_attr)]
#![register_attr(spirv)]

use spirv_std::glam::{vec4, Vec3, Vec4};
use spirv_std::storage_class::{Input, Output};

#[allow(unused_attributes)]
#[spirv(fragment)]
pub fn main_fs(mut out_color: Output<Vec4>) {
    *out_color = vec4(1.0, 0.0, 0.0, 1.0)
}

#[allow(unused_attributes)]
#[spirv(vertex)]
pub fn main_vs(in_pos: Input<Vec3>, #[spirv(position)] mut out_pos: Output<Vec4>) {
    *out_pos = vec4(in_pos.x, in_pos.y, in_pos.z, 1.0);
}
