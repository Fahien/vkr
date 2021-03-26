// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

#![no_std]
#![feature(register_attr)]
#![register_attr(spirv)]

use spirv_std::{Image2d, SampledImage, glam::{vec4, Mat4, Vec2, Vec3, Vec4}};
use spirv_std::{
    storage_class::{Input, Output, Uniform, UniformConstant},
};

#[allow(unused_attributes)]
#[spirv(fragment)]
pub fn line_fs(color: Input<Vec4>, mut out_color: Output<Vec4>) {
    *out_color = *color;
}

#[allow(unused_attributes)]
#[spirv(vertex)]
pub fn line_vs(
    #[spirv(descriptor_set = 0, binding = 0)] model: Uniform<Mat4>,
    #[spirv(descriptor_set = 1, binding = 0)] view: Uniform<Mat4>,
    in_pos: Input<Vec3>,
    in_color: Input<Vec4>,
    mut color: Output<Vec4>,
    #[spirv(position)] mut out_pos: Output<Vec4>,
) {
    *out_pos = *view * *model * vec4(in_pos.x, in_pos.y, in_pos.z, 1.0);
    *color = *in_color;
}

#[allow(unused_attributes)]
#[spirv(fragment)]
pub fn main_fs(
    #[spirv(descriptor_set = 0, binding = 1)] image: UniformConstant<SampledImage<Image2d>>,
    color: Input<Vec4>,
    uv: Input<Vec2>,
    mut out_color: Output<Vec4>,
) {
    let frag = Vec4::from(image.sample(*uv));
    *out_color = *color * frag;
}

#[allow(unused_attributes)]
#[spirv(vertex)]
pub fn main_vs(
    #[spirv(descriptor_set = 0, binding = 0)] model: Uniform<Mat4>,
    #[spirv(descriptor_set = 1, binding = 0)] view: Uniform<Mat4>,
    in_pos: Input<Vec3>,
    in_color: Input<Vec4>,
    in_uv: Input<Vec2>,
    mut color: Output<Vec4>,
    mut uv: Output<Vec2>,
    #[spirv(position)] mut out_pos: Output<Vec4>,
) {
    *out_pos = *view * *model * vec4(in_pos.x, in_pos.y, in_pos.z, 1.0);
    *color = *in_color;
    uv.x = in_uv.x;
    // UV coords system in Vulkan has inverted Y
    uv.y = 1.0 - in_uv.y;
}
