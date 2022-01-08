// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::vk;

pub struct Texture {
    pub view: vk::ImageView,
    pub sampler: vk::Sampler,
}

impl Texture {
    pub fn new(view: vk::ImageView, sampler: vk::Sampler) -> Self {
        Self { view, sampler }
    }
}
