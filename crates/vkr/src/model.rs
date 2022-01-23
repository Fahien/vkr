// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::{vk, Device};

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

pub trait Binding {
    fn get_set_layout_bindings() -> Vec<vk::DescriptorSetLayoutBinding>;
    fn write_set(
        &self,
        device: &Device,
        set: vk::DescriptorSet,
        frame: &crate::frame::Frame,
        node: Handle<Node>,
    );
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
