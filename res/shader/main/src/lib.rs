// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

#![no_std]
#![feature(register_attr)]
#![register_attr(spirv)]

use spirv_std::storage_class::{Input, Output, Uniform, UniformConstant};
use spirv_std::{
    glam::{vec4, Mat3, Mat4, Vec2, Vec3, Vec4},
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
pub struct Material {
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
pub fn main_fs(
    #[spirv(descriptor_set = 0, binding = 1)] image: UniformConstant<SampledImage<Image2d>>,
    #[spirv(descriptor_set = 2, binding = 0)] material: Uniform<Material>,
    color: Input<Vec4>,
    normal: Input<Vec3>,
    uv: Input<Vec2>,
    mut out_color: Output<Vec4>,
) {
    let frag = Vec4::from(image.sample(*uv));
    *out_color = *color * frag;
    out_color.x *= material.r;
    out_color.y *= material.g;
    out_color.z *= material.b;
    out_color.w *= material.a;
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

    *normal = Mat3::from(model.matrix.inverse().transpose()) * in_normal;

    uv.x = in_uv.x;
    // UV coords system in Vulkan has inverted Y
    uv.y = in_uv.y;
}
