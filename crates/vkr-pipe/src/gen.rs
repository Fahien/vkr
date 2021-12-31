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
        use vkr_core::{Dev, Pass, ShaderModule};
    }
}

pub fn pipeline(pipeline: &Pipeline) -> TokenStream {
    let pipeline_name = format!("Pipeline{}", pipeline.name.to_camelcase())
        .parse::<proc_macro2::TokenStream>()
        .expect("Failed to parse shader name");

    let vs = format!("{}_vs", pipeline.name.to_lowercase());
    let fs = format!("{}_fs", pipeline.name.to_lowercase());

    quote! {
        pub struct #pipeline_name {
            pipeline: vk::Pipeline
        }

        impl #pipeline_name {
            pub fn new_layout(device: &Rc<Device>) -> vk::PipelineLayout {
                let create_info = vk::PipelineLayoutCreateInfo::builder().build();
                let layout = unsafe { device.create_pipeline_layout(&create_info, None) };
                layout.expect("Failed to create Vulkan pipeline layout")
            }

            pub fn new_impl(shader_module: &ShaderModule, vs: &str, fs: &str, pass: &Pass) -> vk::Pipeline {
                let vs_entry = CString::new(vs).expect("Failed to create vertex entry point");
                let fs_entry = CString::new(fs).expect("Failed to create vertex entry point");

                let stages = [
                    shader_module.get_vert(&vs_entry),
                    shader_module.get_frag(&fs_entry)
                ];

                let layout = Self::new_layout(&shader_module.device);

                let vertex_input = vk::PipelineVertexInputStateCreateInfo::builder()
                    // TODO: collect bindings and attributes
                    .build();

                let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
                    .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
                    .build();

                let rasterization = vk::PipelineRasterizationStateCreateInfo::builder()
                    .line_width(1.0)
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

                let states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
                let dynamics = vk::PipelineDynamicStateCreateInfo::builder()
                    .dynamic_states(&states)
                    .build();

                let create_info = vk::GraphicsPipelineCreateInfo::builder()
                    .stages(&stages)
                    .layout(layout)
                    .render_pass(pass.render)
                    .vertex_input_state(&vertex_input)
                    .input_assembly_state(&input_assembly)
                    .rasterization_state(&rasterization)
                    .viewport_state(&view)
                    .multisample_state(&multisample)
                    .dynamic_state(&dynamics)
                    .build();

                let pipelines = unsafe { shader_module.device.create_graphics_pipelines(vk::PipelineCache::null(), &[create_info], None) };
                let mut pipelines = pipelines.expect("Failed to create Vulkan graphics pipeline");
                let pipeline = pipelines.pop().expect("Failed to pop Vulkan pipeline");

                pipeline
            }

            pub fn new(shader_module: &ShaderModule, render_pass: &Pass) -> Self {
                let pipeline = Self::new_impl(shader_module, #vs, #fs, render_pass);

                Self {
                    pipeline
                }
            }
        }
    }
}

pub fn crate_module(crate_module: &CrateModule, pipelines: &[Pipeline]) -> TokenStream {
    let crate_name: proc_macro2::TokenStream = format!("Crate{}", crate_module.name.to_camelcase())
        .parse()
        .unwrap();

    let shader_spv = format!("{}.spv", crate_module.name.replace('-', "_"));

    let pipeline_vars = pipelines.iter().map(|m| {
        m.name
            .to_lowercase()
            .parse::<TokenStream>()
            .expect("Failed to parse shader name")
    });

    let pipeline_defs = pipelines.iter().map(|m| {
        let pipeline_name = format!(
            "{}: Pipeline{}",
            m.name.to_lowercase(),
            m.name.to_camelcase()
        );
        pipeline_name
            .parse::<TokenStream>()
            .expect("Failed to parse shader name")
    });

    let pipeline_vars_impl = pipelines.iter().map(|m| {
        let pipeline_name = format!(
            "let {} = Pipeline{}::new(&shader_module, &pass)",
            m.name.to_lowercase(),
            m.name.to_camelcase()
        );
        pipeline_name
            .parse::<TokenStream>()
            .expect("Failed to parse shader name")
    });

    quote! {
        pub struct #crate_name {
            shader_module: ShaderModule,
            pass: Pass,
            pub #( #pipeline_defs, )*
        }

        impl #crate_name {
            pub fn new(dev: &Dev) -> Self {
                const CODE: &[u8] = include_bytes!(env!(#shader_spv));
                let shader_module = ShaderModule::new(&dev.device, CODE);

                let pass = Pass::new(dev);

                #( #pipeline_vars_impl; )*

                Self {
                    shader_module,
                    pass,
                #( #pipeline_vars, )*
                }
            }
        }
    }
}
