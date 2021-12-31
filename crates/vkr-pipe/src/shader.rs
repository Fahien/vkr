// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

#[derive(Debug)]
pub enum ShaderType {
    Vertex,
    Fragment,
}

pub struct Pipeline {
    pub name: String,
    pub arg_types: Vec<syn::Ident>,
}

impl Pipeline {
    pub fn new(name: String, arg_types: Vec<syn::Ident>) -> Self {
        Self { name, arg_types }
    }
}
