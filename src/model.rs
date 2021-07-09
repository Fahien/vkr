// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use super::*;
use ash::{vk, Device};
use memoffset::offset_of;
use nalgebra as na;

pub fn create_set_layout(
    device: &Device,
    bindings: &[vk::DescriptorSetLayoutBinding],
) -> vk::DescriptorSetLayout {
    let set_layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
        .bindings(bindings)
        .build();
    unsafe { device.create_descriptor_set_layout(&set_layout_info, None) }
        .expect("Failed to create Vulkan descriptor set layout")
}

pub trait VertexInput {
    fn get_bindings() -> vk::VertexInputBindingDescription;

    fn get_attributes() -> Vec<vk::VertexInputAttributeDescription>;

    /// @todo Would it be useful to follow a convention where we know exactly which set layout is at a certain index?
    fn get_set_layouts(device: &Device) -> Vec<vk::DescriptorSetLayout>;

    fn get_constants() -> Vec<vk::PushConstantRange> {
        vec![]
    }

    fn write_set_model(_device: &Device, _set: vk::DescriptorSet, _ubo: &Buffer) {}

    fn write_set_image(
        _device: &Device,
        _set: vk::DescriptorSet,
        _view: &ImageView,
        _sampler: &Sampler,
    ) {
    }

    fn get_depth_state() -> vk::PipelineDepthStencilStateCreateInfo {
        vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::GREATER)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false)
            .build()
    }

    fn get_color_blend() -> Vec<vk::PipelineColorBlendAttachmentState> {
        vec![vk::PipelineColorBlendAttachmentState::builder()
            .blend_enable(true)
            .color_write_mask(vk::ColorComponentFlags::all())
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .color_blend_op(vk::BlendOp::ADD)
            .build()]
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

    pub fn white() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }
}

#[repr(C)]
pub struct Material {
    pub color: Color,
    pub albedo: Handle<Texture>,
}

impl Material {
    pub fn new(color: Color) -> Self {
        let albedo = Handle::none();
        Self { color, albedo }
    }

    pub fn textured(albedo: Handle<Texture>) -> Self {
        let color = Color::white();
        Self { color, albedo }
    }

    pub fn get_set_layout_bindings() -> Vec<vk::DescriptorSetLayoutBinding> {
        let color = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER) // color
            .descriptor_count(1) // Referring the shader?
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .build();

        let albedo = vk::DescriptorSetLayoutBinding::builder()
            .binding(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .build();

        vec![color, albedo]
    }

    pub fn write_set(
        device: &Device,
        set: vk::DescriptorSet,
        material: &Buffer,
        albedo: &ImageView,
        sampler: &Sampler,
    ) {
        let buffer_info = vk::DescriptorBufferInfo::builder()
            .range(std::mem::size_of::<Color>() as vk::DeviceSize)
            .buffer(material.buffer)
            .build();

        let buffer_write = vk::WriteDescriptorSet::builder()
            .dst_set(set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&[buffer_info])
            .build();

        let image_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(albedo.view)
            .sampler(sampler.sampler)
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
    fn get_bindings() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Point>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    fn get_attributes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Point, pos) as u32)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(offset_of!(Point, color) as u32)
                .build(),
        ]
    }

    fn get_set_layouts(device: &Device) -> Vec<vk::DescriptorSetLayout> {
        let model_bindings = vec![vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .build()];
        let model = create_set_layout(device, &model_bindings);

        let camera_bindings = Camera::get_set_layout_bindings();
        let camera = create_set_layout(device, &camera_bindings);

        vec![model, camera]
    }

    fn write_set_model(device: &Device, set: vk::DescriptorSet, ubo: &Buffer) {
        // Update immediately the descriptor sets
        let buffer_info = vk::DescriptorBufferInfo::builder()
            .range(std::mem::size_of::<na::Matrix4<f32>>() as vk::DeviceSize)
            .buffer(ubo.buffer)
            .build();

        let write = vk::WriteDescriptorSet::builder()
            .dst_set(set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
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
    fn get_bindings() -> vk::VertexInputBindingDescription {
        Point::get_bindings()
    }

    fn get_attributes() -> Vec<vk::VertexInputAttributeDescription> {
        Point::get_attributes()
    }

    fn get_set_layouts(device: &Device) -> Vec<vk::DescriptorSetLayout> {
        Point::get_set_layouts(device)
    }

    fn write_set_model(device: &Device, set: vk::DescriptorSet, ubo: &Buffer) {
        Point::write_set_model(device, set, ubo);
    }
}

#[repr(C)]
pub struct Vertex {
    pub pos: na::Vector3<f32>,
    pub color: Color,
    pub normal: na::Vector3<f32>,
    pub uv: na::Vector2<f32>,
}

impl Vertex {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            pos: na::Vector3::new(x, y, z),
            color: Color::white(),
            // From the screen towards the viewer
            normal: na::Vector3::new(0.0, 0.0, 1.0),
            uv: na::Vector2::new(0.0, 0.0),
        }
    }
}

