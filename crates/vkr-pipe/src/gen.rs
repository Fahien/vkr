// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use crate::{Camelcase, CrateModule, Pipeline};
use proc_macro2::TokenStream;
use quote::quote;

pub fn header() -> TokenStream {
    quote! {
        use std::{ffi::CString, rc::Rc};
        use ash::{vk, Device};
        use vkr_core::{Dev, Pass, ShaderModule, Pipeline};
    }
}

fn get_format(arg_type: &syn::Ident) -> TokenStream {
    if arg_type == "Vec3" {
        return quote! { vk::Format::R32G32B32_SFLOAT };
    }
    todo!("Failed to get format for: {}", arg_type);
}

pub fn pipeline(pipeline: &Pipeline) -> TokenStream {
    let pipeline_name = format!("Pipeline{}", pipeline.name.to_camelcase())
        .parse::<proc_macro2::TokenStream>()
        .expect("Failed to parse shader name");

    let pipeline_str = pipeline.name.to_camelcase();

    let vs = format!("{}_vs", pipeline.name.to_lowercase());
    let fs = format!("{}_fs", pipeline.name.to_lowercase());

    // Generate bindings
    let mut vertex_attributes = TokenStream::new();

    for (loc, arg_type) in pipeline.arg_types.iter().enumerate() {
        let format = get_format(arg_type);

        let attribute = quote! {
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(#loc as u32)
                .format(#format)
                .offset(0)
                .build(),
        };

        vertex_attributes.extend(attribute);
    }

    quote! {
        pub struct #pipeline_name {
            layout: vk::PipelineLayout,
            pipeline: vk::Pipeline,
            device: Rc<Device>,
            name: String,
        }

        impl #pipeline_name {
            pub fn new_layout(device: &Rc<Device>) -> vk::PipelineLayout {
                let create_info = vk::PipelineLayoutCreateInfo::builder().build();
                let layout = unsafe { device.create_pipeline_layout(&create_info, None) };
                layout.expect("Failed to create Vulkan pipeline layout")
            }

            pub fn new_impl(layout: vk::PipelineLayout, shader_module: &ShaderModule, vs: &str, fs: &str, render_pass: vk::RenderPass) -> vk::Pipeline {
                let vs_entry = CString::new(vs).expect("Failed to create vertex entry point");
                let fs_entry = CString::new(fs).expect("Failed to create vertex entry point");

                let stages = [
                    shader_module.get_vert(&vs_entry),
                    shader_module.get_frag(&fs_entry)
                ];

                let vertex_bindings = [
                    vk::VertexInputBindingDescription::builder()
                        .binding(0)
                        .stride(std::mem::size_of::<[f32;3]>() as u32)
                        .input_rate(vk::VertexInputRate::VERTEX)
                        .build()
                ];
                let vertex_attributes = [
                    #vertex_attributes
                ];
                let vertex_input = vk::PipelineVertexInputStateCreateInfo::builder()
                    .vertex_attribute_descriptions(&vertex_attributes)
                    .vertex_binding_descriptions(&vertex_bindings)
                    .build();

                let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
                    .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
                    .primitive_restart_enable(false)
                    .build();

                let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::builder()
                    .depth_test_enable(true)
                    .depth_write_enable(true)
                    .depth_compare_op(vk::CompareOp::GREATER)
                    .depth_bounds_test_enable(false)
                    .stencil_test_enable(false)
                    .build();

                let rasterization = vk::PipelineRasterizationStateCreateInfo::builder()
                    .line_width(1.0)
                    .depth_clamp_enable(false)
                    .rasterizer_discard_enable(false)
                    .polygon_mode(vk::PolygonMode::FILL)
                    .cull_mode(vk::CullModeFlags::NONE)
                    .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                    .depth_bias_enable(false)
                    .build();

                // Pass as input? Or just use a default value.
                let width = 1920;
                let height = 1080;

                let viewport = vk::Viewport::builder()
                    .x(0.0)
                    .y(0.0)
                    .width(width as f32)
                    .height(height as f32)
                    .min_depth(1.0) // TODO: 1.0 is near?
                    .max_depth(0.0) // 0.0 is far?
                    .build();

                let scissor = vk::Rect2D::builder()
                    .offset(vk::Offset2D::builder().x(0).y(0).build())
                    .extent(vk::Extent2D::builder().width(width).height(height).build())
                    .build();

                let view = vk::PipelineViewportStateCreateInfo::builder()
                    .viewports(&[viewport])
                    .scissors(&[scissor])
                    .build();

                let multisample = vk::PipelineMultisampleStateCreateInfo::builder()
                    .rasterization_samples(vk::SampleCountFlags::TYPE_1)
                    .sample_shading_enable(false)
                    .alpha_to_coverage_enable(false)
                    .alpha_to_one_enable(false)
                    .build();

                let blend_attachments = [
                    vk::PipelineColorBlendAttachmentState::builder()
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
                        .build(),
                    vk::PipelineColorBlendAttachmentState::builder()
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
                        .build()
                ];

                let blend = vk::PipelineColorBlendStateCreateInfo::builder()
                    .logic_op_enable(false)
                    .attachments(&blend_attachments)
                    .build();

                let states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
                let dynamics = vk::PipelineDynamicStateCreateInfo::builder()
                    .dynamic_states(&states)
                    .build();

                let create_info = vk::GraphicsPipelineCreateInfo::builder()
                    .stages(&stages)
                    .layout(layout)
                    .render_pass(render_pass)
                    .subpass(0)
                    .vertex_input_state(&vertex_input)
                    .input_assembly_state(&input_assembly)
                    .depth_stencil_state(&depth_stencil)
                    .rasterization_state(&rasterization)
                    .viewport_state(&view)
                    .multisample_state(&multisample)
                    .color_blend_state(&blend)
                    .dynamic_state(&dynamics)
                    .build();

                let pipelines = unsafe { shader_module.device.create_graphics_pipelines(vk::PipelineCache::null(), &[create_info], None) };
                let mut pipelines = pipelines.expect("Failed to create Vulkan graphics pipeline");
                let pipeline = pipelines.pop().expect("Failed to pop Vulkan pipeline");

                pipeline
            }

            pub fn new(shader_module: &ShaderModule, render_pass: vk::RenderPass) -> Self {
                let name = String::from(#pipeline_str);
                let device = shader_module.device.clone();
                let layout = Self::new_layout(&shader_module.device);
                let pipeline = Self::new_impl(layout, shader_module, #vs, #fs, render_pass);

                Self {
                    layout,
                    pipeline,
                    device,
                    name
                }
            }
        }

        impl Pipeline for #pipeline_name {
            fn get_name(&self) -> &String {
                &self.name
            }

            fn get_set_layouts(&self) -> &[vk::DescriptorSetLayout] {
                &self.set_layouts
            }

            fn get_layout(&self) -> vk::PipelineLayout {
                self.layout
            }

            fn get_pipeline(&self) -> vk::Pipeline {
                self.pipeline
            }
        }

        impl Drop for #pipeline_name {
            fn drop(&mut self) {
                unsafe {
                    self.device.destroy_pipeline_layout(self.layout, None);
                    self.device.destroy_pipeline(self.pipeline, None);
                }
            }
        }
    }
}

