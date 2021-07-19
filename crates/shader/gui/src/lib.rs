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
    glam::{vec4, Mat4, Vec2, Vec4},
    image::{Image2d, SampledImage},
};

#[spirv(fragment)]
pub fn gui_fs(
    #[spirv(descriptor_set = 0, binding = 0)] image: &SampledImage<Image2d>,
    uv: Vec2,
    color: Vec4,
    out_color: &mut Vec4,
) {
    let sample: Vec4 = unsafe { image.sample(uv) };
    *out_color = color * sample;
}

#[spirv(vertex)]
pub fn gui_vs(
    #[spirv(push_constant)] transform: &Mat4,
    in_pos: Vec2,
    in_uv: Vec2,
    in_color: Vec4,
    uv: &mut Vec2,
    color: &mut Vec4,
    #[spirv(position, invariant)] out_pos: &mut Vec4,
) {
    *out_pos = *transform * vec4(in_pos.x, in_pos.y, 0.0, 1.0);
    *uv = in_uv;
    *color = in_color;
}
