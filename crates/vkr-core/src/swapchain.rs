// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::borrow::Borrow;

use super::*;

pub struct Swapchain {
    pub images: Vec<Image>,
    pub swapchain: ash::vk::SwapchainKHR,
    pub ext: ash::extensions::khr::Swapchain,
}

impl Swapchain {
    fn create_swapchain(
        ext: &ash::extensions::khr::Swapchain,
        surface: &Surface,
        dev: &Dev,
        width: u32,
        height: u32,
    ) -> ash::vk::SwapchainKHR {
        // This needs to be queried to prevent validation layers complaining
        let surface_capabilities = unsafe {
            surface
                .ext
                .get_physical_device_surface_capabilities(dev.physical, surface.surface)
        }
        .expect("Failed to get Vulkan physical device surface capabilities");

        let create_info = ash::vk::SwapchainCreateInfoKHR::builder()
            .surface(surface.surface)
            .min_image_count(3)
            .image_format(dev.surface_format.format)
            .image_color_space(dev.surface_format.color_space)
            .image_extent(
                ash::vk::Extent2D::builder()
                    .width(width)
                    .height(height)
                    .build(),
            )
            .image_array_layers(1)
            .image_usage(ash::vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(ash::vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(ash::vk::PresentModeKHR::FIFO)
            .clipped(true);
        unsafe { ext.create_swapchain(&create_info, None) }
            .expect("Failed to create Vulkan swapchain")
    }

    pub fn new(ctx: &Ctx, surface: &Surface, dev: &Dev, width: u32, height: u32) -> Self {
        // Swapchain (instance, logical device, surface formats)
        let device: &ash::Device = dev.device.borrow();
        let ext = ash::extensions::khr::Swapchain::new(&ctx.instance, device);

        let swapchain = Self::create_swapchain(&ext, surface, dev, width, height);

        let swapchain_images = unsafe { ext.get_swapchain_images(swapchain) }
            .expect("Failed to get Vulkan swapchain images");

        let mut images = Vec::new();
        for image in swapchain_images.into_iter() {
            images.push(Image::unmanaged(
                image,
                width,
                height,
                dev.surface_format.format,
                dev.surface_format.color_space,
            ));
        }

        Self {
            images,
            swapchain,
            ext,
        }
    }

    pub fn recreate(&mut self, surface: &Surface, dev: &Dev, width: u32, height: u32) {
        dev.wait();

        unsafe {
            self.ext.destroy_swapchain(self.swapchain, None);
        }

        self.swapchain = Self::create_swapchain(&self.ext, surface, dev, width, height);

        let swapchain_images = unsafe { self.ext.get_swapchain_images(self.swapchain) }
            .expect("Failed to get Vulkan swapchain images");

        self.images.clear();
        for image in swapchain_images.into_iter() {
            self.images.push(Image::unmanaged(
                image,
                width,
                height,
                dev.surface_format.format,
                dev.surface_format.color_space,
            ));
        }
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            self.ext.destroy_swapchain(self.swapchain, None);
        }
    }
}