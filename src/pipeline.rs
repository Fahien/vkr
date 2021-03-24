// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{ffi::CString, rc::Rc};

use ash::{vk, Device};

use crate::{
    gfx::Pass,
    model::{Line, Vertex, VertexInput},
    shader::ShaderModule,
    Descriptors,
};

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
        vert: vk::PipelineShaderStageCreateInfo,
        frag: vk::PipelineShaderStageCreateInfo,
        topology: vk::PrimitiveTopology,
        pass: &Pass,
        width: u32,
        height: u32,
    ) -> Self {
        let layout_bindings = T::get_set_layout_bindings();
        let set_layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&layout_bindings)
            .build();

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

            let depth_state = vk::PipelineDepthStencilStateCreateInfo::builder()
                .depth_test_enable(true)
                .depth_write_enable(true)
                .depth_compare_op(vk::CompareOp::LESS)
                .depth_bounds_test_enable(false)
                .stencil_test_enable(false)
                .build();

            let blend_attachment = [vk::PipelineColorBlendAttachmentState::builder()
                .blend_enable(false)
                .color_write_mask(vk::ColorComponentFlags::all())
                .build()];

            let blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
                .logic_op_enable(false)
                .attachments(&blend_attachment)
                .build();

            let stages = [vert, frag];

            let create_info = [vk::GraphicsPipelineCreateInfo::builder()
                .stages(&stages)
                .vertex_input_state(&vertex_input)
                .input_assembly_state(&input_assembly)
                .viewport_state(&view_state)
                .rasterization_state(&raster_state)
                .multisample_state(&multisample_state)
                .depth_stencil_state(&depth_state)
                .color_blend_state(&blend_state)
                .render_pass(pass.render)
                .subpass(0)
                .layout(layout)
                .build()];

            let pipelines = unsafe {
                device.create_graphics_pipelines(vk::PipelineCache::null(), &create_info, None)
            }
            .expect("Failed to create Vulkan graphics pipeline");
            pipelines[0]
        };

        Self {
            graphics,
            set_layout,
            layout,
            device: device.clone(),
        }
    }

    pub fn line(device: &Rc<Device>, pass: &Pass, width: u32, height: u32) -> Self {
        let shader = ShaderModule::new(device);
        let vs = CString::new("line_vs").expect("Failed to create entrypoint");
        let fs = CString::new("line_fs").expect("Failed to create entrypoint");
        Self::new::<Line>(
            device,
            shader.get_vert(&vs),
            shader.get_frag(&fs),
            vk::PrimitiveTopology::LINE_STRIP,
            pass,
            width,
            height,
        )
    }

    pub fn main(device: &Rc<Device>, pass: &Pass, width: u32, height: u32) -> Self {
        let shader = ShaderModule::new(device);
        let vs = CString::new("main_vs").expect("Failed to create entrypoint");
        let fs = CString::new("main_fs").expect("Failed to create entrypoint");
        Self::new::<Vertex>(
            device,
            shader.get_vert(&vs),
            shader.get_frag(&fs),
            vk::PrimitiveTopology::TRIANGLE_LIST,
            pass,
            width,
            height,
        )
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

