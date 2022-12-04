// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::vk;

pub struct Image {
    pub image: vk::Image,
    pub format: vk::Format,
    pub color_space: vk::ColorSpaceKHR,
    pub width: u32,
    pub height: u32,
}

impl Image {
    pub fn new(
        image: vk::Image,
        format: vk::Format,
        color_space: vk::ColorSpaceKHR,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            image,
            format,
            color_space,
            width,
            height,
        }
    }
}
