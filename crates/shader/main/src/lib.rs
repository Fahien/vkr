// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT
#![cfg_attr(
    target_arch = "spirv",
    no_std,
    feature(register_attr, lang_items),
    register_attr(spirv)
)]
// HACK(eddyb) can't easily see warnings otherwise from `spirv-builder` builds.
#![deny(warnings)]

#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

use spirv_std::{
    glam::{vec4, Mat4, Vec2, Vec3, Vec4},
    image::{Image2d, SampledImage},
};

#[allow(unused_attributes)]
#[spirv(fragment)]
pub fn line_fs(color: Vec4, out_color: &mut Vec4, out_normal: &mut Vec4) {
    *out_color = color;
    *out_normal = vec4(0.0, 0.0, 1.0, 1.0);
}

#[allow(unused_attributes)]
#[spirv(vertex)]
pub fn line_vs(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] model: &Mat4,
    #[spirv(uniform, descriptor_set = 0, binding = 1)] _model_view: &Mat4,
    #[spirv(uniform, descriptor_set = 1, binding = 0)] view: &Mat4,
    #[spirv(uniform, descriptor_set = 1, binding = 1)] proj: &Mat4,
    in_pos: Vec3,
    in_color: Vec4,
    _in_normal: Vec3,
    color: &mut Vec4,
    #[spirv(position)] out_pos: &mut Vec4,
) {
    *out_pos = *proj * *view * *model * vec4(in_pos.x, in_pos.y, in_pos.z, 1.0);
    *color = in_color;
}

#[allow(unused_attributes)]
#[spirv(fragment)]
pub fn main_fs(
    #[spirv(uniform, descriptor_set = 2, binding = 0)] material_color: &Vec4,
    #[spirv(descriptor_set = 2, binding = 1)] material_albedo: &SampledImage<Image2d>,
    color: Vec4,
    normal: Vec3,
    uv: Vec2,
    out_color: &mut Vec4,
    out_normal: &mut Vec4,
) {
    let frag: Vec4 = unsafe { material_albedo.sample(uv) };
    *out_color = color * frag;
    *out_color = *out_color * *material_color;

    out_normal.x = (normal.x + 1.0) / 2.0;
    out_normal.y = (normal.y + 1.0) / 2.0;
    out_normal.z = (normal.z + 1.0) / 2.0;
    out_normal.w = 1.0;
}

#[spirv(vertex)]
pub fn main_vs(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] model: &Mat4,
    #[spirv(uniform, descriptor_set = 0, binding = 1)] model_view: &Mat4,
    #[spirv(uniform, descriptor_set = 1, binding = 0)] view: &Mat4,
    #[spirv(uniform, descriptor_set = 1, binding = 1)] proj: &Mat4,
    in_pos: Vec3,
    in_color: Vec4,
    in_normal: Vec3,
    in_uv: Vec2,
    color: &mut Vec4,
    normal: &mut Vec3,
    uv: &mut Vec2,
    #[spirv(position)] out_pos: &mut Vec4,
) {
    *out_pos = *proj * *view * *model * vec4(in_pos.x, in_pos.y, in_pos.z, 1.0);

    *color = in_color;

    let temp_normal = *model_view * vec4(in_normal.x, in_normal.y, in_normal.z, 1.0);
    normal.x = temp_normal.x;
    normal.y = temp_normal.y;
    normal.z = temp_normal.z;

    uv.x = in_uv.x;
    uv.y = in_uv.y;
}
