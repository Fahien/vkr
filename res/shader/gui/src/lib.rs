// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

#![no_std]
#![feature(register_attr)]
#![register_attr(spirv)]

use spirv_std::storage_class::{Input, Output, PushConstant, Uniform, UniformConstant};
use spirv_std::{
    glam::{vec4, Mat4, Vec2, Vec3, Vec4},
    Image2d, SampledImage,
};

#[spirv(block)]
pub struct Mat {
    matrix: Mat4,
}

#[allow(unused_attributes)]
#[spirv(fragment)]
pub fn gui_fs(
    #[spirv(descriptor_set = 0, binding = 0)] image: UniformConstant<SampledImage<Image2d>>,
    uv: Input<Vec2>,
    color: Input<Vec4>,
    mut out_color: Output<Vec4>,
) {
    let frag = Vec4::from(image.sample(*uv));
    *out_color = *color * frag;
}

#[allow(unused_attributes)]
#[spirv(vertex)]
pub fn gui_vs(
    transform: PushConstant<Mat>,
    in_pos: Input<Vec2>,
    in_uv: Input<Vec2>,
    in_color: Input<Vec4>,
    mut uv: Output<Vec2>,
    mut color: Output<Vec4>,
    #[spirv(position)] mut out_pos: Output<Vec4>,
) {
    *out_pos = transform.matrix * vec4(in_pos.x, in_pos.y, 0.0, 1.0);
    *uv = *in_uv;
    *color = *in_color;
}
