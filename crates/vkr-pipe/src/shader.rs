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
}

impl Pipeline {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}
