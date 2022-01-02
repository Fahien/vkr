// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::vk;
use std::{cell::RefCell, ops::Deref, rc::Rc};

use super::*;

pub struct Buffer {
    allocation: vk_mem::Allocation,
    pub buffer: vk::Buffer,
    usage: vk::BufferUsageFlags,
    pub size: vk::DeviceSize,
    pub allocator: Rc<RefCell<vk_mem::Allocator>>,
}

impl Buffer {
    pub fn create_buffer(
        allocator: &vk_mem::Allocator,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
    ) -> (vk::Buffer, vk_mem::Allocation) {
        assert!(size >= 32);

        let buffer_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .build();

        // Vulkan memory
        let mut create_info = vk_mem::AllocationCreateInfo::default();
        create_info.usage = vk_mem::MemoryUsage::CpuToGpu;
        create_info.required_flags = vk::MemoryPropertyFlags::HOST_VISIBLE;
        create_info.preferred_flags =
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_CACHED;

        let (buffer, allocation, _) = allocator
            .create_buffer(&buffer_info, &create_info)
            .expect("Failed to create Vulkan buffer");

        (buffer, allocation)
    }

    /// Loads data from a png image in `path` directly into a staging buffer
    pub fn load(allocator: &Rc<RefCell<vk_mem::Allocator>>, png: &mut Png) -> Self {
        let size = png.info.buffer_size().max(32);
        let usage = vk::BufferUsageFlags::TRANSFER_SRC;

        // Create staging buffer
        let (buffer, allocation) =
            Self::create_buffer(&allocator.deref().borrow(), size as vk::DeviceSize, usage);

        let alloc = allocator.deref().borrow();
        let data = alloc
            .map_memory(&allocation)
            .expect("Failed to map Vulkan memory");

        // Allocate the output buffer
        let mut buf = unsafe { std::slice::from_raw_parts_mut(data, size) };

        // Read the next frame. An APNG might contain multiple frames.
        png.reader.next_frame(&mut buf).unwrap();

        alloc.unmap_memory(&allocation);

        Self {
            allocation,
            buffer,
            usage,
            size: size as vk::DeviceSize,
            allocator: allocator.clone(),
        }
    }

    pub fn new_with_size(
        allocator: &Rc<RefCell<vk_mem::Allocator>>,
        usage: vk::BufferUsageFlags,
        size: vk::DeviceSize,
    ) -> Self {
        assert!(size >= 32);
        let allocator = allocator.clone();
        let (buffer, allocation) = Self::create_buffer(&allocator.deref().borrow(), size, usage);

        Self {
            allocation,
            buffer,
            size,
            usage,
            allocator,
        }
    }

    pub fn new<T>(allocator: &Rc<RefCell<vk_mem::Allocator>>, usage: vk::BufferUsageFlags) -> Self {
        let size = std::mem::size_of::<T>() as vk::DeviceSize;
        let size = size.max(32);
        Self::new_with_size(allocator, usage, size)
    }

    pub fn new_arr<T>(
        allocator: &Rc<RefCell<vk_mem::Allocator>>,
        usage: vk::BufferUsageFlags,
        arr: &[T],
    ) -> Self {
        let size = (std::mem::size_of::<T>() * arr.len()) as vk::DeviceSize;
        let size = size.max(32);
        let mut buffer = Self::new_with_size(allocator, usage, size);
        buffer.upload_raw(arr.as_ptr(), size);
        buffer
    }

    pub fn from_data(
        allocator: &Rc<RefCell<vk_mem::Allocator>>,
        data: &[u8],
        usage: vk::BufferUsageFlags,
    ) -> Self {
        let size = data.len() as vk::DeviceSize;
        let size = size.max(32);
        let mut buffer = Self::new_with_size(allocator, usage, size);
        buffer.upload_arr(data);
        buffer
    }

    pub fn upload<T>(&mut self, data: &T) {
        self.upload_raw(data as *const T, std::mem::size_of::<T>() as vk::DeviceSize);
    }

    pub fn map<T>(&mut self) -> &[T] {
        let alloc = self.allocator.deref().borrow();
        let data = alloc
            .map_memory(&self.allocation)
            .expect("Failed to map Vulkan memory");
        let slice_size = self.size as usize / std::mem::size_of::<T>();
        unsafe { std::slice::from_raw_parts(data as _, slice_size) }
    }

    pub fn unmap(&mut self) {
        let alloc = self.allocator.deref().borrow();
        alloc.unmap_memory(&self.allocation);
    }

    pub fn upload_raw<T>(&mut self, src: *const T, size: vk::DeviceSize) {
        let alloc = self.allocator.deref().borrow();
        let data = alloc
            .map_memory(&self.allocation)
            .expect("Failed to map Vulkan memory");
        unsafe { data.copy_from(src as _, size as usize) };
        alloc.unmap_memory(&self.allocation);
    }

    pub fn upload_arr<T>(&mut self, arr: &[T]) {
        // Create a new buffer if not enough size for the vector
        let size = (arr.len() * std::mem::size_of::<T>()) as vk::DeviceSize;
        let size = size.max(32);
        if size as vk::DeviceSize != self.size {
            let alloc = self.allocator.deref().borrow();
            alloc.destroy_buffer(self.buffer, &self.allocation);

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
        self.allocator
            .deref()
            .borrow()
            .destroy_buffer(self.buffer, &self.allocation);
    }
}
