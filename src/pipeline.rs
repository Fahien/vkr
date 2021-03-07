// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{ffi::CString, rc::Rc};

use ash::{vk, Device};
use byteorder::{ByteOrder, NativeEndian};

use crate::{gfx::*, model::*};

pub struct Pipeline {
    pub graphics: vk::Pipeline,
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
        // Pipeline layout (device, shader reflection?)
        let layout = {
            let create_info = vk::PipelineLayoutCreateInfo::builder().build();
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
            let vertex_attribute = T::get_attributes();

            let vertex_binding = [vertex_binding];
            let vertex_attribute = [vertex_attribute];

            let vertex_input = vk::PipelineVertexInputStateCreateInfo::builder()
                .vertex_attribute_descriptions(&vertex_attribute)
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

        unsafe {
            device.destroy_pipeline_layout(layout, None);
        }

        Self {
            graphics,
            device: Rc::clone(&device),
        }
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.graphics, None);
        }
    }
}
