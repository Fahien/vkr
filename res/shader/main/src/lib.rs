// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

#![no_std]
#![feature(register_attr)]
#![register_attr(spirv)]

use spirv_std::storage_class::{Input, Output, Uniform, UniformConstant};
use spirv_std::{
    glam::{vec2, vec4, Mat4, Vec2, Vec3, Vec4},
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
    color: Input<Vec4>,
    normal: Input<Vec3>,
    uv: Input<Vec2>,
    mut out_color: Output<Vec4>,
    mut out_normal: Output<Vec4>
) {
    let frag = Vec4::from(image.sample(*uv));
    *out_color = *color * frag;

    let normal = normal.normalize();
    out_normal.x = normal.x;
    out_normal.y = normal.y;
    out_normal.z = normal.z;
    out_normal.w = 1.0;
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
    // UV coords system in Vulkan has inverted Y
    uv.y = in_uv.y;
}

#[allow(unused_attributes)]
#[spirv(fragment)]
pub fn present_fs(
    // @todo Input attachments
    #[spirv(descriptor_set = 0, binding = 0)] inv_view_proj: Uniform<Mat>,
    #[spirv(descriptor_set = 1, binding = 0)] inv_resolution: Uniform<Vec2>,
    uv: Input<Vec2>,
    mut out_color: Output<Vec4>,
) {
    *out_color = image.read_subpass(*uv);
    out_color.x = 0.5;
}

#[allow(unused_attributes)]
#[spirv(vertex)]
pub fn present_vs(
    in_uv: Input<Vec2>,
    mut out_uv: Output<Vec2>,
    #[spirv(position)] mut out_pos: Output<Vec4>,
) {
    *out_uv = *in_uv;
    *out_pos = vec4(in_uv.x * 2.0 - 1.0, in_uv.y * 2.0 - 1.0, 0.0, 1.0);
}
