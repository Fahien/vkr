// Copyright © 2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::{vk, Device};
use std::rc::Rc;

pub struct DescriptorPool {
    sets: Vec<Vec<vk::DescriptorSet>>,
    pool: vk::DescriptorPool,
    pub device: Rc<Device>,
}

impl DescriptorPool {
    /// `set_count`: How many descriptor sets can be created in this pool
    /// `uniform_count`: Total amount of uniform descriptors among all sets
    /// `sampler_count`: Total amount of combined image sampler descriptors among all sets
    /// `input_count`: Total amount of input attachment descriptors among all sets
    pub fn new(
        device: &Rc<Device>,
        set_count: u32,
        uniform_count: u32,
        sampler_count: u32,
        input_count: u32,
    ) -> Self {
        let pool = unsafe {
            let uniform_pool_size = vk::DescriptorPoolSize::builder()
                .descriptor_count(uniform_count)
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .build();

            let sampler_pool_size = vk::DescriptorPoolSize::builder()
                .descriptor_count(sampler_count)
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .build();

            let input_pool_size = vk::DescriptorPoolSize::builder()
                .descriptor_count(input_count)
                .ty(vk::DescriptorType::INPUT_ATTACHMENT)
                .build();

            let pool_sizes = vec![uniform_pool_size, sampler_pool_size, input_pool_size];
            let create_info = vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(&pool_sizes)
                .max_sets(set_count)
                .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
                .build();
            device.create_descriptor_pool(&create_info, None)
        }
        .expect("Failed to create Vulkan descriptor pool");

        Self {
            sets: vec![],
            pool,
            device: device.clone(),
        }
    }

    pub fn allocate(&mut self, layouts: &[vk::DescriptorSetLayout]) -> Vec<vk::DescriptorSet> {
        let create_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.pool)
            .set_layouts(layouts)
            .build();

        let sets = unsafe { self.device.allocate_descriptor_sets(&create_info) }
            .expect("Failed to allocate Vulkan descriptor sets");

        // Store sets in the pool for release later
        self.sets.push(sets.clone());
        sets
    }

    pub fn free(&mut self, sets: Vec<vk::DescriptorSet>) {
        if let Some((index, sets)) = self.sets.iter().enumerate().find(|(_, s)| **s == sets) {
            unsafe { self.device.free_descriptor_sets(self.pool, sets) }
                .expect("Failed to free descriptor sets");
            self.sets.remove(index);
        } else {
            panic!("Can not free descriptor sets. Maybe they belog to a differe descriptor pool?");
        }
    }
}

impl Drop for DescriptorPool {
    fn drop(&mut self) {
        unsafe { self.device.destroy_descriptor_pool(self.pool, None) };
    }
}
