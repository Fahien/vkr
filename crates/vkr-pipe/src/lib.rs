// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

extern crate proc_macro;

use proc_macro::*;

use quote::quote;

mod util;
use util::*;

mod shader;
use shader::*;

mod module;
use module::*;

#[proc_macro]
pub fn pipewriter(input: TokenStream) -> TokenStream {
    let shader_crate = input.to_string().replace("\"", "");
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let crate_dir = current_dir.join(&shader_crate);

    let crate_module = CrateModule::new(crate_dir);

    // Build the Pipeline implementation
    gen_pipelines(&crate_module)
}

fn gen_pipelines(crate_module: &CrateModule) -> TokenStream {
    let crate_name: proc_macro2::TokenStream = format!("Crate{}", crate_module.name.to_camelcase())
        .parse()
        .unwrap();
    let shader_spv = format!("{}.spv", crate_module.name.replace('-', "_"));

    let pipelines = get_pipelines(&crate_module.file);

    let mut gen = quote! {
        use std::{ffi::CString, rc::Rc};
        use ash::{vk, Device};
        use vkr_core::{Dev, Pass, ShaderModule};
    };

    for pipeline in &pipelines {
        let pipeline_name = format!("Pipeline{}", pipeline.name.to_camelcase())
            .parse::<proc_macro2::TokenStream>()
            .expect("Failed to parse shader name");

        let vs = format!("{}_vs", pipeline.name.to_lowercase());
        let fs = format!("{}_fs", pipeline.name.to_lowercase());

        let pipeline_gen = quote! {
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
        };

        gen.extend(pipeline_gen);
    }

    let pipeline_vars = pipelines.iter().map(|m| {
        m.name
            .to_lowercase()
            .parse::<proc_macro2::TokenStream>()
            .expect("Failed to parse shader name")
    });

    let pipeline_defs = pipelines.iter().map(|m| {
        let pipeline_name = format!(
            "{}: Pipeline{}",
            m.name.to_lowercase(),
            m.name.to_camelcase()
        );
        pipeline_name
            .parse::<proc_macro2::TokenStream>()
            .expect("Failed to parse shader name")
    });

    let pipeline_vars_impl = pipelines.iter().map(|m| {
        let pipeline_name = format!(
            "let {} = Pipeline{}::new(&shader_module, &pass)",
            m.name.to_lowercase(),
            m.name.to_camelcase()
        );
        pipeline_name
            .parse::<proc_macro2::TokenStream>()
            .expect("Failed to parse shader name")
    });

    let crate_gen = quote! {
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
    };
    gen.extend(crate_gen);

    gen.into()
}

/// Collects all the pipelines found in a shader file
fn get_pipelines(file: &syn::File) -> Vec<Pipeline> {
    let mut pipelines = vec![];

    let functions = file
        .items
        .iter()
        .filter_map(|i| inner_value!(i, syn::Item::Fn(f) => f));

    // Go through all the functions of the file
    for func in functions {
        if let Some(spirv) = get_spirv(func) {
            let shader_type = get_shader_type(&spirv);
            if let Some(ShaderType::Fragment) = shader_type {
                // Extract prefix of function
                let prefix = get_prefix(&func.sig.ident.to_string());
                // Convert to camelcase and use it to name the pipeline
                let name = prefix.to_camelcase();
                pipelines.push(Pipeline::new(name));
            }
        }
    }

    pipelines
}

/// Analyzes the attributes of a function, looking for a spirv `MetaList`
fn get_spirv(func: &syn::ItemFn) -> Option<syn::MetaList> {
    func.attrs
        .iter()
        // which are metas
        .filter_map(|attr| attr.parse_meta().ok())
        // which are lists
        .filter_map(|meta| inner_value!(meta, syn::Meta::List(l) => l))
        // which idents are spirv
        .filter(|list| list.path.get_ident().unwrap() == "spirv")
        .next() // and take first
}

/// Analyzes a spirv `MetaList`, looking for vertex and fragment `Path`s
/// and returns the corresponding shader type
fn get_shader_type(spirv: &syn::MetaList) -> Option<ShaderType> {
    for nested in &spirv.nested {
        if let syn::NestedMeta::Meta(meta) = nested {
            if let syn::Meta::Path(path) = meta {
                if let Some(ident) = path.get_ident() {
                    if ident == "vertex" {
                        return Some(ShaderType::Vertex);
                    } else if ident == "fragment" {
                        return Some(ShaderType::Fragment);
                    }
                }
            }
        }
    }
    None
}
