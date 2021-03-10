// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

#![no_std]
#![feature(register_attr)]
#![register_attr(spirv)]

use spirv_std::glam::{vec4, Vec3, Vec4, Mat4};
use spirv_std::storage_class::{Input, Output, Uniform};

#[allow(unused_attributes)]
#[spirv(fragment)]
pub fn main_fs(color: Input<Vec4>, mut out_color: Output<Vec4>) {
    *out_color = *color;
}

#[spirv(block)]
pub struct Ubo {
    matrix: Mat4,
}

#[allow(unused_attributes)]
#[spirv(vertex)]
pub fn main_vs(
    #[spirv(descriptor_set = 0, binding = 0)] model: Uniform<Ubo>,
    in_pos: Input<Vec3>,
    in_color: Input<Vec4>,
    mut color: Output<Vec4>,
    #[spirv(position)] mut out_pos: Output<Vec4>,
) {
    *out_pos = (*model).matrix * vec4(in_pos.x, in_pos.y, in_pos.z, 1.0);
    *color = *in_color;
}
