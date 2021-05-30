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
    glam::{vec4, Mat3, Mat4, Vec2, Vec3, Vec4},
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
    #[spirv(uniform, descriptor_set = 1, binding = 1)] proj_from_view: &Mat4,
    in_pos: Vec3,
    in_color: Vec4,
    color: &mut Vec4,
    #[spirv(position)] out_pos: &mut Vec4,
) {
    *out_pos = *proj_from_view
        * *view_from_world
        * *world_from_model
        * vec4(in_pos.x, in_pos.y, in_pos.z, 1.0);
    *color = in_color;
}

#[spirv(fragment)]
pub fn main_fs(
    #[spirv(descriptor_set = 0, binding = 1)] image: &SampledImage<Image2d>,
    color: Vec4,
    _normal: Vec3,
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
    #[spirv(uniform, descriptor_set = 1, binding = 1)] proj_from_view: &Mat4,
    in_pos: Vec3,
    in_color: Vec4,
    in_normal: Vec3,
    in_uv: Vec2,
    color: &mut Vec4,
    normal: &mut Vec3,
    uv: &mut Vec2,
    #[spirv(position)] out_pos: &mut Vec4,
) {
    *out_pos = *proj_from_view
        * *view_from_world
        * *world_from_model
        * vec4(in_pos.x, in_pos.y, in_pos.z, 1.0);

    *color = in_color;

    *normal = Mat3::from_mat4(*world_from_model) * in_normal;

    *uv = in_uv;
}
