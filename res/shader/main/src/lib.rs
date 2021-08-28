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
    glam::{vec4, IVec2, Mat3, Mat4, Vec2, Vec3, Vec4},
    image::{Image, Image2d, SampledImage},
};

#[allow(unused_attributes)]
#[spirv(fragment)]
pub fn line_fs(color: Vec4, out_color: &mut Vec4) {
    *out_color = color;
}

pub struct Mat {
    matrix: Mat4,
}

pub struct Color {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

#[allow(unused_attributes)]
#[spirv(vertex)]
pub fn line_vs(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] model: &Mat,
    #[spirv(uniform, descriptor_set = 0, binding = 1)] _normal_matrix: &Mat,
    #[spirv(uniform, descriptor_set = 1, binding = 0)] view: &Mat,
    #[spirv(uniform, descriptor_set = 1, binding = 1)] proj: &Mat,
    in_pos: Vec3,
    in_color: Vec4,
    _in_normal: Vec3,
    color: &mut Vec4,
    #[spirv(position)] out_pos: &mut Vec4,
) {
    *out_pos = proj.matrix * view.matrix * model.matrix * vec4(in_pos.x, in_pos.y, in_pos.z, 1.0);
    *color = in_color;
}

#[allow(unused_attributes)]
#[spirv(fragment)]
pub fn normal_fs(
    #[spirv(uniform, descriptor_set = 2, binding = 0)] material_color: &Color,
    #[spirv(descriptor_set = 2, binding = 1)] material_albedo: &SampledImage<Image2d>,
    color: Vec4,
    normal: Vec3,
    uv: Vec2,
    out_color: &mut Vec4,
) {
    let frag: Vec4 = unsafe { material_albedo.sample(uv) };
    *out_color = color * frag;
    out_color.x *= normal.x * material_color.r;
    out_color.y *= normal.y * material_color.g;
    out_color.z *= normal.z * material_color.b;
    out_color.w *= material_color.a;
}

#[allow(unused_attributes)]
#[spirv(fragment)]
pub fn main_fs(
    #[spirv(uniform, descriptor_set = 2, binding = 0)] material_color: &Color,
    #[spirv(descriptor_set = 2, binding = 1)] material_albedo: &SampledImage<Image2d>,
    color: Vec4,
    _normal: Vec3,
    uv: Vec2,
    out_color: &mut Vec4,
) {
    let frag: Vec4 = unsafe { material_albedo.sample(uv) };
    *out_color = color * frag;
    out_color.x *= material_color.r;
    out_color.y *= material_color.g;
    out_color.z *= material_color.b;
    out_color.w *= material_color.a;
}

#[allow(unused_attributes)]
#[spirv(vertex)]
pub fn main_vs(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] model: &Mat,
    #[spirv(uniform, descriptor_set = 0, binding = 1)] normal_matrix: &Mat,
    #[spirv(uniform, descriptor_set = 1, binding = 0)] view: &Mat,
    #[spirv(uniform, descriptor_set = 1, binding = 1)] proj: &Mat,
    in_pos: Vec3,
    in_color: Vec4,
    in_normal: Vec3,
    in_uv: Vec2,
    color: &mut Vec4,
    normal: &mut Vec3,
    uv: &mut Vec2,
    #[spirv(position)] out_pos: &mut Vec4,
) {
    *out_pos = proj.matrix * view.matrix * model.matrix * vec4(in_pos.x, in_pos.y, in_pos.z, 1.0);

    *color = in_color;

    *normal = Mat3::from(normal_matrix.matrix) * in_normal;

    uv.x = in_uv.x;
    uv.y = in_uv.y;
}

#[spirv(fragment)]
pub fn present_fs(
    #[spirv(descriptor_set = 0, binding = 0, input_attachment_index = 0)] image: &Image!(subpass, type=f32, sampled=false),
    out_color: &mut Vec4,
) {
    let frag: Vec4 = image.read_subpass(IVec2::new(0, 0));
    *out_color = frag;
}

#[allow(unused_attributes)]
#[spirv(vertex)]
pub fn present_vs(in_pos: Vec4, #[spirv(position)] out_pos: &mut Vec4) {
    *out_pos = in_pos;
}
