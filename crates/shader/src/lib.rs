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

use spirv_std::{
    glam::{vec4, Mat4, Vec2, Vec3, Vec4},
    image::{Image2d, SampledImage},
};

#[spirv(fragment)]
pub fn line_fs(color: Vec4, out_color: &mut Vec4) {
    *out_color = color;
}

#[spirv(vertex)]
pub fn line_vs(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] world_from_model: &Mat4,
    #[spirv(uniform, descriptor_set = 1, binding = 0)] view_from_world: &Mat4,
    in_pos: Vec3,
    in_color: Vec4,
    color: &mut Vec4,
    #[spirv(position)] out_pos: &mut Vec4,
) {
    *out_pos = *view_from_world * *world_from_model * vec4(in_pos.x, in_pos.y, in_pos.z, 1.0);
    *color = in_color;
}

#[spirv(fragment)]
pub fn main_fs(
    #[spirv(descriptor_set = 0, binding = 1)] image: &SampledImage<Image2d>,
    color: Vec4,
    uv: Vec2,
    out_color: &mut Vec4,
) {
    let sample: Vec4 = unsafe { image.sample(uv) };
    *out_color = color * sample;
}

#[spirv(vertex)]
pub fn main_vs(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] world_from_model: &Mat4,
    #[spirv(uniform, descriptor_set = 1, binding = 0)] view_from_world: &Mat4,
    in_pos: Vec3,
    in_color: Vec4,
    in_uv: Vec2,
    color: &mut Vec4,
    uv: &mut Vec2,
    #[spirv(position)] out_pos: &mut Vec4,
) {
    *out_pos = *view_from_world * *world_from_model * vec4(in_pos.x, in_pos.y, in_pos.z, 1.0);
    *color = in_color;
    uv.x = in_uv.x;
    // UV coords system in Vulkan has inverted Y
    uv.y = 1.0 - in_uv.y;
}
