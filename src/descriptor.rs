// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{collections::HashMap, rc::Rc};

use ash::{vk, Device};

use crate::{model::Node, util::Handle, Material};

type SetCache<T> = HashMap<(vk::DescriptorSetLayout, Handle<T>), Vec<vk::DescriptorSet>>;

/// Per-frame resource which contains a descriptor pool and a vector
/// of descriptor sets of each pipeline layout used for rendering.
pub struct Descriptors {
    /// Descriptor sets for the GUI
    pub gui_sets: Vec<vk::DescriptorSet>,

    /// These descriptor sets are for camera view and proj uniform, therefore we need NxM descriptor sets
    /// where N is the number of pipeline layouts, and M is the number of nodes with cameras
    pub view_sets: SetCache<Node>,

    /// These descriptor sets are for model matrix uniforms, therefore we need NxM descriptor sets
    /// where N is the number of pipeline layouts, and M is the node with the model matrix
    pub model_sets: SetCache<Node>,

    /// These descriptor sets are for material uniforms, therefore we need NxM descriptor sets
    /// where N is the number of pipeline layouts, and M is the number of materials
    pub material_sets: SetCache<Material>,

    /// Descriptor sets for the present subpass
    pub present_sets: Vec<vk::DescriptorSet>,

    /// Descriptor pools should be per-pipeline layout as weel as they could differ in terms of uniforms and samplers?
    /// Or can we provide sufficient descriptors for all supported pipeline layouts? Trying this approach.
    pool: vk::DescriptorPool,

    pub device: Rc<Device>,
}

impl Descriptors {
    pub fn new(device: &Rc<Device>) -> Self {
        let pool = unsafe {
            // Support multiple matrices and material color for 3 pipelines
            let uniform_count = 10 * 3;
            let uniform_pool_size = vk::DescriptorPoolSize::builder()
                .descriptor_count(uniform_count)
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .build();

            // Support 1 material and 1 gui font texture for 3 pipelines
            let sampler_count = 2 * 3;
            let sampler_pool_size = vk::DescriptorPoolSize::builder()
                .descriptor_count(sampler_count)
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .build();

            // Support 3 input attachments
            let input_count = 3;
            let input_pool_size = vk::DescriptorPoolSize::builder()
                .descriptor_count(input_count)
                .ty(vk::DescriptorType::INPUT_ATTACHMENT)
                .build();

            let set_count = 16; // 5 nodes, 1 camera, 5 materials, 1 gui?
            let pool_sizes = vec![uniform_pool_size, sampler_pool_size, input_pool_size];
            let create_info = vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(&pool_sizes)
                .max_sets(set_count)
                .build();
            device.create_descriptor_pool(&create_info, None)
        }
        .expect("Failed to create Vulkan descriptor pool");

        Self {
            gui_sets: vec![],
            view_sets: SetCache::new(),
            model_sets: SetCache::new(),
            material_sets: SetCache::new(),
            present_sets: vec![],
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
