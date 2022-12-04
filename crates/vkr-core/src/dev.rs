// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{
    ffi::{CStr, CString},
    rc::Rc,
};

use crate::{ctx::Ctx, surface::Surface};

use ash::vk;

pub struct Dev {
    pub surface_format: vk::SurfaceFormatKHR,
    pub graphics_command_pool: vk::CommandPool,
    pub graphics_queue: vk::Queue,
    pub physical: vk::PhysicalDevice,
    pub device: Rc<ash::Device>,
}

impl Dev {
    fn get_graphics_queue_index(
        instance: &ash::Instance,
        physical: vk::PhysicalDevice,
        surface: &Surface,
    ) -> u32 {
        // Queue information (instance, physical device)
        let queue_properties =
            unsafe { instance.get_physical_device_queue_family_properties(physical) };

        let mut graphics_queue_index = std::u32::MAX;

        for (i, queue) in queue_properties.iter().enumerate() {
            let supports_presentation = unsafe {
                surface
                    .ext
                    .get_physical_device_surface_support(physical, i as u32, surface.surface)
            }
            .expect("Failed to check presentation support for Vulkan physical device");
            if queue.queue_flags.contains(vk::QueueFlags::GRAPHICS) && supports_presentation {
                graphics_queue_index = i as u32;
                break;
            }
        }

        assert!(
            graphics_queue_index != std::u32::MAX,
            "Failed to find graphics queue"
        );

        graphics_queue_index
    }

    pub fn new(ctx: &Ctx, surface: &Surface) -> Self {
        // Physical device
        let physical = {
            let phydevs = unsafe {
                ctx.instance
                    .enumerate_physical_devices()
                    .expect("Failed to enumerate Vulkan physical devices")
            };
            phydevs[0]
        };
        let properties = unsafe { ctx.instance.get_physical_device_properties(physical) };
        let name = unsafe { CStr::from_ptr(properties.device_name.as_ptr()) };
        println!("Physical device: {:?}", name);

        let graphics_queue_index = Dev::get_graphics_queue_index(&ctx.instance, physical, surface);

        // Logical device (physical device, surface, device required extensions (swapchain), queue information)
        let queue_infos = vec![vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(graphics_queue_index)
            // Highest priority for a single graphics queue
            .queue_priorities(&[1.0])
            .build()];

        let portability_subset_name = CString::new("VK_KHR_portability_subset").unwrap();
        let device_extensions = [
            ash::extensions::khr::Swapchain::name().as_ptr(),
            portability_subset_name.as_ptr(),
        ];
        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_infos)
            .enabled_extension_names(&device_extensions);

        let device = unsafe {
            ctx.instance
                .create_device(physical, &device_create_info, None)
                .expect("Failed to create Vulkan logical device")
        };

        let graphics_queue = unsafe { device.get_device_queue(graphics_queue_index, 0) };

        // Command pool
        let create_info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(graphics_queue_index);
        let graphics_command_pool = {
            unsafe {
                device
                    .create_command_pool(&create_info, None)
                    .expect("Failed to create Vulkan command pool")
            }
        };

        // Surface format
        let surface_format = {
            let surface_formats = unsafe {
                surface
                    .ext
                    .get_physical_device_surface_formats(physical, surface.surface)
            }
            .expect("Failed to get Vulkan physical device surface formats");

            surface_formats[1]
        };
        println!("Surface format: {:?}", surface_format.format);

        Self {
            surface_format,
            graphics_command_pool,
            graphics_queue,
            physical,
            device: Rc::new(device),
        }
    }
}

impl Drop for Dev {
    fn drop(&mut self) {
        unsafe {
            self.device
                .destroy_command_pool(self.graphics_command_pool, None);
            self.device.destroy_device(None);
        }
    }
}
