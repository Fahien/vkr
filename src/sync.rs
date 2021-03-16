// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::rc::Rc;

use ash::{vk, Device};

pub struct Semaphore {
    pub semaphore: vk::Semaphore,
    device: Rc<Device>,
}

impl Semaphore {
    pub fn new(device: &Rc<Device>) -> Self {
        let create_info = vk::SemaphoreCreateInfo::builder().build();
        let semaphore = unsafe { device.create_semaphore(&create_info, None) }
            .expect("Failed to create Vulkan semaphore");

        Self {
            semaphore,
            device: device.clone(),
        }
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe { self.device.destroy_semaphore(self.semaphore, None) };
    }
}
