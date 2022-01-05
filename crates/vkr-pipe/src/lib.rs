// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

extern crate proc_macro;

use std::collections::{HashMap, HashSet};

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

    gen.extend(gen::cache(crate_module, &pipelines));

    for pipeline in &pipelines {
        let pipeline_gen = gen::pipeline(pipeline);
        gen.extend(pipeline_gen);
    }

    gen.into()
}

/// Collects all the pipelines found in a shader file
fn get_pipelines(file: &syn::File) -> Vec<Pipeline> {
    let functions = file
        .items
        .iter()
        .filter_map(|i| inner_value!(i, syn::Item::Fn(f) => f));

    // Collect names first
    let names: HashSet<String> = functions
        .clone()
        .filter(|func| {
            let spirv = get_spirv(&func.attrs);
            if let Some(meta) = spirv.as_ref() {
                return matches!(get_shader_type(meta), Some(ShaderType::Vertex));
            }
            false
        })
        .map(|func| get_prefix(&func.sig.ident.to_string()).to_camelcase())
        .collect();

    // TODO contruct pipelines now and then populate args and uniforms?
    let mut builders: HashMap<String, PipelineBuilder> = names
        .into_iter()
        .map(|name| (name.clone(), Pipeline::builder().name(name)))
        .collect();

    // Go through all the functions of the file
    for func in functions {
        // Analyze spirv attribute
        if let Some(spirv) = get_spirv(&func.attrs) {
            // Collect input parameters
            let shader_type = get_shader_type(&spirv);

            // Extract prefix of function
            let prefix = get_prefix(&func.sig.ident.to_string());
            // Convert to camelcase and use it to name the pipeline
            let name = prefix.to_camelcase();

            let builder = builders.get_mut(&name).unwrap();

            if matches!(shader_type, Some(ShaderType::Vertex)) {
                let arg_types = get_args_type(func);
                builder.arg_types(arg_types);
            }

            builder.add_uniforms(get_uniforms(func));
        }
    }

    builders.into_iter().map(|(_, b)| b.build()).collect()
}

/// Analyzes the attributes of a function, looking for a spirv `MetaList`
fn get_spirv(attrs: &[syn::Attribute]) -> Option<syn::MetaList> {
    attrs
        .iter()
        // which are metas
        .filter_map(|attr| attr.parse_meta().ok())
        // which are lists
        .filter_map(|meta| inner_value!(meta, syn::Meta::List(l) => l))
        // which idents are spirv
        .filter(|list| list.path.get_ident().unwrap() == "spirv")
        .next() // and take first
}

#[allow(unused)]
fn dump_meta<'m>(list: &'m syn::MetaList) {
    for nested in &list.nested {
        if let syn::NestedMeta::Meta(meta) = nested {
            if let syn::Meta::Path(path) = meta {
                if let Some(id) = path.get_ident() {
                    eprintln!("path: {}", id);
                }
            }
            if let syn::Meta::NameValue(name_value) = meta {
                if let Some(id) = name_value.path.get_ident() {
                    eprintln!("path: {}", id);
                }
            }
        }
    }
}

fn get_meta_name_value<'m>(list: &'m syn::MetaList, ident: &str) -> Option<&'m syn::MetaNameValue> {
    for nested in &list.nested {
        if let syn::NestedMeta::Meta(meta) = nested {
            if let syn::Meta::NameValue(name_value) = meta {
                if let Some(id) = name_value.path.get_ident() {
                    if id == ident {
                        return Some(name_value);
                    }
                }
            }
        }
    }
    None
}

fn get_meta_path<'m>(list: &'m syn::MetaList, ident: &str) -> Option<&'m syn::Path> {
    for nested in &list.nested {
        if let syn::NestedMeta::Meta(meta) = nested {
            if let syn::Meta::Path(path) = meta {
                if let Some(id) = path.get_ident() {
                    if id == ident {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
}

/// Analyzes a spirv `MetaList`, looking for vertex and fragment `Path`s
/// and returns the corresponding shader type
fn get_shader_type(spirv: &syn::MetaList) -> Option<ShaderType> {
    // TODO use get_meta_path
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

fn get_arg_segment(arg: &syn::FnArg) -> Option<syn::Ident> {
    match arg {
        syn::FnArg::Typed(t) => match &*t.ty {
            syn::Type::Path(p) => {
                if !p.path.segments.is_empty() {
                    return Some(p.path.segments[0].ident.clone());
                }
            }
            syn::Type::Reference(r) => match &*r.elem {
                syn::Type::Path(p) => {
                    if !p.path.segments.is_empty() {
                        return Some(p.path.segments[0].ident.clone());
                    }
                }
                _ => (),
            },
            _ => (),
        },
        _ => (),
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
                        match seg.ident.to_string().as_str() {
                            "Vec4" | "Vec3" | "Vec2" => ret.push(seg.ident.clone()),
                            _ => todo!("Handle input {}: {}:{}", seg.ident, file!(), line!()),
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

fn get_spirv_value(spirv: &syn::MetaList, id: &str) -> u32 {
    let desc_set = get_meta_name_value(&spirv, id).unwrap();
    inner_value!(&desc_set.lit, syn::Lit::Int(i) => i)
        .unwrap()
        .base10_parse::<u32>()
        .unwrap()
}

fn get_uniforms(func: &syn::ItemFn) -> Vec<Uniform> {
    let mut uniforms = vec![];

    for arg in &func.sig.inputs {
        match arg {
            syn::FnArg::Typed(t) => {
                let spirv = get_spirv(&t.attrs);
                if let Some(spirv) = spirv {
                    let path = get_meta_path(&spirv, "uniform");
                    if path.is_some() {
                        let ident = get_arg_segment(arg).unwrap();
                        let desc_set = get_spirv_value(&spirv, "descriptor_set");
                        let binding = get_spirv_value(&spirv, "binding");
                        // TODO get stage
                        uniforms.push(Uniform::new(ident, desc_set, binding, ShaderType::Vertex))
                    }
                }
            }
            _ => (),
        }
    }

    uniforms
}
