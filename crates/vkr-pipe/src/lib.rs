// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

extern crate proc_macro;
use std::{fs::File, io::Read};

use proc_macro::*;

use quote::quote;
use syn;

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
    for item in &file.items {
        match item {
            syn::Item::Fn(i) => {
                eprintln!("fn {}", i.sig.ident);
                for attr in &i.attrs {
                    let meta = attr.parse_meta().unwrap();
                    match &meta {
                        syn::Meta::Path(_) => eprintln!("path"),
                        syn::Meta::List(l) => {
                            for elem in &l.nested {
                                match &elem {
                                    syn::NestedMeta::Meta(m) => match &m {
                                        syn::Meta::Path(p) => {
                                            eprintln!("path: {}", p.get_ident().unwrap())
                                        }
                                        syn::Meta::List(_) => eprintln!("{}", "list"),
                                        syn::Meta::NameValue(n) => eprintln!("{}", n.lit.suffix()),
                                    },
                                    syn::NestedMeta::Lit(l) => eprintln!("Literal: {}", l.suffix()),
                                }
                            }
                        }
                        syn::Meta::NameValue(v) => eprintln!("{}", v.lit.suffix()),
                    }
                }
            }
            syn::Item::Const(i) => eprintln!("{}", i.ident),
            syn::Item::Enum(i) => eprintln!("{}", i.ident),
            syn::Item::ExternCrate(i) => eprintln!("{}", i.ident),
            syn::Item::ForeignMod(i) => eprintln!("{}", i.abi.name.as_ref().unwrap().value()),
            syn::Item::Impl(_) => eprintln!("{}", "impl"),
            syn::Item::Macro(_) => eprintln!("{}", "macro"),
            syn::Item::Macro2(i) => eprintln!("{}", i.ident),
            syn::Item::Mod(i) => eprintln!("{}", i.ident),
            syn::Item::Static(i) => eprintln!("{}", i.ident),
            syn::Item::Struct(i) => eprintln!("{}", i.ident),
            syn::Item::Trait(i) => eprintln!("{}", i.ident),
            syn::Item::TraitAlias(i) => eprintln!("{}", i.ident),
            syn::Item::Type(i) => eprintln!("{}", i.ident),
            syn::Item::Union(i) => eprintln!("{}", i.ident),
            syn::Item::Verbatim(_) => eprintln!("{}", "verbatim"),
            syn::Item::__TestExhaustive(_) => eprintln!("{}", "text"),
            _ => (),
        }
    }

    let gen = quote! {
        struct Pipeline {

        }
    };
    gen.into()
}
