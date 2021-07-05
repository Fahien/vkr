// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

#![no_std]
#![feature(register_attr)]
#![register_attr(spirv)]

use spirv_std::storage_class::{Input, Output, Uniform, UniformConstant};
use spirv_std::{
    glam::{vec4, Mat4, Vec2, Vec3, Vec4},
    Image2d, SampledImage,
};

#[allow(unused_attributes)]
#[spirv(fragment)]
pub fn line_fs(color: Input<Vec4>, mut out_color: Output<Vec4>) {
    *out_color = *color;
}

#[spirv(block)]
pub struct Mat {
    matrix: Mat4,
}

#[spirv(block)]
pub struct Color {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

#[allow(unused_attributes)]
#[spirv(vertex)]
pub fn line_vs(
    #[spirv(descriptor_set = 0, binding = 0)] model: Uniform<Mat>,
    #[spirv(descriptor_set = 1, binding = 0)] view: Uniform<Mat>,
    #[spirv(descriptor_set = 1, binding = 1)] proj: Uniform<Mat>,
    in_pos: Input<Vec3>,
    in_color: Input<Vec4>,
    mut color: Output<Vec4>,
    #[spirv(position)] mut out_pos: Output<Vec4>,
) {
    *out_pos = proj.matrix * view.matrix * model.matrix * vec4(in_pos.x, in_pos.y, in_pos.z, 1.0);
    *color = *in_color;
}

#[allow(unused_attributes)]
#[spirv(fragment)]
pub fn normal_fs(
    #[spirv(descriptor_set = 2, binding = 0)] material_color: Uniform<Color>,
    #[spirv(descriptor_set = 2, binding = 1)] material_albedo: UniformConstant<SampledImage<Image2d>>,
    color: Input<Vec4>,
    normal: Input<Vec3>,
    uv: Input<Vec2>,
    mut out_color: Output<Vec4>,
) {
    let frag = Vec4::from(material_albedo.sample(*uv));
    *out_color = *color * frag;
    out_color.x *= normal.x * material_color.r;
    out_color.y *= normal.y * material_color.g;
    out_color.z *= normal.z * material_color.b;
    out_color.w *= material_color.a;
}

#[allow(unused_attributes)]
#[spirv(fragment)]
pub fn main_fs(
    #[spirv(descriptor_set = 2, binding = 0)] material_color: Uniform<Color>,
    #[spirv(descriptor_set = 2, binding = 1)] material_albedo: UniformConstant<SampledImage<Image2d>>,
    color: Input<Vec4>,
    normal: Input<Vec3>,
    uv: Input<Vec2>,
    mut out_color: Output<Vec4>,
) {
    let frag = Vec4::from(material_albedo.sample(*uv));
    *out_color = *color * frag;
    out_color.x *= material_color.r;
    out_color.y *= material_color.g;
    out_color.z *= material_color.b;
    out_color.w *= material_color.a;
}

#[allow(unused_attributes)]
#[spirv(vertex)]
pub fn main_vs(
    #[spirv(descriptor_set = 0, binding = 0)] model: Uniform<Mat>,
    #[spirv(descriptor_set = 1, binding = 0)] view: Uniform<Mat>,
    #[spirv(descriptor_set = 1, binding = 1)] proj: Uniform<Mat>,
    in_pos: Input<Vec3>,
    in_color: Input<Vec4>,
    in_normal: Input<Vec3>,
    in_uv: Input<Vec2>,
    mut color: Output<Vec4>,
    mut normal: Output<Vec3>,
    mut uv: Output<Vec2>,
    #[spirv(position)] mut out_pos: Output<Vec4>,
) {
    *out_pos = proj.matrix * view.matrix * model.matrix * vec4(in_pos.x, in_pos.y, in_pos.z, 1.0);

    *color = *in_color;

    let temp_normal = model.matrix * vec4(in_normal.x, in_normal.y, in_normal.z, 1.0);
    normal.x = temp_normal.x;
    normal.y = temp_normal.y;
    normal.z = temp_normal.z;

    uv.x = in_uv.x;
    uv.y = in_uv.y;
}
