// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{ffi::CString, rc::Rc};

use ash::{vk, Device};
use byteorder::{ByteOrder, NativeEndian};

pub struct ShaderModule {
    shader: vk::ShaderModule,
    device: Rc<Device>,
}

impl ShaderModule {
    pub fn new(device: &Rc<Device>, bytes: &[u8]) -> Self {
        let device = device.clone();

        let mut code = vec![0; bytes.len() / std::mem::size_of::<u32>()];
        NativeEndian::read_u32_into(bytes, code.as_mut_slice());

        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code(code.as_slice())
            .build();
        let shader = unsafe { device.create_shader_module(&create_info, None) }
            .expect("Failed to create Vulkan shader module");

        Self { shader, device }
    }

    pub fn main(device: &Rc<Device>) -> Self {
        const SHADERS: &[u8] = include_bytes!(env!("vkr_main_shaders.spv"));
        Self::new(device, SHADERS)
    }

    pub fn gui(device: &Rc<Device>) -> Self {
        const SHADERS: &[u8] = include_bytes!(env!("vkr_gui_shaders.spv"));
        Self::new(device, SHADERS)
    }

    /// The entrypoint c string should be alive until the pipeline has been created
    pub fn get_stage(
        &self,
        entrypoint: &CString,
        flag: vk::ShaderStageFlags,
    ) -> vk::PipelineShaderStageCreateInfo {
        vk::PipelineShaderStageCreateInfo::builder()
            .stage(flag)
            .module(self.shader)
            .name(entrypoint)
            .build()
    }

    pub fn get_vert(&self, entrypoint: &CString) -> vk::PipelineShaderStageCreateInfo {
        self.get_stage(entrypoint, vk::ShaderStageFlags::VERTEX)
    }

    pub fn get_frag(&self, entrypoint: &CString) -> vk::PipelineShaderStageCreateInfo {
        self.get_stage(entrypoint, vk::ShaderStageFlags::FRAGMENT)
    }
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_shader_module(self.shader, None);
        }
    }
}
