// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::rc::Rc;

use ash::vk;
use byteorder::{ByteOrder, NativeEndian};

pub struct ShaderModule {
    pub shader: vk::ShaderModule,
    device: Rc<ash::Device>,
}

impl ShaderModule {
    pub fn new(device: &Rc<ash::Device>, spv: &[u8]) -> Self {
        let device = device.clone();

        let mut code = vec![0; spv.len() / std::mem::size_of::<u32>()];
        NativeEndian::read_u32_into(spv, code.as_mut_slice());

        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code(code.as_slice())
            .build();
        let shader = unsafe { device.create_shader_module(&create_info, None) }
            .expect("Failed to create Vulkan shader module");

        Self { shader, device }
    }
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_shader_module(self.shader, None);
        }
    }
}
