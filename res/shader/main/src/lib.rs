// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT
#![cfg_attr(
    target_arch = "spirv",
    no_std,
    feature(register_attr, lang_items),
    register_attr(spirv)
)]

#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

use spirv_std::{
    glam::{vec4, IVec2, Mat3, Mat4, Vec2, Vec3, Vec4},
    image::{Image, Image2d, SampledImage},
};

#[allow(unused_attributes)]
#[spirv(fragment)]
pub fn line_fs(color: Vec4, normal: Vec3, out_color: &mut Vec4, out_normal: &mut Vec4) {
    *out_color = color;
    let normal = normal.normalize();
    *out_normal = normal.extend(1.0);
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
    in_normal: Vec3,
    color: &mut Vec4,
    normal: &mut Vec3,
    #[spirv(position)] out_pos: &mut Vec4,
) {
    *out_pos = proj.matrix * view.matrix * model.matrix * vec4(in_pos.x, in_pos.y, in_pos.z, 1.0);
    *color = in_color;
    *normal = in_normal;
}

#[allow(unused_attributes)]
#[spirv(fragment)]
pub fn main_fs(
    #[spirv(push_constant)] sun_dir: &Vec3,
    #[spirv(uniform, descriptor_set = 2, binding = 0)] material_color: &Color,
    #[spirv(descriptor_set = 2, binding = 1)] material_albedo: &SampledImage<Image2d>,
    color: Vec4,
    normal: Vec3,
    uv: Vec2,
    out_color: &mut Vec4,
    out_normal: &mut Vec4,
) {
    let frag: Vec4 = unsafe { material_albedo.sample(uv) };
    *out_color = color * frag;
    out_color.x *= material_color.r;
    out_color.y *= material_color.g;
    out_color.z *= material_color.b;
    out_color.w *= material_color.a;

    let temp_sun_dir = sun_dir.normalize();
    let normal = normal.normalize();
    let diffuse_factor = normal.dot(temp_sun_dir).max(0.0);
    out_color.x = 0.005 * out_color.x + 0.955 * diffuse_factor * out_color.x;
    out_color.y = 0.005 * out_color.y + 0.955 * diffuse_factor * out_color.y;
    out_color.z = 0.005 * out_color.z + 0.955 * diffuse_factor * out_color.z;

    out_normal.x = (normal.x + 1.0) / 2.0;
    out_normal.y = (normal.y + 1.0) / 2.0;
    out_normal.z = (normal.z + 1.0) / 2.0;
    out_normal.w = 1.0;
}

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

#[allow(unused_attributes)]
#[spirv(fragment)]
pub fn normal_fs(
    #[spirv(descriptor_set = 0, binding = 0, input_attachment_index = 0)] _albedo: &Image!(subpass, type=f32, sampled=false),
    #[spirv(descriptor_set = 0, binding = 1, input_attachment_index = 1)] normal: &Image!(subpass, type=f32, sampled=false),
    #[spirv(descriptor_set = 0, binding = 2, input_attachment_index = 2)] _depth: &Image!(subpass, type=f32, sampled=false),
    out_color: &mut Vec4,
) {
    let norm: Vec4 = normal.read_subpass(IVec2::new(0, 0));
    out_color.x = (norm.x * 2.0) - 1.0;
    out_color.y = (norm.y * 2.0) - 1.0;
    out_color.z = (norm.z * 2.0) - 1.0;
    out_color.w = 1.0;
}

#[allow(unused_attributes)]
#[spirv(fragment)]
pub fn depth_fs(
    #[spirv(descriptor_set = 0, binding = 0, input_attachment_index = 0)] _albedo: &Image!(subpass, type=f32, sampled=false),
    #[spirv(descriptor_set = 0, binding = 1, input_attachment_index = 1)] _normal: &Image!(subpass, type=f32, sampled=false),
    #[spirv(descriptor_set = 0, binding = 2, input_attachment_index = 2)] depth: &Image!(subpass, type=f32, sampled=false),
    out_color: &mut Vec4,
) {
    let depth: Vec4 = depth.read_subpass(IVec2::new(0, 0));
    *out_color = vec4(depth.x, depth.x, depth.x, 1.0);
}

#[allow(unused_attributes)]
#[spirv(fragment)]
pub fn present_fs(
    #[spirv(descriptor_set = 0, binding = 0, input_attachment_index = 0)] albedo: &Image!(subpass, type=f32, sampled=false),
    #[spirv(descriptor_set = 0, binding = 1, input_attachment_index = 1)] normal: &Image!(subpass, type=f32, sampled=false),
    #[spirv(descriptor_set = 0, binding = 2, input_attachment_index = 2)] depth: &Image!(subpass, type=f32, sampled=false),
    out_color: &mut Vec4,
) {
    let frag: Vec4 = albedo.read_subpass(IVec2::new(0, 0));
    let _norm: Vec4 = normal.read_subpass(IVec2::new(0, 0));
    let _depth: Vec4 = depth.read_subpass(IVec2::new(0, 0));
    *out_color = frag;
}

#[allow(unused_attributes)]
#[spirv(vertex)]
pub fn present_vs(in_pos: Vec4, #[spirv(position)] out_pos: &mut Vec4) {
    *out_pos = in_pos;
}
