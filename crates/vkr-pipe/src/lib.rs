// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

extern crate proc_macro;
use std::{fs::File, io::Read};

use proc_macro::*;

use quote::quote;

mod util;
use util::*;

mod shader;
use shader::*;

#[proc_macro]
pub fn pipewriter(input: TokenStream) -> TokenStream {
    let shader = input.to_string().replace("\"", "");
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let shader_path = current_dir.join(&shader);
    let mut code = File::open(&shader_path)
        .expect(&format!("Failed to open shader {}", shader_path.display()));
    let mut buf = String::new();
    code.read_to_string(&mut buf)
        .expect(&format!("Failed to read shader {}", shader_path.display()));
    let file = syn::parse_file(&buf).expect(&format!("Failed to parse {}", shader_path.display()));

    // Build the Pipeline implementation
    gen_pipelines(&file)
}

fn gen_pipelines(file: &syn::File) -> TokenStream {
    let mut gen = TokenStream::new();

    for pipeline in get_pipelines(file) {
        let struct_name: proc_macro2::TokenStream =
            format!("Pipeline{}", pipeline.name).parse().unwrap();

        let pipeline_gen = quote! {
            struct #struct_name {

            }
        };

        gen.extend::<TokenStream>(pipeline_gen.into());
    }

    gen
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