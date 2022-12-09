// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use crate::{Handle, Trs};

#[derive(Default)]
pub struct Node {
    pub trs: Trs,
    pub children: Vec<Handle<Node>>,
}

impl Node {
    pub fn new() -> Self {
        Node {
            trs: Trs::new(),
            children: vec![],
        }
    }
}
