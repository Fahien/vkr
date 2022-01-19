// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashSet;

use crate::{shader::UniformType, Camelcase, CrateModule, Pipeline, Uniform};

pub fn header() -> TokenStream {
    quote! {
        use std::{collections::HashMap, ffi::CString, rc::Rc};
        use ash::{vk, Device};
        use vkr_core::{Dev, Pass, ShaderModule, Pipeline, Texture, Frame, Model, Node};
        use vkr_util::Handle;
    }
}

pub fn set_layout_bindings(uniforms: &[Uniform], set: u32) -> TokenStream {
    let mut gen = quote! {};

    let set_uniforms = uniforms
        .iter()
        .filter(|u| matches!(u.descriptor_set, Some(s) if s == set));

    for uniform in set_uniforms {
        let binding = uniform.binding;
        let descriptor_type = uniform.get_descriptor_type();
        let stage = uniform.stage;
        gen.extend(quote! {
            vk::DescriptorSetLayoutBinding::builder()
                .binding(#binding)
                .descriptor_type(#descriptor_type)
                .descriptor_count(1) // Count what?
                .stage_flags(#stage)
                .build(),
        });
    }

    gen
}

pub fn push_constant_ranges(uniforms: &[Uniform]) -> TokenStream {
    let mut gen = quote! {};

    let set_uniforms = uniforms
        .iter()
        .filter(|u| u.uniform_type == UniformType::PushConstant);

    for uniform in set_uniforms {
        let range = uniform
            .get_range()
            .expect("Failed to get push constant range");
        let stage = uniform.stage;
        gen.extend(quote! {
            vk::PushConstantRange::builder()
            .offset(0)
            .stage_flags(#stage)
            .size(#range as u32)
            .build(),
        });
    }

    gen
}

fn get_sorted_sets(uniforms: &[Uniform]) -> Vec<u32> {
    let sets: HashSet<_> = uniforms.iter().filter_map(|u| u.descriptor_set).collect();
    let mut sets: Vec<_> = sets.into_iter().collect();
    sets.sort();
    sets
}

pub fn set_layouts_methods(uniforms: &[Uniform]) -> TokenStream {
    let mut gen = quote! {
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
    };

    let mut set_layouts = quote! {};
    for set in get_sorted_sets(uniforms) {
        let bindings = set_layout_bindings(uniforms, set);
        set_layouts.extend(quote! {
            Self::create_set_layout(
                device,
                &[
                    #bindings
                ]
            ),
        })
    }

    gen.extend(quote! {
        pub fn new_set_layouts(device: &Device) -> Vec<vk::DescriptorSetLayout> {
            vec![
                #set_layouts
            ]
        }
    });

    gen
}

pub fn write_set_methods(uniforms: &[Uniform]) -> TokenStream {
    let mut gen = quote! {};

    for set in get_sorted_sets(uniforms) {
        let set_uniforms = uniforms
            .iter()
            .filter(|u| matches!(u.descriptor_set, Some(s) if s == set));
        let mut writes = quote! {};

        for uniform in set_uniforms {
            let binding = uniform.binding;
            let descriptor_type = uniform.get_descriptor_type();
            let info = uniform.get_info();
            writes.extend(quote! {
                vk::WriteDescriptorSet::builder()
                    .dst_set(set)
                    .dst_binding(#binding)
                    .dst_array_element(0)
                    .descriptor_type(#descriptor_type)
                    #info
                    .build(),
            });
        }

        let args = uniforms.iter().filter_map(|u| {
            if matches! (u.descriptor_set, Some(s) if s == set) {
                let arg = format!("{}: {}", u.name, u.get_write_set_type())
                    .parse::<proc_macro2::TokenStream>()
                    .unwrap();
                return Some(arg);
            }
            None
        });

        let arguments = quote! {
            &self,
            set: vk::DescriptorSet
            #( ,#args )*
        };

        let write_set_sign = format!("write_set_{}", set)
            .parse::<proc_macro2::TokenStream>()
            .unwrap();
        gen.extend(quote! {
            /// We do not know whether descriptor sets are allocated together and stored in a vector
            /// or they are allocated one by one, therefore we just expect one descriptor set here.
            pub fn #write_set_sign(
                #arguments
            ) {
                // TODO: calculate range by looking at shader argument and assert buffer size >= range
                let writes = [
                    #writes
                ];

                unsafe {
                    self.device.update_descriptor_sets(&writes, &[]);
                }
            }
        });
    }

    gen
}

pub fn pipeline(pipeline: &Pipeline) -> TokenStream {
    let pipeline_name = format!("Pipeline{}", pipeline.name.to_camelcase())
        .parse::<proc_macro2::TokenStream>()
        .expect("Failed to parse shader name");

    let pipeline_str = pipeline.name.to_camelcase();

    let vs = format!("{}_vs", pipeline.name.to_lowercase());
    let fs = format!("{}_fs", pipeline.name.to_lowercase());

    let pipeline_cache_name = format!("PipelineCache{}", pipeline.name.to_camelcase())
        .parse::<proc_macro2::TokenStream>()
        .expect("Failed to parse shader name");

    let pipeline_cache = quote! {
        pub struct #pipeline_cache_name {
            sets: HashMap<usize, Vec<vk::DescriptorSet>>,
            pool: vk::DescriptorPool,
            pub device: Rc<Device>,
        }

        impl #pipeline_cache_name {
            pub fn new(device: &Rc<Device>) -> Self {
                let pool = unsafe {
                    // Support 1 model matrix, 1 view matrix, 1 proj matrix?
                    let uniform_count = 32;
                    let uniform_pool_size = vk::DescriptorPoolSize::builder()
                        .descriptor_count(uniform_count)
                        .ty(vk::DescriptorType::UNIFORM_BUFFER)
                        .build();

                    // Support 1 material and a gui texture?
                    let sampler_count = 16;
                    let sampler_pool_size = vk::DescriptorPoolSize::builder()
                        .descriptor_count(sampler_count)
                        .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .build();

                    // Support 3 input attachments
                    let input_count = 3;
                    let input_pool_size = vk::DescriptorPoolSize::builder()
                        .descriptor_count(input_count)
                        .ty(vk::DescriptorType::INPUT_ATTACHMENT)
                        .build();

                    let set_count = 16; // 5 nodes, 1 camera, 5 materials, 1 gui?
                    let pool_sizes = vec![uniform_pool_size, sampler_pool_size, input_pool_size];

                    let create_info = vk::DescriptorPoolCreateInfo::builder()
                        .pool_sizes(&pool_sizes)
                        .max_sets(set_count)
                        .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
                        .build();

                    device.create_descriptor_pool(&create_info, None)
                        .expect("Failed to create Vulkan descriptor pool")
                };

                Self {
                    sets: HashMap::new(),
                    pool,
                    device: device.clone(),
                }
            }

            pub fn allocate(&mut self, layouts: &[vk::DescriptorSetLayout]) -> Vec<vk::DescriptorSet> {
                let create_info = vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(self.pool)
                    .set_layouts(layouts)
                    .build();

                unsafe { self.device.allocate_descriptor_sets(&create_info) }
                    .expect("Failed to allocate Vulkan descriptor sets")
            }

            pub fn free(&self, descriptors: &[vk::DescriptorSet]) {
                unsafe {
                    self.device
                        .free_descriptor_sets(self.pool, descriptors)
                        .expect("msFailed to free descriptor sets");
                }
            }
        }

        impl Drop for #pipeline_cache_name {
            fn drop(&mut self) {
                unsafe { self.device.destroy_descriptor_pool(self.pool, None) };
            }
        }
    };

    let set_layouts_methods = set_layouts_methods(&pipeline.uniforms);
    let push_constants = push_constant_ranges(&pipeline.uniforms);
    let write_set_methods = write_set_methods(&pipeline.uniforms);

    quote! {
        #pipeline_cache

        pub struct #pipeline_name {
            caches: Vec<#pipeline_cache_name>,
            pipeline: vk::Pipeline,
            layout: vk::PipelineLayout,
            set_layouts: Vec<vk::DescriptorSetLayout>,
            device: Rc<Device>,
            name: String,
        }

        impl #pipeline_name {
            #set_layouts_methods

            pub fn new_layout(device: &Rc<Device>, set_layouts: &[vk::DescriptorSetLayout]) -> vk::PipelineLayout {
                let push_constants = [
                    #push_constants
                ];

                let mut create_info = vk::PipelineLayoutCreateInfo::builder()
                    .set_layouts(set_layouts);
                if push_constants.len() > 0 {
                    create_info = create_info.
                    push_constant_ranges(&push_constants);
                }
                let create_info = create_info.build();
                let layout = unsafe { device.create_pipeline_layout(&create_info, None) };
                layout.expect("Failed to create Vulkan pipeline layout")
            }

            pub fn new_impl<V: VertexInput>(layout: vk::PipelineLayout, shader_module: &ShaderModule, vs: &str, fs: &str, render_pass: vk::RenderPass, subpass: u32) -> vk::Pipeline {
                let vs_entry = CString::new(vs).expect("Failed to create vertex entry point");
                let fs_entry = CString::new(fs).expect("Failed to create vertex entry point");

                let stages = [
                    shader_module.get_vert(&vs_entry),
                    shader_module.get_frag(&fs_entry)
                ];

                let vertex_bindings = V::get_bindings();
                let vertex_attributes = V::get_attributes();
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

                let mut blend_attachments = vec![
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
                        .build()];

                if subpass == 0 {
                    blend_attachments.push(blend_attachments[0]);
                }

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
                    .subpass(subpass)
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

            pub fn new<V: VertexInput>(shader_module: &ShaderModule, render_pass: vk::RenderPass, subpass: u32) -> Self {
                let name = String::from(#pipeline_str);
                let device = shader_module.device.clone();
                let set_layouts = Self::new_set_layouts(&shader_module.device);
                let layout = Self::new_layout(&shader_module.device, &set_layouts);
                let pipeline = Self::new_impl::<V>(layout, shader_module, #vs, #fs, render_pass, subpass);

                Self {
                    caches: vec![],
                    pipeline,
                    layout,
                    set_layouts,
                    device,
                    name
                }
            }

            pub fn get_cache(&mut self, index: usize) -> &mut #pipeline_cache_name {
                while index >= self.caches.len() {
                    self.caches.push(#pipeline_cache_name::new(&self.device));
                }

                &mut self.caches[index]
            }

            #write_set_methods
        }

        impl Pipeline for #pipeline_name {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }

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

            fn bind(&self, frame: &mut Frame, model: &Model, node: Handle<Node>) {
                self.bind_impl(frame, model, node)
            }

            fn draw(&self, frame: &mut Frame, model: &Model, node: Handle<Node>) {
                self.draw_impl(frame, model, node)
            }
        }

        impl Drop for #pipeline_name {
            fn drop(&mut self) {
                unsafe {
                    self.device.destroy_pipeline(self.pipeline, None);
                    self.device.destroy_pipeline_layout(self.layout, None);
                    for set_layout in &self.set_layouts {
                        self.device.destroy_descriptor_set_layout(*set_layout, None);
                    }
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
                Box::new(Pipeline{1}::new::<V>(shader_module, render_pass, subpass))
            }}",
            crate_module.name.to_camelcase(),
            m.name.to_camelcase(),
        )
        .parse::<TokenStream>()
        .expect("Failed to parse shader name")
    });

    let pipeline_count = pipelines.len();

    let max_subpasses = 4usize;
    let nones = (0..max_subpasses - 1).fold("None".to_string(), |acc, _| format!("{}, None", acc));
    let pipeline_init = pipelines.iter().map(|_| {
        format!("[{}]", nones)
            .parse::<TokenStream>()
            .expect("Failed to parse shader name")
    });

    quote! {
        #[derive(Copy,Clone,Debug)]
        pub enum #enum_name {
            #( #pipeline_names, )*
        }

        impl #enum_name {
            fn create_pipeline<V: VertexInput>(&self, shader_module: &ShaderModule, render_pass: vk::RenderPass, subpass: u32) -> Box<dyn Pipeline> {
                match self {
                    #( #pipeline_new, )*
                }
            }
        }

        pub struct PipelinePool {
            pass: Pass,
            // Each entry in the array is a vector, where each vector position corresponds to the subpass index
            pipelines: [[Option<Box<dyn Pipeline>>;#max_subpasses];#pipeline_count],
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

            fn create_pipeline<V: VertexInput>(&mut self, shader: #enum_name, subpass: u32) {
                assert!(self.pipelines[shader as usize][subpass as usize].is_none());

                let render_pass = self.pass.render;
                let shader_module = self.get_shader_module();
                let pipeline = shader.create_pipeline::<V>(shader_module, render_pass, subpass);
                self.pipelines[shader as usize][subpass as usize] = Some(pipeline);
            }

            pub fn get<V: VertexInput>(&mut self, shader: #enum_name, subpass: u32) -> &Box<dyn Pipeline> {
                if self.pipelines[shader as usize][subpass as usize].is_none() {
                    self.create_pipeline::<V>(shader, subpass)
                }

                self.pipelines[shader as usize][subpass as usize].as_ref().unwrap()
            }

            pub fn get_mut<V: VertexInput>(&mut self, shader: #enum_name, subpass: u32) -> &mut Box<dyn Pipeline> {
                if self.pipelines[shader as usize][subpass as usize].is_none() {
                    self.create_pipeline::<V>(shader, subpass)
                }

                self.pipelines[shader as usize][subpass as usize].as_mut().unwrap()
            }
        }
    }
}
