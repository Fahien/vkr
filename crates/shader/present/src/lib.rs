// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

#![cfg_attr(
    target_arch = "spirv",
    no_std,
    feature(register_attr, lang_items),
    register_attr(spirv)
)]
#![deny(warnings)]

#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

use spirv_std::{
    glam::{vec4, IVec2, Vec2, Vec4},
    image::Image,
};

#[allow(unused_attributes)]
#[spirv(fragment)]
pub fn normal_fs(
    #[spirv(descriptor_set = 0, binding = 0, input_attachment_index = 0)] albedo: &Image!(subpass, type=f32, sampled=false),
    #[spirv(descriptor_set = 0, binding = 1, input_attachment_index = 1)] normal: &Image!(subpass, type=f32, sampled=false),
    out_color: &mut Vec4,
) {
    let _frag: Vec4 = albedo.read_subpass(IVec2::new(0, 0));
    let norm: Vec4 = normal.read_subpass(IVec2::new(0, 0));
    out_color.x = (norm.x * 2.0) - 1.0;
    out_color.y = (norm.y * 2.0) - 1.0;
    out_color.z = (norm.z * 2.0) - 1.0;
    out_color.w = 1.0;
}

#[spirv(vertex)]
pub fn normal_vs(in_pos: Vec2, #[spirv(position, invariant)] out_pos: &mut Vec4) {
    *out_pos = vec4(in_pos.x, in_pos.y, 0.0, 1.0);
}

#[allow(unused_attributes)]
#[spirv(fragment)]
pub fn present_fs(
    #[spirv(descriptor_set = 0, binding = 0, input_attachment_index = 0)] albedo: &Image!(subpass, type=f32, sampled=false),
    #[spirv(descriptor_set = 0, binding = 1, input_attachment_index = 1)] normal: &Image!(subpass, type=f32, sampled=false),
    out_color: &mut Vec4,
) {
    let frag: Vec4 = albedo.read_subpass(IVec2::new(0, 0));
    let _norm: Vec4 = normal.read_subpass(IVec2::new(0, 0));
    *out_color = frag;
}

#[spirv(vertex)]
pub fn present_vs(in_pos: Vec2, #[spirv(position, invariant)] out_pos: &mut Vec4) {
    *out_pos = vec4(in_pos.x, in_pos.y, 0.0, 1.0);
}
