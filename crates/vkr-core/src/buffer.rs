// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{cell::RefCell, rc::Rc};

use ash::vk;

use crate::image::Png;

pub struct Buffer {
    allocation: vk_mem::Allocation,
    pub buffer: vk::Buffer,
    pub size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    pub allocator: Rc<RefCell<vk_mem::Allocator>>,
}

impl Buffer {
    fn create_buffer(
        allocator: &vk_mem::Allocator,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
    ) -> (vk::Buffer, vk_mem::Allocation) {
        let buffer_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(usage)
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

    pub fn new<T>(allocator: &Rc<RefCell<vk_mem::Allocator>>, usage: vk::BufferUsageFlags) -> Self {
        // Default size allows for 3 vertices
        let size = std::mem::size_of::<T>() as vk::DeviceSize * 3;

        let (buffer, allocation) = Self::create_buffer(&allocator.borrow(), size, usage);

        Self {
            allocation,
            buffer,
            size,
            usage,
            allocator: allocator.clone(),
        }
    }

    /// Loads data from a png image in `path` directly into a staging buffer
    pub fn load(allocator: &Rc<RefCell<vk_mem::Allocator>>, png: &mut Png) -> Self {
        let size = png.reader.output_buffer_size();
        let usage = vk::BufferUsageFlags::TRANSFER_SRC;

        // Create staging buffer
        let (buffer, allocation) =
            Self::create_buffer(&allocator.borrow(), size as vk::DeviceSize, usage);

        let alloc = allocator.borrow();
        let data = unsafe { alloc.map_memory(allocation) }.expect("Failed to map Vulkan memory");

        // Allocate the output buffer
        let buf = unsafe { std::slice::from_raw_parts_mut(data, size) };

        // Read the next frame. An APNG might contain multiple frames.
        png.reader.next_frame(buf).unwrap();

        unsafe { alloc.unmap_memory(allocation) };

        Self {
            allocation,
            buffer,
            usage,
            size: size as vk::DeviceSize,
            allocator: allocator.clone(),
        }
    }

    pub fn upload_raw<T>(&mut self, src: *const T, size: vk::DeviceSize) {
        let alloc = self.allocator.borrow();
        let data =
            unsafe { alloc.map_memory(self.allocation) }.expect("Failed to map Vulkan memory");
        unsafe { data.copy_from(src as _, size as usize) };
        unsafe { alloc.unmap_memory(self.allocation) };
    }

    pub fn upload<T>(&mut self, data: &T) {
        self.upload_raw(data as *const T, std::mem::size_of::<T>() as vk::DeviceSize);
    }

    pub fn upload_arr<T>(&mut self, arr: &[T]) {
        // Create a new buffer if not enough size for the vector
        let size = (arr.len() * std::mem::size_of::<T>()) as vk::DeviceSize;
        if size as vk::DeviceSize != self.size {
            let alloc = self.allocator.borrow();
            unsafe { alloc.destroy_buffer(self.buffer, self.allocation) };

            self.size = size;
            let (buffer, allocation) = Self::create_buffer(&alloc, size, self.usage);
            self.buffer = buffer;
            self.allocation = allocation;
        }

        self.upload_raw(arr.as_ptr(), size);
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
