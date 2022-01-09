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

use spirv_std::{
    glam::{vec4, Mat4, Vec2, Vec3, Vec4},
    image::{Image, Image2d, SampledImage},
};

// This file is parsed as a `syn::File`
// This function will appear within its `items`
// We can parse its attributes as a `Meta` to find a `MetaList` (spirv)
// A `MetaList` contains other `Meta`s, in this case a `Path` (fragment)
#[spirv(fragment)]
pub fn main_fs(out_color: &mut Vec4) {
    *out_color = vec4(1.0, 0.0, 0.0, 1.0)
}

#[spirv(vertex)]
pub fn main_vs(in_pos: Vec3, #[spirv(position)] out_pos: &mut Vec4) {
    *out_pos = vec4(in_pos.x, in_pos.y, in_pos.z, 1.0);
}

#[spirv(fragment)]
pub fn secondary_fs(out_color: &mut Vec4) {
    *out_color = vec4(1.0, 0.0, 0.0, 1.0)
}

#[spirv(vertex)]
pub fn secondary_vs(in_pos: Vec3, in_uv: Vec2, #[spirv(position)] out_pos: &mut Vec4) {
    *out_pos = vec4(in_pos.x, in_uv.y, in_pos.z, 1.0);
}

#[spirv(fragment)]
pub fn uniform_fs(
    #[spirv(uniform, descriptor_set = 2, binding = 0)] color: &Vec4,
    #[spirv(descriptor_set = 2, binding = 1)] albedo: &SampledImage<Image2d>,
    #[spirv(descriptor_set = 3, binding = 0, input_attachment_index = 0)] _att: &Image!(subpass, type=f32, sampled=false),
    out_color: &mut Vec4,
) {
    let sample: Vec4 = unsafe { albedo.sample(Vec2::new(0.0, 0.0)) };
    *out_color = *color * sample;
}

#[spirv(vertex)]
pub fn uniform_vs(
    in_pos: Vec3,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] view: &Mat4,
    #[spirv(uniform, descriptor_set = 1, binding = 0)] transform: &Mat4,
    #[spirv(position)] out_pos: &mut Vec4,
) {
    *out_pos = *view * *transform * vec4(in_pos.x, in_pos.y, in_pos.z, 1.0);
}
