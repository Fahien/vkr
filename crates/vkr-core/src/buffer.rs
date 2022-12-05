// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{cell::RefCell, rc::Rc};

use ash::vk;

use crate::Vertex;

pub struct Buffer {
    allocation: vk_mem::Allocation,
    pub buffer: vk::Buffer,
    pub size: vk::DeviceSize,
    allocator: Rc<RefCell<vk_mem::Allocator>>,
}

impl Buffer {
    fn create_buffer(
        allocator: &vk_mem::Allocator,
        size: vk::DeviceSize,
    ) -> (vk::Buffer, vk_mem::Allocation) {
        let buffer_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .build();

        // Vulkan memory
        let create_info = vk_mem::AllocationCreateInfo::new()
            .usage(vk_mem::MemoryUsage::CpuToGpu)
            .required_flags(vk::MemoryPropertyFlags::HOST_VISIBLE)
            .preferred_flags(
                vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_CACHED,
            );

        let (buffer, allocation, _) =
            unsafe { allocator.create_buffer(&buffer_info, &create_info) }
                .expect("Failed to create Vulkan buffer");

        (buffer, allocation)
    }

    pub fn new(allocator: &Rc<RefCell<vk_mem::Allocator>>) -> Self {
        // Default size allows for 3 vertices
        let size = std::mem::size_of::<Vertex>() as vk::DeviceSize * 3;

        let (buffer, allocation) = Self::create_buffer(&allocator.borrow(), size);

        Self {
            allocation,
            buffer,
            size,
            allocator: allocator.clone(),
        }
    }

    pub fn upload<T>(&mut self, src: *const T, size: vk::DeviceSize) {
        let alloc = self.allocator.borrow();
        let data =
            unsafe { alloc.map_memory(self.allocation) }.expect("Failed to map Vulkan memory");
        unsafe { data.copy_from(src as _, size as usize) };
        unsafe { alloc.unmap_memory(self.allocation) };
    }

    pub fn upload_arr<T>(&mut self, arr: &[T]) {
        // Create a new buffer if not enough size for the vector
        let size = (arr.len() * std::mem::size_of::<T>()) as vk::DeviceSize;
        if size as vk::DeviceSize != self.size {
            let alloc = self.allocator.borrow();
            unsafe { alloc.destroy_buffer(self.buffer, self.allocation) };

            self.size = size;
            let (buffer, allocation) = Self::create_buffer(&alloc, size);
            self.buffer = buffer;
            self.allocation = allocation;
        }

        self.upload(arr.as_ptr(), size);
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.allocator
                .borrow()
                .destroy_buffer(self.buffer, self.allocation)
        };
    }
}
