use std::error::Error;

use proc_macro2::{Literal, TokenStream};
use quote::quote;
use spirv_reflect::{types::ReflectDescriptorType, ShaderModule};

//macro_rules! p {
//    ($($tokens: tt)*) => {
//        println!("cargo:warning={}", format!($($tokens)*))
//    }
//}

fn descriptor_type_to_tokens(descriptor_type: ReflectDescriptorType) -> TokenStream {
    match descriptor_type {
        ReflectDescriptorType::Undefined => todo!(),
        ReflectDescriptorType::Sampler => todo!(),
        ReflectDescriptorType::CombinedImageSampler => todo!(),
        ReflectDescriptorType::SampledImage => todo!(),
        ReflectDescriptorType::StorageImage => todo!(),
        ReflectDescriptorType::UniformTexelBuffer => todo!(),
        ReflectDescriptorType::StorageTexelBuffer => todo!(),
        ReflectDescriptorType::UniformBuffer => quote! { vk::DescriptorType::UNIFORM_BUFFER },
        ReflectDescriptorType::StorageBuffer => todo!(),
        ReflectDescriptorType::UniformBufferDynamic => todo!(),
        ReflectDescriptorType::StorageBufferDynamic => todo!(),
        ReflectDescriptorType::InputAttachment => todo!(),
        ReflectDescriptorType::AccelerationStructureNV => todo!(),
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

pub fn get_pipeline_cache_template(
    pipeline_cache_name: &TokenStream,
) -> Result<TokenStream, Box<dyn Error>> {
    let pipeline_cache_code = quote! {
        pub struct #pipeline_cache_name {
            sets: HashMap<usize, Vec<vk::DescriptorSet>>,
            pool: vk::DescriptorPool,
            pub device: Rc<ash::Device>,
        }

        impl #pipeline_cache_name {
            pub fn new(device: &Rc<ash::Device>) -> Self {
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

    Ok(pipeline_cache_code)
}

fn get_set_layout_bindings(module: &ShaderModule) -> Result<TokenStream, Box<dyn Error>> {
    let mut set_layout_bindings_code = quote! {};

    let desc_sets = module.enumerate_descriptor_sets(None)?;
    for desc_set in &desc_sets {
        for bind in &desc_set.bindings {
            let binding = bind.binding;
            let descriptor_type = descriptor_type_to_tokens(bind.descriptor_type);
            let count = bind.count;

            set_layout_bindings_code.extend(quote! {
                vk::DescriptorSetLayoutBinding::builder()
                .binding(#binding)
                .descriptor_type(#descriptor_type)
                .descriptor_count(#count)
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .build(),
            });
        }
    }

    Ok(set_layout_bindings_code)
}

pub fn get_bind_methods(module: &ShaderModule) -> Result<TokenStream, Box<dyn Error>> {
    // for all blocks, we should be able to generate a bind method
    let mut code = quote! {};

    let desc_sets = module.enumerate_descriptor_sets(None)?;
    for desc_set in &desc_sets {
        for bind in &desc_set.bindings {
            let binding = bind.binding;
            let descriptor_type = descriptor_type_to_tokens(bind.descriptor_type);
            let writes = quote! {
                vk::WriteDescriptorSet::builder()
                    .dst_set(set)
                    .dst_binding(#binding)
                    .dst_array_element(0)
                    .descriptor_type(#descriptor_type)
                    .build(),
            };

            let bind_signature = format!("bind_{}", bind.name)
                .parse::<proc_macro2::TokenStream>()
                .unwrap();
            code.extend(quote! {
                /// We do not know whether descriptor sets are allocated together and stored in a vector
                /// or they are allocated one by one, therefore we just expect one descriptor set here.
                pub fn #bind_signature(&self, set: vk::DescriptorSet) {
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
    }

    Ok(code)
}

pub fn get_pipeline_template(
    pipeline_name: &str,
    vert_spv_data: &[u8],
    frag_spv_data: &[u8],
) -> Result<TokenStream, Box<dyn Error>> {
    let vert_spv = Literal::byte_string(vert_spv_data);
    let frag_spv = Literal::byte_string(frag_spv_data);

    let module = spirv_reflect::create_shader_module(vert_spv_data)?;

    let set_layout_bindings_code = get_set_layout_bindings(&module)?;
    let bind_methods = get_bind_methods(&module)?;

    let pipeline_cache_name: TokenStream =
        format!("{}PipelineCache", capitalize(pipeline_name)).parse()?;
    let pipeline_cache_code = get_pipeline_cache_template(&pipeline_cache_name)?;

    let pipeline_struct_name: TokenStream =
        format!("{}Pipeline", capitalize(pipeline_name)).parse()?;

    let rust_code = quote! {
    /// Copyright © 2021-2022
    /// Author: Antonio Caggiano <info@antoniocaggiano.eu>
    /// SPDX-License-Identifier: MIT

    /// AUTOGENERATED: please do not delete it unless you want to regenerate it from scratch

    use std::{rc::Rc, collections::HashMap};

    use vkr_core::{
        ash::{self, vk},
        *,
    };

    #pipeline_cache_code

    pub struct #pipeline_struct_name {
        /// A pipeline cache for each frame
        caches: Vec<#pipeline_cache_name>,
        pipeline: vk::Pipeline,
        layout: vk::PipelineLayout,

        /// A pipeline can have multiple descriptor set layouts.
        /// This can be useful for binding at different frequencies.
        set_layouts: Vec<vk::DescriptorSetLayout>,
        device: Rc<ash::Device>,
    }

    impl #pipeline_struct_name {
        /// If I understand it correctly, a descriptor set may have multiple bindings
        fn get_set_layout_bindings() -> Vec<vk::DescriptorSetLayoutBinding> {
            vec![
                #set_layout_bindings_code
            ]
        }

        /// Different descriptor sets can be bound and updated at different times?
        /// I could use one desc set for the model matrix, but then I would need to
        /// call write_set for each node.
        /// A different approach would be creating a desc set for each node, but there
        /// is a limit to the number of sets that a pool can create, right?
        fn get_set_layouts(device: &Rc<ash::Device>) -> Vec<vk::DescriptorSetLayout> {
            let set_layout_bindings = Self::get_set_layout_bindings();
            let set_layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(&set_layout_bindings);

            let set_layout = unsafe {
                device.create_descriptor_set_layout(&set_layout_info, None)
            }
            .expect("Failed to create Vulkan descriptor set layout");

            vec![set_layout]
        }

        pub fn new<V: VertexInput>(
            dev: &mut Dev,
            topology: vk::PrimitiveTopology,
            pass: &Pass,
            width: u32,
            height: u32
        ) -> Self {
            let set_layouts = Self::get_set_layouts(&dev.device);

            // Pipeline layout (device, shader reflection?)
            let layout = {
                let create_info = vk::PipelineLayoutCreateInfo::builder()
                    .set_layouts(&set_layouts)
                    .build();
                unsafe { dev.device.create_pipeline_layout(&create_info, None) }
                    .expect("Failed to create Vulkan pipeline layout")
            };

            // Graphics pipeline (shaders, renderpass)
            #[allow(clippy::octal_escapes)]
            let pipeline = {
                let frag_mod = ShaderModule::new(
                    &dev.device,
                    #frag_spv,
                );
                let vert_mod = ShaderModule::new(
                    &dev.device,
                    #vert_spv,
                );

                let entrypoint = std::ffi::CString::new("main").expect("Failed to create main entrypoint");
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

                let vertex_bindings = V::get_bindings();
                let vertex_attributes = V::get_attributes();

                let vertex_input = vk::PipelineVertexInputStateCreateInfo::builder()
                    .vertex_attribute_descriptions(&vertex_attributes)
                    .vertex_binding_descriptions(&vertex_bindings)
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
                caches: vec![],
                pipeline,
                layout,
                set_layouts,
                device: Rc::clone(&dev.device),
            }
        }

        /// Returns the cache associated to a frame, creating it on demand
        pub fn get_cache(&mut self, frame_index: usize) -> &mut #pipeline_cache_name {
            while frame_index >= self.caches.len() {
                self.caches.push(#pipeline_cache_name::new(&self.device));
            }
            &mut self.caches[frame_index]
        }

        #bind_methods
    }

    impl Drop for #pipeline_struct_name {
        fn drop(&mut self) {
            unsafe {
                for set_layout in &self.set_layouts {
                    self.device
                        .destroy_descriptor_set_layout(*set_layout, None);
                }
                self.device.destroy_pipeline_layout(self.layout, None);
                self.device.destroy_pipeline(self.pipeline, None);
            }
        }
    }

    impl Pipeline for #pipeline_struct_name {
        fn get_pipeline(&self) -> vk::Pipeline {
            self.pipeline
        }

        fn get_layout(&self) -> vk::PipelineLayout {
            self.layout
        }

        fn get_set_layouts(&self) -> &[vk::DescriptorSetLayout] {
            &self.set_layouts
        }
    }
    };

    Ok(rust_code)
}
