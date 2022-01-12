// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::{vk, Device};
use memoffset::offset_of;

use crate::*;

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
    fn get_pipeline() -> Pipelines {
        Pipelines::MAIN
    }

    fn get_bindings() -> vk::VertexInputBindingDescription;

    fn get_attributes() -> Vec<vk::VertexInputAttributeDescription>;

    /// @TODO Would it be useful to follow a convention where we know exactly which set layout is at a certain index?
    /// The answer is definitely yes: 0 model, 1 camera, 2 material
    fn get_set_layouts(device: &Device) -> Vec<vk::DescriptorSetLayout>;

    fn get_constants() -> Vec<vk::PushConstantRange> {
        vec![]
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

    fn write_set_model_view(device: &Device, set: vk::DescriptorSet, model_view: &Buffer) {
        // Update immediately the descriptor sets
        let buffer_info = vk::DescriptorBufferInfo::builder()
            .range(std::mem::size_of::<na::Matrix4<f32>>() as vk::DeviceSize)
            .buffer(model_view.buffer)
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

    fn get_color_blend(subpass: u32) -> Vec<vk::PipelineColorBlendAttachmentState> {
        let mut state = vec![vk::PipelineColorBlendAttachmentState::builder()
            .blend_enable(true)
            .color_write_mask(
                vk::ColorComponentFlags::R
                    | vk::ColorComponentFlags::G
                    | vk::ColorComponentFlags::B,
            )
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .color_blend_op(vk::BlendOp::ADD)
            .build()];

        if subpass == 0 {
            state.push(state[0]);
        }

        state
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
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(2)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Point, normal) as u32)
                .build(),
        ]
    }

    fn get_set_layouts(device: &Device) -> Vec<vk::DescriptorSetLayout> {
        let model_bindings = vec![
            vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .build(),
            vk::DescriptorSetLayoutBinding::builder()
                .binding(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .build(),
        ];
        let model = create_set_layout(device, &model_bindings);

        let camera_bindings = Camera::get_set_layout_bindings();
        let camera = create_set_layout(device, &camera_bindings);

        vec![model, camera]
    }
}

impl VertexInput for Line {
    fn get_pipeline() -> Pipelines {
        Pipelines::LINE
    }

    fn get_bindings() -> vk::VertexInputBindingDescription {
        Point::get_bindings()
    }

    fn get_attributes() -> Vec<vk::VertexInputAttributeDescription> {
        Point::get_attributes()
    }

    fn get_set_layouts(device: &Device) -> Vec<vk::DescriptorSetLayout> {
        Point::get_set_layouts(device)
    }
}

pub trait Binding {
    fn get_set_layout_bindings() -> Vec<vk::DescriptorSetLayoutBinding>;
    fn write_set(&self, device: &Device, set: vk::DescriptorSet, frame: &crate::frame::Frame, node: Handle<Node>);
}

impl Binding for Camera {
    fn get_set_layout_bindings() -> Vec<vk::DescriptorSetLayoutBinding> {
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

    fn write_set(
        &self,
        device: &Device,
        set: vk::DescriptorSet,
        frame: &crate::frame::Frame,
        node: Handle<Node>,
    ) {
        let view = frame.res.view_buffers.get(&node).unwrap();

        let handle = Handle::new(self.id);
        let proj = frame.res.proj_buffers.get(&handle).unwrap();

        let view_buffer_info = vk::DescriptorBufferInfo::builder()
            .range(std::mem::size_of::<na::Matrix4<f32>>() as vk::DeviceSize)
            .buffer(view.buffer)
            .build();

        let view_buffer_write = vk::WriteDescriptorSet::builder()
            .dst_set(set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&[view_buffer_info])
            .build();

        let proj_buffer_info = vk::DescriptorBufferInfo::builder()
            .range(std::mem::size_of::<na::Matrix4<f32>>() as vk::DeviceSize)
            .buffer(proj.buffer)
            .build();

        let proj_buffer_write = vk::WriteDescriptorSet::builder()
            .dst_set(set)
            .dst_binding(1)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&[proj_buffer_info])
            .build();

        let writes = vec![view_buffer_write, proj_buffer_write];

        unsafe {
            device.update_descriptor_sets(&writes, &[]);
        }
    }
}


pub fn write_view_set(device: &Device, set: vk::DescriptorSet, view: &Buffer) {
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

pub fn write_proj_set(device: &Device, set: vk::DescriptorSet, proj: &Buffer) {
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

pub fn write_present_set(
    device: &ash::Device,
    set: vk::DescriptorSet,
    albedo: &ImageView,
    normal: &ImageView,
    sampler: &Sampler,
) {
    let image_info = vk::DescriptorImageInfo::builder()
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .image_view(albedo.view)
        .sampler(sampler.sampler)
        .build();

    let image_write = vk::WriteDescriptorSet::builder()
        .dst_set(set)
        .dst_binding(0)
        .dst_array_element(0)
        .descriptor_type(vk::DescriptorType::INPUT_ATTACHMENT)
        .image_info(&[image_info])
        .build();

    let normal_image_info = vk::DescriptorImageInfo::builder()
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .image_view(normal.view)
        .sampler(sampler.sampler)
        .build();

    let normal_image_write = vk::WriteDescriptorSet::builder()
        .dst_set(set)
        .dst_binding(1)
        .dst_array_element(0)
        .descriptor_type(vk::DescriptorType::INPUT_ATTACHMENT)
        .image_info(&[normal_image_info])
        .build();

    let writes = vec![image_write, normal_image_write];

    unsafe {
        device.update_descriptor_sets(&writes, &[]);
    }
}

impl VertexInput for PresentVertex {
    fn get_pipeline() -> Pipelines {
        Pipelines::PRESENT
    }

    fn get_bindings() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    fn get_attributes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            // position
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(offset_of!(Self, pos) as u32)
                .build(),
        ]
    }

    fn get_set_layouts(device: &Device) -> Vec<vk::DescriptorSetLayout> {
        let albedo_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::INPUT_ATTACHMENT)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .build();

        let normal_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(1)
            .descriptor_type(vk::DescriptorType::INPUT_ATTACHMENT)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .build();

        let bindings = vec![albedo_binding, normal_binding];
        let set_layout = create_set_layout(device, &bindings);
        vec![set_layout]
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
        let model_bindings = vec![
            vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .build(),
            vk::DescriptorSetLayoutBinding::builder()
                .binding(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .build(),
        ];
        let model = create_set_layout(device, &model_bindings);

        let camera_bindings = Camera::get_set_layout_bindings();
        let camera = create_set_layout(device, &camera_bindings);

        let material_bindings = Material::get_set_layout_bindings();
        let material = create_set_layout(device, &material_bindings);

        vec![model, camera, material]
    }
}
