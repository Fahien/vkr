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
            return matches!(get_shader_type(func), Some(ShaderType::Vertex));
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
        if let Some(shader_type) = get_shader_type(func) {
            // Extract prefix of function
            let prefix = get_prefix(&func.sig.ident.to_string());
            // Convert to camelcase and use it to name the pipeline
            let name = prefix.to_camelcase();

            let builder = builders.get_mut(&name).unwrap();

            if shader_type == ShaderType::Vertex {
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

/// Analyzes a function attributes, looking for vertex and fragment `Path`s
/// and returns the corresponding shader type
fn get_shader_type(func: &syn::ItemFn) -> Option<ShaderType> {
    if let Some(spirv) = get_spirv(&func.attrs) {
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
    }
    None
}

fn get_arg_name(arg: &syn::PatType) -> Option<syn::Ident> {
    match &*arg.pat {
        syn::Pat::Box(_) => todo!(),
        syn::Pat::Ident(i) => return Some(i.ident.clone()),
        syn::Pat::Lit(_) => todo!(),
        syn::Pat::Macro(_) => todo!(),
        syn::Pat::Or(_) => todo!(),
        syn::Pat::Path(_) => todo!(),
        syn::Pat::Range(_) => todo!(),
        syn::Pat::Reference(_) => todo!(),
        syn::Pat::Rest(_) => todo!(),
        syn::Pat::Slice(_) => todo!(),
        syn::Pat::Struct(_) => todo!(),
        syn::Pat::Tuple(_) => todo!(),
        syn::Pat::TupleStruct(_) => todo!(),
        syn::Pat::Type(_) => todo!(),
        syn::Pat::Verbatim(_) => todo!(),
        syn::Pat::Wild(_) => todo!(),
        syn::Pat::__TestExhaustive(_) => todo!(),
    }
}

fn get_arg_segment(arg: &syn::PatType) -> Option<syn::Ident> {
    match &*arg.ty {
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

fn get_spirv_value(spirv: &syn::MetaList, id: &str) -> Option<u32> {
    if let Some(desc_set) = get_meta_name_value(&spirv, id) {
        Some(
            inner_value!(&desc_set.lit, syn::Lit::Int(i) => i)
                .unwrap()
                .base10_parse::<u32>()
                .unwrap(),
        )
    } else {
        None
    }
}

fn get_uniforms(func: &syn::ItemFn) -> Vec<Uniform> {
    let mut uniforms = vec![];

    let shader_type = get_shader_type(func).expect("Can not get uniforms from this function");

    for arg in &func.sig.inputs {
        match arg {
            syn::FnArg::Typed(arg) => {
                let spirv = get_spirv(&arg.attrs);
                if let Some(spirv) = spirv {
                    if let Some(desc_set) = get_spirv_value(&spirv, "descriptor_set") {
                        let name = get_arg_name(arg).expect("Failed to get argument name");
                        let ident = get_arg_segment(arg).unwrap();
                        let binding = get_spirv_value(&spirv, "binding").unwrap();
                        uniforms.push(Uniform::new(name, ident, desc_set, binding, shader_type))
                    }
                }
            }
            _ => (),
        }
    }

    uniforms
}
