// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{collections::HashMap, rc::Rc};

use ash::{vk, Device};

use crate::{model::Node, util::Handle};

type SetCache = HashMap<(vk::PipelineLayout, Handle<Node>), Vec<vk::DescriptorSet>>;

/// Per-frame resource which contains a descriptor pool and a vector
/// of descriptor sets of each pipeline layout used for rendering.
pub struct Descriptors {
    /// These descriptor sets are for camera view and proj uniform, therefore we need NxM descriptor sets
    /// where N is the number of pipeline layouts, and M is the number of nodes with cameras
    pub view_sets: SetCache,

    /// These descriptor sets are for model matrix uniforms, therefore we need NxM descriptor sets
    /// where N is the number of pipeline layouts, and M is the node with the model matrix
    pub model_sets: SetCache,

    /// Descriptor pools should be per-pipeline layout as weel as they could differ in terms of uniforms and samplers?
    /// Or can we provide sufficient descriptors for all supported pipeline layouts? Trying this approach.
    pool: vk::DescriptorPool,

    device: Rc<Device>,
}

impl Descriptors {
    pub fn new(device: &Rc<Device>) -> Self {
        let pool = unsafe {
            let uniform_pool_size = vk::DescriptorPoolSize::builder()
                .descriptor_count(3 * 2) // Support model, view and proj matrix for 2 pipelines
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .build();
            let sampler_pool_size = vk::DescriptorPoolSize::builder()
                .descriptor_count(2) // Support 1 material for 2 pipelines
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .build();

            let pool_sizes = vec![uniform_pool_size, sampler_pool_size];
            let create_info = vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(&pool_sizes)
                // @todo Use a parameter instead of 2 for frame count
                .max_sets(2 * 2) // Support 2 frames with 2 pipelines
                .build();
            device.create_descriptor_pool(&create_info, None)
        }
        .expect("Failed to create Vulkan descriptor pool");

        Self {
            view_sets: HashMap::new(),
            model_sets: HashMap::new(),
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
