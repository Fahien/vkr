// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{ffi::CString, rc::Rc};

use ash::{vk, Device};

use crate::{
    gfx::Pass,
    model::{Line, Vertex, VertexInput},
    shader::ShaderModule,
    Descriptors, PresentVertex,
};

use enum_ordinalize::*;
use variant_count::*;

#[derive(Debug, Clone, Copy, VariantCount, Ordinalize)]
pub enum Pipelines {
    LINE,
    MAIN,
    NORMAL,
    PRESENT,
}

/// Collection of built-in pipelines
pub struct DefaultPipelines {
    /// When debug is set, it is used instead of the one requested by a mesh
    pub debug: Option<Pipelines>,
    pub pipelines: [Pipeline; Pipelines::VARIANT_COUNT],
}

impl DefaultPipelines {
    pub fn new(device: &Rc<Device>, pass: &Pass, width: u32, height: u32) -> Self {
        let line = Pipeline::line(device, pass, width, height);
        let main = Pipeline::main(device, pass, width, height);
        let normal = Pipeline::normal(device, pass, width, height);
        let present = Pipeline::present(device, pass, width, height);
        let debug = None;

        let pipelines = [line, main, normal, present];

        Self { debug, pipelines }
    }

    pub fn get<T: VertexInput>(&self) -> &Pipeline {
        match self.debug {
            Some(index) => &self.pipelines[index as usize],
            None => &self.pipelines[T::get_pipeline() as usize],
        }
    }

    pub fn get_mut<T: VertexInput>(&mut self) -> &mut Pipeline {
        match self.debug {
            Some(index) => &mut self.pipelines[index as usize],
            None => &mut self.pipelines[T::get_pipeline() as usize],
        }
    }
}

pub struct PipelineCache {
    /// List of descriptors, one for each swapchain image
    pub descriptors: Descriptors,
}

impl PipelineCache {
    pub fn new(device: &Rc<Device>) -> Self {
        Self {
            descriptors: Descriptors::new(device),
        }
    }
}
pub struct Pipeline {
    pub graphics: vk::Pipeline,
    /// A pipeline layout depends on set layouts, constants, etc, to be created.
    pub layout: vk::PipelineLayout,
    /// Set layouts do not really depend on anything
    pub set_layouts: Vec<vk::DescriptorSetLayout>,
    device: Rc<Device>,
}

impl Pipeline {
    pub fn new<T: VertexInput>(
        device: &Rc<Device>,
        vert: vk::PipelineShaderStageCreateInfo,
        frag: vk::PipelineShaderStageCreateInfo,
        topology: vk::PrimitiveTopology,
        dynamic_state: &vk::PipelineDynamicStateCreateInfo,
        pass: &Pass,
        width: u32,
        height: u32,
        subpass: u32,
    ) -> Self {
        let set_layouts = T::get_set_layouts(device);
        let constants = T::get_constants();

        // Pipeline layout (device, descriptorset layouts, shader reflection?)
        let layout = {
            let create_info = vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&set_layouts)
                .push_constant_ranges(&constants)
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
                .min_depth(1.0)
                .max_depth(0.0)
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

            let depth_state = T::get_depth_state();

            let blend_attachment = T::get_color_blend();

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
                .dynamic_state(&dynamic_state)
                .render_pass(pass.render)
                .subpass(subpass)
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
            set_layouts,
            layout,
            device: device.clone(),
        }
    }

    pub fn line(device: &Rc<Device>, pass: &Pass, width: u32, height: u32) -> Self {
        let shader = ShaderModule::main(device);
        let vs = CString::new("line_vs").expect("Failed to create entrypoint");
        let fs = CString::new("line_fs").expect("Failed to create entrypoint");

        let states = vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder()
            .dynamic_states(&states)
            .build();

        Self::new::<Line>(
            device,
            shader.get_vert(&vs),
            shader.get_frag(&fs),
            vk::PrimitiveTopology::LINE_STRIP,
            &dynamic_state,
            pass,
            width,
            height,
            0,
        )
    }

    pub fn main(device: &Rc<Device>, pass: &Pass, width: u32, height: u32) -> Self {
        let shader = ShaderModule::main(device);
        let vs = CString::new("main_vs").expect("Failed to create entrypoint");
        let fs = CString::new("main_fs").expect("Failed to create entrypoint");

        let states = vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder()
            .dynamic_states(&states)
            .build();

        Self::new::<Vertex>(
            device,
            shader.get_vert(&vs),
            shader.get_frag(&fs),
            vk::PrimitiveTopology::TRIANGLE_LIST,
            &dynamic_state,
            pass,
            width,
            height,
            0,
        )
    }

    /// Returns a graphics pipeline which draws the normals of primitive's surfaces as a color
    pub fn normal(device: &Rc<Device>, pass: &Pass, width: u32, height: u32) -> Self {
        let shader = ShaderModule::main(device);
        let vs = CString::new("main_vs").expect("Failed to create entrypoint");
        let fs = CString::new("normal_fs").expect("Failed to create entrypoint");

        let states = vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder()
            .dynamic_states(&states)
            .build();

        Self::new::<Vertex>(
            device,
            shader.get_vert(&vs),
            shader.get_frag(&fs),
            vk::PrimitiveTopology::TRIANGLE_LIST,
            &dynamic_state,
            pass,
            width,
            height,
            0,
        )
    }

    pub fn present(device: &Rc<Device>, pass: &Pass, width: u32, height: u32) -> Self {
        let shader = ShaderModule::main(device);
        let vs = CString::new("present_vs").expect("Failed to create entrypoint");
        let fs = CString::new("present_fs").expect("Failed to create entry point");

        let dynamic_state = vk::PipelineDynamicStateCreateInfo::default();

        Self::new::<PresentVertex>(
            device,
            shader.get_vert(&vs),
            shader.get_frag(&fs),
            vk::PrimitiveTopology::TRIANGLE_LIST,
            &dynamic_state,
            pass,
            width,
            height,
            1,
        )
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            for set_layout in &self.set_layouts {
                self.device.destroy_descriptor_set_layout(*set_layout, None);
            }
            self.device.destroy_pipeline_layout(self.layout, None);
            self.device.destroy_pipeline(self.graphics, None);
        }
    }
}
