// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{ffi::CString, rc::Rc};

use byteorder::{ByteOrder, NativeEndian};

use vkr_core::Pipeline;
use vkr_core::{dev::Dev, pass::Pass, vertex::Vertex};

use vkr_core::ash::{self, vk};

pub struct MainPipeline {
    pub graphics: vk::Pipeline,
    device: Rc<ash::Device>,
}

impl MainPipeline {
    pub fn new(dev: &mut Dev, pass: &Pass, width: u32, height: u32) -> Self {
        // Pipeline layout (device, shader reflection?)
        let layout = {
            let create_info = vk::PipelineLayoutCreateInfo::builder().build();
            unsafe { dev.device.create_pipeline_layout(&create_info, None) }
                .expect("Failed to create Vulkan pipeline layout")
        };

        // Graphics pipeline (shaders, renderpass)
        let graphics = {
            let frag_spv = include_bytes!("../res/shader/frag.spv");
            let vert_spv = include_bytes!("../res/shader/vert.spv");

            let mut frag_code = vec![0; frag_spv.len() / std::mem::size_of::<u32>()];
            let mut vert_code = vec![0; vert_spv.len() / std::mem::size_of::<u32>()];

            NativeEndian::read_u32_into(vert_spv, vert_code.as_mut_slice());
            NativeEndian::read_u32_into(frag_spv, frag_code.as_mut_slice());

            let create_info = vk::ShaderModuleCreateInfo::builder()
                .code(frag_code.as_slice())
                .build();
            let frag_mod = unsafe { dev.device.create_shader_module(&create_info, None) }
                .expect("Failed to create Vulkan shader module");

            let create_info = vk::ShaderModuleCreateInfo::builder()
                .code(vert_code.as_slice())
                .build();
            let vert_mod = unsafe { dev.device.create_shader_module(&create_info, None) }
                .expect("Failed to create Vulkan shader module");

            let entrypoint = CString::new("main").expect("Failed to create main entrypoint");
            let vert_stage = vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert_mod)
                .name(&entrypoint)
                .build();
            let frag_stage = vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag_mod)
                .name(&entrypoint)
                .build();

            let vertex_binding = vk::VertexInputBindingDescription::builder()
                .binding(0)
                .stride(std::mem::size_of::<Vertex>() as u32)
                .input_rate(vk::VertexInputRate::VERTEX)
                .build();

            let vertex_attribute = vk::VertexInputAttributeDescription::builder()
                .location(0)
                .binding(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(0)
                .build();

            let vertex_binding = [vertex_binding];
            let vertex_attribute = [vertex_attribute];

            let vertex_input = vk::PipelineVertexInputStateCreateInfo::builder()
                .vertex_attribute_descriptions(&vertex_attribute)
                .vertex_binding_descriptions(&vertex_binding)
                .build();

            let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
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
                .color_write_mask(vk::ColorComponentFlags::RGBA)
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
                dev.device
                    .create_graphics_pipelines(vk::PipelineCache::null(), &create_info, None)
            }
            .expect("Failed to create Vulkan graphics pipeline");
            unsafe {
                dev.device.destroy_shader_module(vert_mod, None);
                dev.device.destroy_shader_module(frag_mod, None);
            }
            pipelines[0]
        };

        unsafe {
            dev.device.destroy_pipeline_layout(layout, None);
        }

        Self {
            graphics,
            device: Rc::clone(&dev.device),
        }
    }
}

impl Drop for MainPipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.graphics, None);
        }
    }
}

impl Pipeline for MainPipeline {
    fn get_pipeline(&self) -> vk::Pipeline {
        self.graphics
    }
}
