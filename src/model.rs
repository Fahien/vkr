// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use super::*;
use ash::{vk, Device};
use memoffset::offset_of;
use nalgebra as na;

pub trait VertexInput {
    fn get_bindings() -> ash::vk::VertexInputBindingDescription;
    fn get_attributes() -> Vec<ash::vk::VertexInputAttributeDescription>;
    fn get_set_layout_bindings() -> Vec<ash::vk::DescriptorSetLayoutBinding>;
    fn write_set(
        device: &Device,
        set: vk::DescriptorSet,
        ubo: &Buffer,
        view: Option<&ImageView>,
        sampler: Option<&Sampler>,
    );
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

    pub fn white() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }
}

#[repr(C)]
pub struct Point {
    pos: na::Vector3<f32>,
    color: Color,
}

impl Point {
    pub fn new(pos: na::Vector3<f32>, color: Color) -> Self {
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

    fn get_set_layout_bindings() -> Vec<ash::vk::DescriptorSetLayoutBinding> {
        vec![ash::vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(ash::vk::DescriptorType::UNIFORM_BUFFER) // delta time?
            .descriptor_count(1) // Referring the shader
            .stage_flags(ash::vk::ShaderStageFlags::VERTEX)
            .build()]
    }

    fn write_set(
        device: &Device,
        set: vk::DescriptorSet,
        ubo: &Buffer,
        _view: Option<&ImageView>,
        _sampler: Option<&Sampler>,
    ) {
        // Update immediately the descriptor sets
        let buffer_info = ash::vk::DescriptorBufferInfo::builder()
            .range(std::mem::size_of::<na::Matrix4<f32>>() as ash::vk::DeviceSize)
            .buffer(ubo.buffer)
            .build();

        let write = ash::vk::WriteDescriptorSet::builder()
            .dst_set(set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(ash::vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&[buffer_info])
            .build();

        let writes = vec![write];
        unsafe {
            device.update_descriptor_sets(&writes, &[]);
        }
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

    fn get_set_layout_bindings() -> Vec<ash::vk::DescriptorSetLayoutBinding> {
        Point::get_set_layout_bindings()
    }

    fn write_set(
        device: &Device,
        set: vk::DescriptorSet,
        ubo: &Buffer,
        view: Option<&ImageView>,
        sampler: Option<&Sampler>,
    ) {
        Point::write_set(device, set, ubo, view, sampler);
    }
}

#[repr(C)]
pub struct Vertex {
    pub pos: na::Vector3<f32>,
    pub color: Color,
    pub uv: na::Vector2<f32>,
}

impl Vertex {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            pos: na::Vector3::new(x, y, z),
            color: Color::new(1.0, 1.0, 1.0, 1.0),
            uv: na::Vector2::new(0.0, 0.0),
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
            ash::vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(2)
                .format(ash::vk::Format::R32G32_SFLOAT)
                .offset(offset_of!(Vertex, uv) as u32)
                .build(),
        ]
    }

    fn get_set_layout_bindings() -> Vec<ash::vk::DescriptorSetLayoutBinding> {
        let uniform = ash::vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(ash::vk::DescriptorType::UNIFORM_BUFFER) // delta time?
            .descriptor_count(1) // Referring the shader
            .stage_flags(ash::vk::ShaderStageFlags::VERTEX)
            .build();

        let sampler = ash::vk::DescriptorSetLayoutBinding::builder()
            .binding(1)
            .descriptor_type(ash::vk::DescriptorType::COMBINED_IMAGE_SAMPLER) // delta time?
            .descriptor_count(1) // Referring the shader
            .stage_flags(ash::vk::ShaderStageFlags::FRAGMENT)
            .build();

        vec![uniform, sampler]
    }

    fn write_set(
        device: &Device,
        set: vk::DescriptorSet,
        ubo: &Buffer,
        view: Option<&ImageView>,
        sampler: Option<&Sampler>,
    ) {
        // Update immediately the descriptor sets
        let buffer_info = ash::vk::DescriptorBufferInfo::builder()
            .range(std::mem::size_of::<na::Matrix4<f32>>() as ash::vk::DeviceSize)
            .buffer(ubo.buffer)
            .build();

        let image_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .sampler(sampler.unwrap().sampler)
            .image_view(view.unwrap().view)
            .build();

        let buffer_write = ash::vk::WriteDescriptorSet::builder()
            .dst_set(set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(ash::vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&[buffer_info])
            .build();

        let image_write = vk::WriteDescriptorSet::builder()
            .dst_set(set)
            .dst_binding(1)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&[image_info])
            .build();

        let writes = vec![buffer_write, image_write];

        unsafe {
            device.update_descriptor_sets(&writes, &[]);
        }
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

    pub fn translate(&mut self, trs: &na::Vector3<f32>) {
        let trs = na::Translation3::from(*trs);
        self.model.append_translation_mut(&trs);
    }

    pub fn rotate(&mut self, rot: &na::UnitQuaternion<f32>) {
        self.model.append_rotation_mut(rot);
    }

    pub fn scale(&mut self, scl: &na::Vector3<f32>) {
        self.scale = *scl;
    }
}

pub struct Camera {
    pub proj: na::Matrix4<f32>,
}

impl Camera {
    pub fn perspective(aspect: f32, fovy: f32, znear: f32, zfar: f32) -> Self {
        Self {
            proj: na::Perspective3::new(aspect, fovy, znear, zfar).to_homogeneous(),
        }
    }

    pub fn orthographic(
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        znear: f32,
        zfar: f32,
    ) -> Self {
        Self {
            proj: na::Orthographic3::new(left, right, bottom, top, znear, zfar).to_homogeneous(),
        }
    }
}

pub struct Node {
    pub trs: Trs,
    pub children: Vec<Handle<Node>>,
    pub camera: Handle<Camera>,
}

impl Node {
    pub fn new() -> Self {
        Node {
            trs: Trs::new(),
            children: vec![],
            camera: Handle::none(),
        }
    }
}

pub struct Model {
    pub cameras: Pack<Camera>,
    pub nodes: Pack<Node>,
    pub images: Pack<Image>,
    pub views: Pack<ImageView>,
    pub samplers: Pack<Sampler>,
    pub textures: Pack<Texture>,
}

impl Model {
    pub fn new() -> Self {
        Self {
            cameras: Pack::new(),
            nodes: Pack::new(),
            images: Pack::new(),
            views: Pack::new(),
            samplers: Pack::new(),
            textures: Pack::new(),
        }
    }
}
