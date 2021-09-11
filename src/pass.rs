// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::rc::Rc;

use ash::{version::DeviceV1_0, vk};

use super::*;

pub struct Pass {
    pub render: vk::RenderPass,
    device: Rc<ash::Device>,
}

impl Pass {
    pub fn new(dev: &mut Dev) -> Self {
        // Render pass (swapchain surface format, device)
        let present_attachment = vk::AttachmentDescription::builder()
            // @todo This format should come from a "framebuffer" object
            .format(dev.surface_format.format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .build();

        let depth_attachment = vk::AttachmentDescription::builder()
            // @todo This format should come from a "framebuffer" object
            .format(vk::Format::D32_SFLOAT)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .build();

        let albedo_attachment = vk::AttachmentDescription::builder()
            // @todo This format should come from a "framebuffer" object
            .format(dev.surface_format.format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .build();

        let normal_attachment = vk::AttachmentDescription::builder()
            .format(vk::Format::A2R10G10B10_UNORM_PACK32)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .build();

        let attachments = [
            present_attachment,
            depth_attachment,
            albedo_attachment,
            normal_attachment,
        ];

        let present_ref = vk::AttachmentReference::builder()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();

        let depth_ref = vk::AttachmentReference::builder()
            .attachment(1)
            .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .build();

        let albedo_ref = vk::AttachmentReference::builder()
            .attachment(2)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();

        let normal_ref = vk::AttachmentReference::builder()
            .attachment(3)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();

        let first_color_refs = [albedo_ref, normal_ref];
        let second_color_refs = [present_ref];

        let depth_input_ref = vk::AttachmentReference::builder()
            .attachment(1)
            .layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .build();

        let albedo_input_ref = vk::AttachmentReference::builder()
            .attachment(2)
            .layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .build();

        let normal_input_ref = vk::AttachmentReference::builder()
            .attachment(3)
            .layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .build();

        let input_refs = [albedo_input_ref, normal_input_ref, depth_input_ref];

        // Two subpasses
        let subpasses = [
            // First subpass writes albedo and depth
            vk::SubpassDescription::builder()
                .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                .color_attachments(&first_color_refs)
                .depth_stencil_attachment(&depth_ref)
                .build(),
            vk::SubpassDescription::builder()
                .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                .color_attachments(&second_color_refs)
                .input_attachments(&input_refs)
                .build(),
        ];

        let init_dependency = vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .build();

        let output_to_input_dependency = vk::SubpassDependency::builder()
            .src_subpass(0)
            .dst_subpass(1)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .dependency_flags(vk::DependencyFlags::BY_REGION)
            .build();

        let present_dependency = vk::SubpassDependency::builder()
            .src_subpass(0)
            .dst_subpass(vk::SUBPASS_EXTERNAL)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(
                vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            )
            .dst_stage_mask(vk::PipelineStageFlags::BOTTOM_OF_PIPE)
            .dst_access_mask(vk::AccessFlags::MEMORY_READ)
            .dependency_flags(vk::DependencyFlags::BY_REGION)
            .build();

        let dependencies = [
            init_dependency,
            output_to_input_dependency,
            present_dependency,
        ];

        // Build the render pass
        let create_info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies)
            .build();
        let render = unsafe { dev.device.create_render_pass(&create_info, None) }
            .expect("Failed to create Vulkan render pass");

        Self {
            render,
            device: Rc::clone(&dev.device),
        }
    }
}

impl Drop for Pass {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_render_pass(self.render, None);
        }
    }
}
