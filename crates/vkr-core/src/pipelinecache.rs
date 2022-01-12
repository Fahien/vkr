// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::rc::Rc;

use ash::Device;

use crate::Descriptors;

pub struct PipelineCache {
    /// List of descriptors, one for each swapchain image
    pub descriptors: Descriptors,
}

impl PipelineCache {
    pub fn new(device: &Rc<Device>) -> Self {
        Self {
            descriptors: Descriptors::new(device),
        }
    }
}
