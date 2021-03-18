// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::rc::Rc;

use ash::{version::DeviceV1_0, *};

pub struct Fence {
    /// Ideally only Queue::submit should be allowed to modify this flag
    pub can_wait: bool,
    pub fence: vk::Fence,
    device: Rc<Device>,
}

impl Fence {
    pub fn new(device: &Rc<Device>, flags: vk::FenceCreateFlags) -> Self {
        let device = device.clone();

        let can_wait = flags.contains(vk::FenceCreateFlags::SIGNALED);

        let create_info = vk::FenceCreateInfo::builder().flags(flags).build();
        let fence = unsafe { device.create_fence(&create_info, None) }
            .expect("Failed to create Vulkan fence");

        Self {
            can_wait,
            fence,
            device,
        }
    }

    pub fn unsignaled(device: &Rc<Device>) -> Self {
        Self::new(device, vk::FenceCreateFlags::default())
    }

    pub fn signaled(device: &Rc<Device>) -> Self {
        Self::new(device, vk::FenceCreateFlags::SIGNALED)
    }

    pub fn wait(&mut self) {
        if self.can_wait {
            unsafe {
                self.device
                    .wait_for_fences(&[self.fence], true, std::u64::MAX)
            }
            .expect("Failed waiting for Vulkan fence");
            self.can_wait = false;
        }
    }

    pub fn reset(&mut self) {
        self.can_wait = false;
        unsafe { self.device.reset_fences(&[self.fence]) }.expect("Failed to reset Vulkan fence");
    }
}

impl Drop for Fence {
    fn drop(&mut self) {
        self.wait();
        unsafe {
            self.device.destroy_fence(self.fence, None);
        }
    }
}
