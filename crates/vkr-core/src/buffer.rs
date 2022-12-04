// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::rc::Rc;

use ash::vk;

use crate::{ctx::Ctx, dev::Dev, vertex::Vertex};

pub struct Buffer {
    memory: vk::DeviceMemory,
    pub buffer: vk::Buffer,
    pub size: u64,
    device: Rc<ash::Device>,
}

impl Buffer {
    pub fn new(ctx: &Ctx, dev: &mut Dev) -> Self {
        // Vertex buffer of triangle to draw
        let size = std::mem::size_of::<Vertex>() as u64 * 3;
        let buffer_create_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .build();
        let buffer = unsafe { dev.device.create_buffer(&buffer_create_info, None) }
            .expect("Failed to create Vulkan vertex buffer");

        let requirements = unsafe { dev.device.get_buffer_memory_requirements(buffer) };

        let memory_type_index: u32 = {
            let mut mem_index: u32 = 0;
            let memory_properties = unsafe {
                ctx.instance
                    .get_physical_device_memory_properties(dev.physical)
            };
            for (i, memtype) in memory_properties.memory_types.iter().enumerate() {
                let res: vk::MemoryPropertyFlags = memtype.property_flags
                    & (vk::MemoryPropertyFlags::HOST_VISIBLE
                        | vk::MemoryPropertyFlags::HOST_COHERENT);
                if (requirements.memory_type_bits & (1 << i) != 0) && res.as_raw() != 0 {
                    mem_index = i as u32;
                }
            }
            mem_index
        };
        if memory_type_index == 0 {
            panic!("Failed to find Vulkan memory type index");
        }

        let mem_allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(requirements.size)
            .memory_type_index(memory_type_index)
            .build();
        let memory = unsafe { dev.device.allocate_memory(&mem_allocate_info, None) }
            .expect("Failed to allocate Vulkan memory");

        let offset = vk::DeviceSize::default();
        unsafe { dev.device.bind_buffer_memory(buffer, memory, offset) }
            .expect("Failed to bind Vulkan memory to buffer");

        Self {
            memory,
            buffer,
            size,
            device: Rc::clone(&dev.device),
        }
    }

    pub fn upload<T>(&mut self, src: *const T, size: usize) {
        let flags = vk::MemoryMapFlags::default();
        let data = unsafe { self.device.map_memory(self.memory, 0, self.size, flags) }
            .expect("Failed to map Vulkan memory");

        unsafe {
            data.copy_from(src as _, size);
            self.device.unmap_memory(self.memory);
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.device
                .device_wait_idle()
                .expect("Failed to wait for the device");
            self.device.free_memory(self.memory, None);
            self.device.destroy_buffer(self.buffer, None);
        }
    }
}
