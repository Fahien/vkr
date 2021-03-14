// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{collections::HashMap, rc::Rc};

use ash::{vk, Device};

use crate::{util::Handle, model::Node};

/// Per-frame resource which contains a descriptor pool and a vector
/// of descriptor sets of each pipeline layout used for rendering.
pub struct Descriptors {
    /// These descriptor sets are for model matrix uniforms, therefore we need NxM descriptor sets
    /// where N is the number of pipeline layouts, and M is the node with the model matrix
    pub sets: HashMap<(vk::PipelineLayout, Handle<Node>), Vec<vk::DescriptorSet>>,
    pool: vk::DescriptorPool,
    device: Rc<Device>,
}

impl Descriptors {
    pub fn new(device: &Rc<Device>) -> Self {
        let pool = unsafe {
            let pool_size = vk::DescriptorPoolSize::builder()
                // Just one for the moment
                .descriptor_count(1)
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .build();
            let pool_sizes = vec![pool_size, pool_size];
            let create_info = vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(&pool_sizes)
                // Support 4 different pipeline layouts
                .max_sets(2)
                .build();
            device.create_descriptor_pool(&create_info, None)
        }
        .expect("Failed to create Vulkan descriptor pool");

        Self {
            sets: HashMap::new(),
            pool,
            device: device.clone(),
        }
    }

    pub fn allocate(&mut self, layouts: &[vk::DescriptorSetLayout]) -> Vec<vk::DescriptorSet> {
        let create_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.pool)
            .set_layouts(layouts)
            .build();

        unsafe { self.device.allocate_descriptor_sets(&create_info) }
            .expect("Failed to allocate Vulkan descriptor sets")
    }
}

impl Drop for Descriptors {
    fn drop(&mut self) {
        unsafe { self.device.destroy_descriptor_pool(self.pool, None) };
    }
}
