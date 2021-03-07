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
pub fn main_fs(color: Input<Vec4>, mut out_color: Output<Vec4>) {
    *out_color = *color;
}

#[allow(unused_attributes)]
#[spirv(vertex)]
pub fn main_vs(
    in_pos: Input<Vec3>,
    in_color: Input<Vec4>,
    mut color: Output<Vec4>,
    #[spirv(position)] mut out_pos: Output<Vec4>,
) {
    *out_pos = vec4(in_pos.x, in_pos.y, in_pos.z, 1.0);
    *color = *in_color;
}
