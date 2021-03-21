// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{
    cell::RefCell,
    fs::{create_dir, File},
    path::Path,
    rc::Rc,
};

use ash::{version::DeviceV1_0, *};

use super::*;

pub struct Png {
    pub info: png::OutputInfo,
    pub reader: png::Reader<File>,
}

impl Png {
    /// Opens a PNG file without loading data yet
    pub fn open(path: &str) -> Self {
        let path = Path::new(path);
        let file = File::open(path).unwrap();

        let decoder = png::Decoder::new(file);
        let (info, reader) = decoder.read_info().unwrap();

        Self { info, reader }
    }
}

pub struct Image {
    /// Whether this image is manages and should be freed, or not (like swapchain images)
    managed: bool,
    pub image: ash::vk::Image,
    layout: ash::vk::ImageLayout,
    pub extent: ash::vk::Extent3D,
    pub format: ash::vk::Format,
    pub color_space: ash::vk::ColorSpaceKHR,
    allocation: Option<vk_mem::Allocation>,
    allocator: Option<Rc<RefCell<vk_mem::Allocator>>>,
}

impl Image {
    pub fn unmanaged(
        image: ash::vk::Image,
        width: u32,
        height: u32,
        format: ash::vk::Format,
        color_space: ash::vk::ColorSpaceKHR,
    ) -> Self {
        let extent = ash::vk::Extent3D::builder()
            .width(width)
            .height(height)
            .depth(1)
            .build();

        Self {
            managed: true,
            image,
            layout: vk::ImageLayout::UNDEFINED,
            extent,
            format,
            color_space,
            allocation: None,
            allocator: None,
        }
    }

