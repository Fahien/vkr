// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

#![cfg_attr(
    target_arch = "spirv",
    feature(register_attr),
    register_attr(spirv),
    no_std
)]
#![deny(warnings)]

use spirv_std::glam::{vec4, Vec3, Vec4, Mat4};

#[spirv(fragment)]
pub fn line_fs(color: Vec4, out_color: &mut Vec4) {
    *out_color = color;
}

#[spirv(vertex)]
pub fn line_vs(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] world_from_model: &Mat4,
    in_pos: Vec3,
    in_color: Vec4,
    color: &mut Vec4,
    #[spirv(position)] out_pos: &mut Vec4,
) {
    *out_pos = *world_from_model * vec4(in_pos.x, in_pos.y, in_pos.z, 1.0);
    *color = in_color;
}

#[spirv(fragment)]
pub fn main_fs(color: Vec4, out_color: &mut Vec4) {
    *out_color = color;
}

#[spirv(vertex)]
pub fn main_vs(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] world_from_model: &Mat4,
    in_pos: Vec3, in_color: Vec4, color: &mut Vec4, #[spirv(position)] out_pos: &mut Vec4) {
    *out_pos = *world_from_model * vec4(in_pos.x, in_pos.y, in_pos.z, 1.0);
    *color = in_color;
}
