// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::vk;
use memoffset::offset_of;

use crate::{Color, Vec3, VertexInput};

#[repr(C)]
pub struct Vertex {
    pub pos: Vec3,
    pub color: Color,
}

impl Vertex {
    pub fn new(x: f32, y: f32, z: f32) -> Vertex {
        Vertex {
            pos: Vec3::new(x, y, z),
            color: Color::new(1.0, 1.0, 1.0, 1.0),
        }
    }
}

impl VertexInput for Vertex {
    fn get_bindings() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()]
    }

    fn get_attributes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Vertex, pos) as u32)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(offset_of!(Vertex, color) as u32)
                .build(),
        ]
    }
}
