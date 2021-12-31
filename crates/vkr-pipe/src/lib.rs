// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

extern crate proc_macro;

use proc_macro::*;

mod util;
use util::*;

mod shader;
use shader::*;

mod module;
use module::*;

mod gen;

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
    let mut gen = gen::header();

    let pipelines = get_pipelines(&crate_module.file);

    for pipeline in &pipelines {
        let pipeline_gen = gen::pipeline(pipeline);
        gen.extend(pipeline_gen);
    }

    let crate_gen = gen::crate_module(&crate_module, &pipelines);
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
        // Analyze spirv attribute
        if let Some(spirv) = get_spirv(func) {
            let mut name = None;

            // Collect input parameters
            let mut arg_types = vec![];

            let shader_type = get_shader_type(&spirv);
            match shader_type {
                Some(ShaderType::Vertex) => {
                    // Extract prefix of function
                    let prefix = get_prefix(&func.sig.ident.to_string());
                    // Convert to camelcase and use it to name the pipeline
                    name = Some(prefix.to_camelcase());

                    arg_types = get_args_type(func);
                }
                _ => (),
            }

            if let Some(name) = name {
                pipelines.push(Pipeline::new(name, arg_types));
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

/// Collects the arguments type of a function
fn get_args_type(func: &syn::ItemFn) -> Vec<syn::Ident> {
    let mut ret = vec![];

    for arg in &func.sig.inputs {
        match arg {
            syn::FnArg::Typed(t) => match &*t.ty {
                syn::Type::Path(p) => {
                    for seg in &p.path.segments {
                        if seg.ident == "Vec3" {
                            ret.push(seg.ident.clone());
                        } else {
                            todo!("Handle input {}", seg.ident);
                        }
                    }
                }
                syn::Type::Reference(_) => {
                    // TODO: look for mutable output values
                }
                _ => (),
            },
            _ => (),
        }
    }

    ret
}