pub fn cache(crate_module: &CrateModule, pipelines: &[Pipeline]) -> TokenStream {
    let enum_name: proc_macro2::TokenStream = format!("Shader{}", crate_module.name.to_camelcase())
        .parse()
        .unwrap();

    let shader_spv = format!("{}.spv", crate_module.name.replace('-', "_"));

    let pipeline_names = pipelines.iter().map(|m| {
        m.name
            .to_camelcase()
            .parse::<TokenStream>()
            .expect("Failed to parse shader name")
    });

    let pipeline_new = pipelines.iter().map(|m| {
        format!(
            "Shader{0}::{1} => {{
                Box::new(Pipeline{1}::new(shader_module, render_pass))
            }}",
            crate_module.name.to_camelcase(),
            m.name.to_camelcase(),
        )
        .parse::<TokenStream>()
        .expect("Failed to parse shader name")
    });

    let pipeline_count = pipelines.len();

    let pipeline_init = pipelines.iter().map(|_| {
        "None"
            .parse::<TokenStream>()
            .expect("Failed to parse shader name")
    });

    quote! {
        #[derive(Copy,Clone,Debug)]
        pub enum #enum_name {
            #( #pipeline_names, )*
        }

        impl #enum_name {
            fn create_pipeline(&self, shader_module: &ShaderModule, render_pass: vk::RenderPass) -> Box<dyn Pipeline> {
                match self {
                    #( #pipeline_new, )*
                }
            }
        }

        pub struct PipelinePool {
            pass: Pass,
            pipelines: [Option<Box<dyn Pipeline>>;#pipeline_count],
            shader_module: Option<ShaderModule>,
            device: Rc<Device>,
        }

        impl PipelinePool {
            /// Returns an empty pipeline cache
            pub fn new(dev: &Dev) -> Self {
                let shader_module = None;

                let pipelines = [
                    #( #pipeline_init, )*
                ];

                let pass = Pass::new(dev);

                Self {
                    pass,
                    pipelines,
                    shader_module,
                    device: dev.device.clone(),
                }
            }

            fn get_shader_module(&mut self) -> &ShaderModule {
                if self.shader_module.is_none() {
                    const CODE: &[u8] = include_bytes!(env!(#shader_spv));
                    self.shader_module = Some(ShaderModule::new(&self.device, CODE));
                }

                self.shader_module.as_ref().unwrap()
            }

            fn create_pipeline(&mut self, shader: #enum_name) {
                assert!(self.pipelines[shader as usize].is_none());

                let render_pass = self.pass.render;
                let shader_module = self.get_shader_module();
                let pipeline = shader.create_pipeline(shader_module, render_pass);
                self.pipelines[shader as usize] = Some(pipeline);
            }

            pub fn get(&mut self, shader: #enum_name) -> &Box<dyn Pipeline> {
                if self.pipelines[shader as usize].is_none() {
                    self.create_pipeline(shader)
                }

                self.pipelines[shader as usize].as_ref().unwrap()
            }
        }
    }
}
