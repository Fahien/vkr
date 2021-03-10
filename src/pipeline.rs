// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{ffi::CString, rc::Rc};

use ash::{vk, Device};
use byteorder::{ByteOrder, NativeEndian};

use crate::{gfx::Pass, model::VertexInput, Descriptors};

pub struct PipelineCache {
    /// List of descriptors, one for each swapchain image
    pub descriptors: Descriptors,
}

impl PipelineCache {
    pub fn new(device: &Rc<Device>) -> Self {
        Self {
            descriptors: Descriptors::new(device)
        }
    }
}

pub struct Pipeline {
    pub graphics: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub set_layout: vk::DescriptorSetLayout,
    device: Rc<Device>,
}

impl Pipeline {
    pub fn new<T: VertexInput>(
        device: &Rc<Device>,
        topology: vk::PrimitiveTopology,
        pass: &Pass,
        width: u32,
        height: u32,
    ) -> Self {
        let set_layout_bindings = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER) // delta time?
            .descriptor_count(1) // Referring the shader
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .build();
        let arr_bindings = vec![set_layout_bindings];

        let set_layout_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&arr_bindings);

        let set_layout = unsafe { device.create_descriptor_set_layout(&set_layout_info, None) }
            .expect("Failed to create Vulkan descriptor set layout");

        let set_layouts = vec![set_layout];

        // Pipeline layout (device, descriptorset layouts, shader reflection?)
        let layout = {
            let create_info = vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&set_layouts)
                .build();
            unsafe { device.create_pipeline_layout(&create_info, None) }
                .expect("Failed to create Vulkan pipeline layout")
        };

        // Graphics pipeline (shaders, renderpass)
        let graphics = {
            const SHADERS: &[u8] = include_bytes!(env!("vkr_shaders.spv"));
            let mut rs_code = vec![0; SHADERS.len() / std::mem::size_of::<u32>()];
            NativeEndian::read_u32_into(SHADERS, rs_code.as_mut_slice());

            let create_info = vk::ShaderModuleCreateInfo::builder()
                .code(rs_code.as_slice())
                .build();
            let rs_mod = unsafe { device.create_shader_module(&create_info, None) }
                .expect("Failed to create Vulkan shader module");

            let entrypoint = CString::new("main_vs").expect("Failed to create main entrypoint");
            let vert_stage = vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(rs_mod)
                .name(&entrypoint)
                .build();
            let entrypoint = CString::new("main_fs").expect("Failed to create main entrypoint");
            let frag_stage = vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(rs_mod)
                .name(&entrypoint)
                .build();

            let vertex_binding = T::get_bindings();
            let vertex_attributes = T::get_attributes();

            let vertex_binding = [vertex_binding];

            let vertex_input = vk::PipelineVertexInputStateCreateInfo::builder()
                .vertex_attribute_descriptions(&vertex_attributes)
                .vertex_binding_descriptions(&vertex_binding)
                .build();

            let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
                .topology(topology)
                .primitive_restart_enable(false)
                .build();

            let raster_state = vk::PipelineRasterizationStateCreateInfo::builder()
                .depth_clamp_enable(false)
                .rasterizer_discard_enable(false)
                .polygon_mode(vk::PolygonMode::FILL)
                .cull_mode(vk::CullModeFlags::NONE)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                .depth_bias_enable(false)
                .line_width(1.0)
                .build();

            let viewport = [vk::Viewport::builder()
                .x(0.0)
                .y(0.0)
                .width(width as f32)
                .height(height as f32)
                .min_depth(0.0)
                .max_depth(1.0)
                .build()];

            let scissor = [vk::Rect2D::builder()
                .offset(vk::Offset2D::builder().x(0).y(0).build())
                .extent(vk::Extent2D::builder().width(width).height(height).build())
                .build()];

            let view_state = vk::PipelineViewportStateCreateInfo::builder()
                .viewports(&viewport)
                .scissors(&scissor)
                .build();

            let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
                .rasterization_samples(vk::SampleCountFlags::TYPE_1)
                .sample_shading_enable(false)
                .alpha_to_coverage_enable(false)
                .alpha_to_one_enable(false)
                .build();

            let blend_attachment = [vk::PipelineColorBlendAttachmentState::builder()
                .blend_enable(false)
                .color_write_mask(vk::ColorComponentFlags::all())
                .build()];

            let blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
                .logic_op_enable(false)
                .attachments(&blend_attachment)
                .build();

            let stages = [vert_stage, frag_stage];

            let create_info = [vk::GraphicsPipelineCreateInfo::builder()
                .stages(&stages)
                .vertex_input_state(&vertex_input)
                .input_assembly_state(&input_assembly)
                .viewport_state(&view_state)
                .rasterization_state(&raster_state)
                .multisample_state(&multisample_state)
                .color_blend_state(&blend_state)
                .render_pass(pass.render)
                .subpass(0)
                .layout(layout)
                .build()];
            let pipelines = unsafe {
                device.create_graphics_pipelines(vk::PipelineCache::null(), &create_info, None)
            }
            .expect("Failed to create Vulkan graphics pipeline");
            unsafe {
                device.destroy_shader_module(rs_mod, None);
            }
            pipelines[0]
        };

        Self {
            graphics,
            set_layout,
            layout,
            device: device.clone(),
        }
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            self.device
                .destroy_descriptor_set_layout(self.set_layout, None);
            self.device.destroy_pipeline_layout(self.layout, None);
            self.device.destroy_pipeline(self.graphics, None);
        }
    }
}