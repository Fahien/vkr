// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use crate::{
    image::{Image, ImageView, Texture},
    Handle, Pack, Sampler, Trs,
};

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

pub struct Model {
    pub nodes: Pack<Node>,
    pub images: Pack<Image>,
    pub views: Pack<ImageView>,
    pub samplers: Pack<Sampler>,
    pub textures: Pack<Texture>,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    pub fn new() -> Self {
        Self {
            nodes: Pack::new(),
            images: Pack::new(),
            views: Pack::new(),
            samplers: Pack::new(),
            textures: Pack::new(),
        }
    }
}
