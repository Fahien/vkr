// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::vk;
use memoffset::offset_of;

use crate::{Color, Vec3, VertexInput};

#[repr(C)]
pub struct Point3 {
    pub pos: Vec3,
    pub color: Color,
}

impl Point3 {
    pub fn new(pos: Vec3, color: Color) -> Self {
        Self { pos, color }
    }
}

impl VertexInput for Point3 {
    fn get_bindings() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()]
    }

    fn get_attributes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Self, pos) as u32)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(offset_of!(Self, color) as u32)
                .build(),
        ]
    }
}
