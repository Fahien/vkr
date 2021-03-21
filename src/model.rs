// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use super::util::*;
use memoffset::offset_of;
use nalgebra as na;

pub trait VertexInput {
    fn get_bindings() -> ash::vk::VertexInputBindingDescription;
    fn get_attributes() -> Vec<ash::vk::VertexInputAttributeDescription>;
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
pub struct Color {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Color { r, g, b, a }
    }
}

#[repr(C)]
pub struct Point {
    pos: Vec3f,
    color: Color,
}

impl Point {
    pub fn new(pos: Vec3f, color: Color) -> Self {
        Self { pos, color }
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

    fn get_attributes() -> Vec<ash::vk::VertexInputAttributeDescription> {
        vec![
            ash::vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(ash::vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Point, pos) as u32)
                .build(),
            ash::vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(ash::vk::Format::R32G32B32A32_SFLOAT)
                .offset(offset_of!(Point, color) as u32)
                .build(),
        ]
    }
}

#[repr(C)]
pub struct Line {
    a: Point,
    b: Point,
}

impl Line {
    pub fn new(a: Point, b: Point) -> Line {
        Line { a, b }
    }
}

impl VertexInput for Line {
    fn get_bindings() -> ash::vk::VertexInputBindingDescription {
        Point::get_bindings()
    }

    fn get_attributes() -> Vec<ash::vk::VertexInputAttributeDescription> {
        Point::get_attributes()
    }
}

#[repr(C)]
pub struct Vertex {
    pos: Vec3f,
    color: Color,
}

impl Vertex {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            pos: Vec3f::new(x, y, z),
            color: Color::new(1.0, 1.0, 1.0, 1.0),
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

    fn get_attributes() -> Vec<ash::vk::VertexInputAttributeDescription> {
        vec![
            ash::vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(ash::vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Vertex, pos) as u32)
                .build(),
            ash::vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(ash::vk::Format::R32G32B32A32_SFLOAT)
                .offset(offset_of!(Vertex, color) as u32)
                .build(),
        ]
    }
}

/// Transform
pub struct Trs {
    model: na::Isometry3<f32>,
    scale: na::Vector3<f32>,
}

impl Trs {
    pub fn new() -> Self {
        Self {
            model: na::Isometry3::identity(),
            scale: na::Vector3::new(1.0, 1.0, 1.0),
        }
    }

    pub fn get_matrix(&self) -> na::Matrix4<f32> {
        // @todo Verify it works as intended
        self.model
            .to_homogeneous()
            .append_nonuniform_scaling(&self.scale)
    }

    pub fn rotate(&mut self, rot: &na::UnitQuaternion<f32>) {
        self.model.append_rotation_mut(rot);
    }
}

pub struct Node {
    pub trs: Trs,
    pub children: Vec<Handle<Node>>,
}

impl Node {
    pub fn new() -> Self {
        Node {
            trs: Trs::new(),
            children: vec![],
        }
    }
}

pub struct Model {
    pub nodes: Pack<Node>,
    pub images: Pack<Image>,
    pub views: Pack<ImageView>,
    pub samplers: Pack<Sampler>,
    pub textures: Pack<Texture>,
}

impl Model {
    pub fn new() -> Self {
        Self {
            nodes: Pack::new(),
            images: Pack::new(),
            views: Pack::new(),
            samplers: Pack::new(),
            textures: Pack::new(),
        }
    }
}
