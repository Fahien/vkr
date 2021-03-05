// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

pub trait VertexInput {
    fn get_bindings() -> ash::vk::VertexInputBindingDescription;
    fn get_attributes() -> ash::vk::VertexInputAttributeDescription;
}

#[repr(C)]
pub struct Vec3f {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3f {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3f { x, y, z }
    }
}

#[repr(C)]
pub struct Point {
    pos: Vec3f,
}

impl Point {
    pub fn _new(x: f32, y: f32, z: f32) -> Self {
        Self {
            pos: Vec3f::new(x, y, z),
        }
    }
}

impl VertexInput for Point {
    fn get_bindings() -> ash::vk::VertexInputBindingDescription {
        ash::vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Point>() as u32)
            .input_rate(ash::vk::VertexInputRate::VERTEX)
            .build()
    }

    fn get_attributes() -> ash::vk::VertexInputAttributeDescription {
        ash::vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(ash::vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build()
    }
}

#[repr(C)]
pub struct Vertex {
    pos: Vec3f,
}

impl Vertex {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            pos: Vec3f::new(x, y, z),
        }
    }
}

impl VertexInput for Vertex {
    fn get_bindings() -> ash::vk::VertexInputBindingDescription {
        ash::vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Vertex>() as u32)
            .input_rate(ash::vk::VertexInputRate::VERTEX)
            .build()
    }

    fn get_attributes() -> ash::vk::VertexInputAttributeDescription {
        ash::vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(ash::vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build()
    }
}
