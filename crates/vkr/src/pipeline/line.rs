// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{ffi::CString, rc::Rc};

use memoffset::offset_of;
use vkr_core::{Dev, Pass, Pipeline, Point3, ShaderModule};

use vkr_core::ash::{self, vk};

pub struct LinePipeline {
    pub graphics: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub set_layout: vk::DescriptorSetLayout,
    device: Rc<ash::Device>,
}

impl LinePipeline {
    fn get_bindings() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Point3>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    fn get_attributes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Point3, pos) as u32)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(offset_of!(Point3, color) as u32)
                .build(),
        ]
    }

    pub fn new(dev: &mut Dev, pass: &Pass, width: u32, height: u32) -> Self {
        let set_layout_bindings = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER) // delta time?
            .descriptor_count(1) // Referring the shader
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .build();
        let arr_bindings = vec![set_layout_bindings];

        let set_layout_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&arr_bindings);

        let set_layout = unsafe {
            dev.device
                .create_descriptor_set_layout(&set_layout_info, None)
        }
        .expect("Failed to create Vulkan descriptor set layout");

        let set_layouts = vec![set_layout];

        // Pipeline layout (device, descriptorset layouts, shader reflection?)
        let layout = {
            let create_info = vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&set_layouts)
                .build();
            unsafe { dev.device.create_pipeline_layout(&create_info, None) }
                .expect("Failed to create Vulkan pipeline layout")
        };

        // Graphics pipeline (shaders, renderpass)
        let graphics = {
            let frag_mod = ShaderModule::new(
                &dev.device,
                include_bytes!("../../res/shader/line.frag.spv"),
            );

            let vert_mod = ShaderModule::new(
                &dev.device,
                include_bytes!("../../res/shader/line.vert.spv"),
            );

            let entrypoint = CString::new("main").expect("Failed to create main entrypoint");
            let vert_stage = vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert_mod.shader)
                .name(&entrypoint)
                .build();
            let frag_stage = vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag_mod.shader)
                .name(&entrypoint)
                .build();

            let vertex_binding = Self::get_bindings();
            let vertex_attributes = Self::get_attributes();

            let vertex_binding = [vertex_binding];

            let vertex_input = vk::PipelineVertexInputStateCreateInfo::builder()
                .vertex_attribute_descriptions(&vertex_attributes)
                .vertex_binding_descriptions(&vertex_binding)
                .build();

            let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
                .topology(vk::PrimitiveTopology::LINE_STRIP)
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

            pipelines[0]
        };

        Self {
            graphics,
            layout,
            set_layout,
            device: Rc::clone(&dev.device),
        }
    }
}

impl Drop for LinePipeline {
    fn drop(&mut self) {
        unsafe {
            self.device
                .destroy_descriptor_set_layout(self.set_layout, None);
            self.device.destroy_pipeline_layout(self.layout, None);
            self.device.destroy_pipeline(self.graphics, None);
        }
    }
}

impl Pipeline for LinePipeline {
    fn get_pipeline(&self) -> vk::Pipeline {
        self.graphics
    }

    fn get_layout(&self) -> vk::PipelineLayout {
        self.layout
    }

    fn get_set_layout(&self) -> vk::DescriptorSetLayout {
        self.set_layout
    }
}