    /// Creates a new empty image
    pub fn new(
        allocator: &Rc<RefCell<vk_mem::Allocator>>,
        width: u32,
        height: u32,
        format: ash::vk::Format,
    ) -> Self {
        let allocator = allocator.clone();

        let extent = ash::vk::Extent3D::builder()
            .width(width)
            .height(height)
            .depth(1)
            .build();

        let image_info = ash::vk::ImageCreateInfo::builder()
            .image_type(ash::vk::ImageType::TYPE_2D)
            .extent(extent)
            .mip_levels(1)
            .array_layers(1)
            .tiling(ash::vk::ImageTiling::OPTIMAL)
            .format(format)
            .initial_layout(ash::vk::ImageLayout::UNDEFINED)
            .usage(ash::vk::ImageUsageFlags::TRANSFER_DST | ash::vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
            .samples(ash::vk::SampleCountFlags::TYPE_1)
            .build();

        let mut alloc_info = vk_mem::AllocationCreateInfo::default();
        alloc_info.usage = vk_mem::MemoryUsage::GpuOnly;

        let (image, allocation, _) = allocator
            .borrow_mut()
            .create_image(&image_info, &alloc_info)
            .expect("Failed to create Vulkan image");

        Self {
            managed: true,
            image,
            layout: ash::vk::ImageLayout::UNDEFINED,
            extent,
            format,
            color_space: vk::ColorSpaceKHR::default(),
            allocation: Some(allocation),
            allocator: Some(allocator),
        }
    }

    pub fn load(dev: &Dev, path: &str) -> Self {
        let mut png = Png::open(path);
        let staging = Buffer::load(&dev.allocator, &mut png);
        let mut image = Image::new(
            &dev.allocator,
            png.info.width,
            png.info.height,
            vk::Format::R8G8B8A8_SRGB,
        );
        image.copy_from(&staging, dev);
        image
    }

    pub fn copy_from(&mut self, staging: &Buffer, dev: &Dev) {
        // @todo Use TRANSFER pool and transfer queue
        let command_buffer = unsafe {
            let alloc_info = ash::vk::CommandBufferAllocateInfo::builder()
                .command_pool(dev.graphics_command_pool)
                .level(ash::vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1)
                .build();
            let buffers = dev
                .device
                .allocate_command_buffers(&alloc_info)
                .expect("Failed to allocate Vulkan command buffer");
            buffers[0]
        };

        unsafe {
            let begin_info = ash::vk::CommandBufferBeginInfo::builder()
                .flags(ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
                .build();
            dev.device.begin_command_buffer(command_buffer, &begin_info)
        }
        .expect("Failed to begin Vulkan command buffer");

        // Undefined -> Transfer dst optimal
        unsafe {
            let new_layout = ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL;

            let src_stage_mask = ash::vk::PipelineStageFlags::TOP_OF_PIPE;
            let dst_stage_mask = ash::vk::PipelineStageFlags::TRANSFER;
            let dependency_flags = ash::vk::DependencyFlags::default();
            let image_memory_barriers = vec![ash::vk::ImageMemoryBarrier::builder()
                .old_layout(self.layout)
                .new_layout(new_layout)
                .image(self.image)
                .subresource_range(
                    ash::vk::ImageSubresourceRange::builder()
                        .aspect_mask(ash::vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1)
                        .build(),
                )
                .dst_access_mask(ash::vk::AccessFlags::TRANSFER_WRITE)
                .build()];
            dev.device.cmd_pipeline_barrier(
                command_buffer,
                src_stage_mask,
                dst_stage_mask,
                dependency_flags,
                &[],
                &[],
                &image_memory_barriers,
            );

            self.layout = new_layout;
        }

        // Copy
        unsafe {
            let dst_image_layout = self.layout;
            let regions = vec![ash::vk::BufferImageCopy::builder()
                .image_subresource(
                    ash::vk::ImageSubresourceLayers::builder()
                        .aspect_mask(ash::vk::ImageAspectFlags::COLOR)
                        .layer_count(1)
                        .build(),
                )
                .image_extent(self.extent)
                .build()];
            dev.device.cmd_copy_buffer_to_image(
                command_buffer,
                staging.buffer,
                self.image,
                dst_image_layout,
                &regions,
            );
        }

        // Transfer dst optimal -> Shader read only optimal
        unsafe {
            let new_layout = ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;

            let src_stage_mask = ash::vk::PipelineStageFlags::TRANSFER;
            let dst_stage_mask = ash::vk::PipelineStageFlags::FRAGMENT_SHADER;
            let dependency_flags = ash::vk::DependencyFlags::default();
            let image_memory_barriers = vec![ash::vk::ImageMemoryBarrier::builder()
                .old_layout(self.layout)
                .new_layout(new_layout)
                .image(self.image)
                .subresource_range(
                    ash::vk::ImageSubresourceRange::builder()
                        .aspect_mask(ash::vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1)
                        .build(),
                )
                .src_access_mask(ash::vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(ash::vk::AccessFlags::SHADER_READ)
                .build()];
            dev.device.cmd_pipeline_barrier(
                command_buffer,
                src_stage_mask,
                dst_stage_mask,
                dependency_flags,
                &[],
                &[],
                &image_memory_barriers,
            );

            self.layout = new_layout;
        }

        // End
        unsafe {
            dev.device
                .end_command_buffer(command_buffer)
                .expect("Failed to end Vulkan command buffer");
        }

        let mut fence = Fence::unsignaled(&dev.device);

        let submits = [ash::vk::SubmitInfo::builder()
            .command_buffers(&[command_buffer])
            .build()];
        dev.graphics_queue.submit(&submits, Some(&mut fence));

        fence.wait();

        unsafe {
            dev.device
                .free_command_buffers(dev.graphics_command_pool, &[command_buffer]);
        }
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        if self.managed {
            if let Some(alloc) = &self.allocator {
                alloc
                    .borrow_mut()
                    .destroy_image(self.image, &self.allocation.unwrap());
            }
        }
    }
}

pub struct ImageView {
    pub view: vk::ImageView,
    device: Rc<Device>,
}

impl ImageView {
    pub fn new(device: &Rc<Device>, image: &Image) -> Self {
        let device = device.clone();

        let create_info = vk::ImageViewCreateInfo::builder()
            .image(image.image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(image.format)
            .subresource_range(
                vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1)
                    .build(),
            )
            .build();

        let view = unsafe { device.create_image_view(&create_info, None) }
            .expect("Failed to create Vulkan image view");

        Self { view, device }
    }
}

impl Drop for ImageView {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image_view(self.view, None);
        }
    }
}

pub struct Texture {
    pub view: Handle<ImageView>,
    pub sampler: Handle<Sampler>,
}

impl Texture {
    pub fn new(view: Handle<ImageView>, sampler: Handle<Sampler>) -> Self {
        Self { view, sampler }
    }
}

#[cfg(test)]
mod test {
    use std::{fs::File, io::BufWriter, path::Path};

    #[test]
    fn save_png() {
        let image_dir = Path::new(r"res/image");
        if !Path::exists(&image_dir) {
            create_dir(image_dir).expect("Failed to create image directory");
        }

        let path = Path::new(r"res/image/test.png");
        let file = File::create(path).unwrap();
        let ref mut w = BufWriter::new(file);

        let mut encoder = png::Encoder::new(w, 2, 2);
        encoder.set_color(png::ColorType::RGBA);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header().unwrap();
        // 4 pixels
        let data = [
            180, 100, 10, 255, 20, 190, 10, 205, 40, 10, 200, 255, 80, 100, 200, 255,
        ];
        writer.write_image_data(&data).unwrap();
    }

    #[test]
    fn test_copy_image() {
        // TODO a CTX without any window
    }
}