impl VertexInput for Vertex {
    fn get_bindings() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    fn get_attributes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            // position
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Vertex, pos) as u32)
                .build(),
            // color
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(offset_of!(Vertex, color) as u32)
                .build(),
            // normal
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(2)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Vertex, normal) as u32)
                .build(),
            // texture coordinates
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(3)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(offset_of!(Vertex, uv) as u32)
                .build(),
        ]
    }

    fn get_set_layouts(device: &Device) -> Vec<vk::DescriptorSetLayout> {
        let model_bindings = vec![vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .build()];
        let model = create_set_layout(device, &model_bindings);

        let camera_bindings = Camera::get_set_layout_bindings();
        let camera = create_set_layout(device, &camera_bindings);

        let material_bindings = Material::get_set_layout_bindings();
        let material = create_set_layout(device, &material_bindings);

        vec![model, camera, material]
    }

    fn write_set_model(device: &Device, set: vk::DescriptorSet, ubo: &Buffer) {
        // Update immediately the descriptor sets
        let buffer_info = vk::DescriptorBufferInfo::builder()
            .range(std::mem::size_of::<na::Matrix4<f32>>() as vk::DeviceSize)
            .buffer(ubo.buffer)
            .build();

        let buffer_write = vk::WriteDescriptorSet::builder()
            .dst_set(set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&[buffer_info])
            .build();

        let writes = vec![buffer_write];

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

    pub fn get_view_matrix(&self) -> na::Matrix4<f32> {
        let mut matrix = self.get_matrix();

        // Invert translation
        matrix.m14 = -matrix.m14;
        matrix.m24 = -matrix.m24;
        matrix.m34 = -matrix.m34;

        matrix
    }

    pub fn get_translation(&self) -> na::Vector3<f32> {
        self.model.translation.vector
    }

    pub fn get_rotation(&self) -> na::UnitQuaternion<f32> {
        self.model.rotation
    }

    /// Rotates around the origin
    pub fn set_rotation(&mut self, rot: &na::UnitQuaternion<f32>) {
        self.model.rotation = *rot;
    }

    pub fn translate(&mut self, trs: &na::Vector3<f32>) {
        let trs = na::Translation3::from(*trs);
        self.model.append_translation_mut(&trs);
    }

    /// Rotates around the current position
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

    /// Parameters here are referred to the camera, where towards direction is positive.
    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let mid = na::Vector3::new(
            (left + right) / (right - left),
            (bottom + top) / (top - bottom),
            near / (near - far),
        );

        let scale = na::Vector3::new(
            2.0 / (right - left),
            2.0 / (top - bottom),
            1.0 / (near - far),
        );

        Self {
            proj: na::Matrix4::new(
                scale.x, 0.0, 0.0, mid.x, 0.0, -scale.y, 0.0, mid.y, 0.0, 0.0, scale.z, mid.z, 0.0,
                0.0, 0.0, 1.0,
            ),
        }
    }

    pub fn get_set_layout_bindings() -> Vec<vk::DescriptorSetLayoutBinding> {
        let view = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER) // delta time?
            .descriptor_count(1) // Referring the shader?
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .build();

        let proj = vk::DescriptorSetLayoutBinding::builder()
            .binding(1)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .build();

        vec![view, proj]
    }

    pub fn write_set_view(device: &Device, set: vk::DescriptorSet, view: &Buffer) {
        let buffer_info = vk::DescriptorBufferInfo::builder()
            .range(std::mem::size_of::<na::Matrix4<f32>>() as vk::DeviceSize)
            .buffer(view.buffer)
            .build();

        let buffer_write = vk::WriteDescriptorSet::builder()
            .dst_set(set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&[buffer_info])
            .build();

        let writes = vec![buffer_write];

        unsafe {
            device.update_descriptor_sets(&writes, &[]);
        }
    }

    pub fn write_set_proj(device: &Device, set: vk::DescriptorSet, proj: &Buffer) {
        let buffer_info = vk::DescriptorBufferInfo::builder()
            .range(std::mem::size_of::<na::Matrix4<f32>>() as vk::DeviceSize)
            .buffer(proj.buffer)
            .build();

        let buffer_write = vk::WriteDescriptorSet::builder()
            .dst_set(set)
            .dst_binding(1)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&[buffer_info])
            .build();

        let writes = vec![buffer_write];

        unsafe {
            device.update_descriptor_sets(&writes, &[]);
        }
    }
}

type ScriptFn = Box<dyn Fn(f32, &mut Pack<Node>, Handle<Node>)>;

pub struct Script {
    pub update: ScriptFn,
}

impl Script {
    pub fn new(update: ScriptFn) -> Self {
        Self { update }
    }

    pub fn update(delta: f32, nodes: &mut Pack<Node>, scripts: &Pack<Script>, node: Handle<Node>) {
        let hscript = nodes.get(node).unwrap().script;
        if let Some(script) = scripts.get(hscript) {
            let func = &script.update;
            func(delta, nodes, node);
        }

        let children = nodes.get(node).unwrap().children.clone();
        for child in children {
            Self::update(delta, nodes, scripts, child);
        }
    }
}

pub struct Node {
    pub trs: Trs,
    pub children: Vec<Handle<Node>>,
    pub camera: Handle<Camera>,
    pub mesh: Handle<Mesh>,
    pub script: Handle<Script>,
}

impl Node {
    pub fn new() -> Self {
        Node {
            trs: Trs::new(),
            children: vec![],
            camera: Handle::none(),
            mesh: Handle::none(),
            script: Handle::none(),
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
    pub materials: Pack<Material>,
    pub primitives: Pack<Primitive>,
    pub meshes: Pack<Mesh>,
    pub scripts: Pack<Script>,
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
            materials: Pack::new(),
            primitives: Pack::new(),
            meshes: Pack::new(),
            scripts: Pack::new(),
        }
    }
}
