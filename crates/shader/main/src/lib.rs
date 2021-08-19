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

#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

use spirv_std::{
    glam::{vec4, Mat3, Mat4, IVec2, Vec2, Vec3, Vec4},
    image::{Image, Image2d, SampledImage},
};

#[spirv(fragment)]
pub fn line_fs(color: Vec4, out_color: &mut Vec4) {
    *out_color = color;
}

#[spirv(vertex)]
pub fn line_vs(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] world_from_model: &Mat4,
    #[spirv(uniform, descriptor_set = 0, binding = 1)] _view_from_model: &Mat4,
    #[spirv(uniform, descriptor_set = 1, binding = 0)] view_from_world: &Mat4,
    #[spirv(uniform, descriptor_set = 1, binding = 1)] proj_from_view: &Mat4,
    in_pos: Vec3,
    in_color: Vec4,
    _in_normal: Vec3,
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
    #[spirv(uniform, descriptor_set = 2, binding = 0)] material_color: &Vec4,
    #[spirv(descriptor_set = 2, binding = 1)] material_albedo: &SampledImage<Image2d>,
    color: Vec4,
    _normal: Vec3,
    uv: Vec2,
    out_color: &mut Vec4,
) {
    let sample: Vec4 = unsafe { material_albedo.sample(uv) };
    *out_color = *material_color * color * sample;
}

#[spirv(vertex)]
pub fn main_vs(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] world_from_model: &Mat4,
    #[spirv(uniform, descriptor_set = 0, binding = 1)] view_from_model: &Mat4,
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

    *normal = Mat3::from_mat4(*view_from_model) * in_normal;

    *uv = in_uv;
}

#[spirv(fragment)]
pub fn normal_fs(
    #[spirv(descriptor_set = 0, binding = 0, input_attachment_index = 0)] _albedo: &Image!(subpass, type=f32, sampled=false),
    #[spirv(descriptor_set = 0, binding = 1, input_attachment_index = 1)] normal: &Image!(subpass, type=f32, sampled=false),
    out_color: &mut Vec4,
) {
    let norm: Vec4 = normal.read_subpass(IVec2::new(0, 0));
    out_color.x = (norm.x * 2.0) - 1.0;
    out_color.y = (norm.y * 2.0) - 1.0;
    out_color.z = (norm.z * 2.0) - 1.0;
    out_color.w = 1.0;
}

#[spirv(fragment)]
pub fn present_fs(
    #[spirv(descriptor_set = 0, binding = 0, input_attachment_index = 0)] albedo: &Image!(subpass, type=f32, sampled=false),
    #[spirv(descriptor_set = 0, binding = 1, input_attachment_index = 1)] _normal: &Image!(subpass, type=f32, sampled=false),
    out_color: &mut Vec4,
) {
    let frag: Vec4 = albedo.read_subpass(IVec2::new(0, 0));
    *out_color = frag;
}

#[spirv(vertex)]
pub fn present_vs(in_pos: Vec2, #[spirv(position, invariant)] out_pos: &mut Vec4) {
    *out_pos = vec4(in_pos.x, in_pos.y, 0.0, 1.0);
}
