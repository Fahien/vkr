// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::vk;
use std::{cell::RefCell, ffi::CStr, ops::Deref, rc::Rc};

use super::*;

pub struct Dev {
    pub surface_format: ash::vk::SurfaceFormatKHR,
    pub graphics_command_pool: CommandPool,
    pub graphics_queue: Queue,
    /// Needs to be public if we want to create buffers outside this module.
    /// The allocator is shared between the various buffers to release resources on drop.
    /// Moreover it needs to be inside a RefCell, so we can mutably borrow it on destroy.
    pub allocator: Rc<RefCell<vk_mem::Allocator>>,
    pub device: Rc<ash::Device>,
    pub physical: ash::vk::PhysicalDevice,
}

impl Dev {
    fn get_graphics_queue_index(
        instance: &ash::Instance,
        physical: ash::vk::PhysicalDevice,
        surface: Option<&Surface>,
    ) -> u32 {
        // Queue information (instance, physical device)
        let queue_properties =
            unsafe { instance.get_physical_device_queue_family_properties(physical) };

        let mut graphics_queue_index = std::u32::MAX;

        for (i, queue) in queue_properties.iter().enumerate() {
            let mut supports_presentation = true;

            if let Some(surface) = surface {
                supports_presentation = unsafe {
                    surface.ext.get_physical_device_surface_support(
                        physical,
                        i as u32,
                        surface.surface,
                    )
                }
                .expect("Failed to check presentation support for Vulkan physical device");
            }

            if queue.queue_flags.contains(ash::vk::QueueFlags::GRAPHICS) && supports_presentation {
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

    pub fn new(ctx: &Ctx, surface: Option<&Surface>) -> Self {
        // Physical device
        let physical = {
            let phydevs = unsafe {
                ctx.instance
                    .enumerate_physical_devices()
                    .expect("Failed to enumerate Vulkan physical devices")
            };

            for physical in &phydevs {
                let properties = unsafe { ctx.instance.get_physical_device_properties(*physical) };
                let name = unsafe { CStr::from_ptr(properties.device_name.as_ptr()) };
                println!("Physical device: {:?}", name);
            }

            // Choose first one for now
            phydevs[0]
        };

        let graphics_queue_index = Dev::get_graphics_queue_index(&ctx.instance, physical, surface);

        // Logical device (physical device, surface, device required extensions (swapchain), queue information)
        let queue_infos = vec![ash::vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(graphics_queue_index)
            // Highest priority for a single graphics queue
            .queue_priorities(&[1.0])
            .build()];

        let mut device_create_info =
            ash::vk::DeviceCreateInfo::builder().queue_create_infos(&queue_infos);

        // Enable some extensions
        let mut enabled_extensions: Vec<*const i8> = vec![];

        let extension_properties =
            unsafe { ctx.instance.enumerate_device_extension_properties(physical) }
                .expect("Failed to enumerate Vulkan device extension properties");

        let mut vulkan_memory_model = false;

        for prop in extension_properties.iter() {
            let name = unsafe { CStr::from_ptr(prop.extension_name.as_ptr()) }
                .to_str()
                .unwrap();
            if name == "VK_KHR_vulkan_memory_model" {
                enabled_extensions.push(prop.extension_name.as_ptr());
                vulkan_memory_model = true;
            }
            else if name == "VK_KHR_portability_subset" {
                enabled_extensions.push(prop.extension_name.as_ptr());
            }
            println!("\t{}", name);
        }

        #[cfg(feature = "win")]
        enabled_extensions.push(ash::extensions::khr::Swapchain::name().as_ptr());

        device_create_info = device_create_info.enabled_extension_names(&enabled_extensions);

        // Used only if extension is available
        let mut vulkan_memory_model_features =
            ash::vk::PhysicalDeviceVulkanMemoryModelFeatures::builder()
                .vulkan_memory_model(true)
                .build();
        if vulkan_memory_model {
            device_create_info = device_create_info.push_next(&mut vulkan_memory_model_features);
        }

        let device_create_info = device_create_info.build();

        let device = unsafe {
            ctx.instance
                .create_device(physical, &device_create_info, None)
                .expect("Failed to create Vulkan logical device")
        };
        let device = Rc::new(device);

        let graphics_queue = Queue::new(&device, graphics_queue_index);

        // Command pool
        let graphics_command_pool = CommandPool::new(&device, graphics_queue_index);

        // Surface format
        let mut surface_format = vk::SurfaceFormatKHR::builder()
            .format(vk::Format::R8G8B8A8_SRGB)
            .color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .build();

        if let Some(surface) = surface {
            surface_format = {
                let surface_formats = unsafe {
                    surface
                        .ext
                        .get_physical_device_surface_formats(physical, surface.surface)
                }
                .expect("Failed to get Vulkan physical device surface formats");

                surface_formats[1]
            }
        };

        println!("Surface format: {:?}", surface_format.format);

        let allocator = {
            let create_info = vk_mem::AllocatorCreateInfo {
                physical_device: physical,
                device: device.deref().clone(),
                instance: ctx.instance.clone(),
                ..Default::default()
            };
            vk_mem::Allocator::new(&create_info)
        }
        .expect("Failed to create Vulkan allocator");

        Self {
            surface_format,
            graphics_command_pool,
            graphics_queue,
            allocator: Rc::new(RefCell::new(allocator)),
            device: device,
            physical,
        }
    }

    pub fn wait(&self) {
        unsafe {
            self.device
                .device_wait_idle()
                .expect("Failed to wait for Vulkan device");
        }
    }
}

impl Drop for Dev {
    fn drop(&mut self) {
        self.wait();
        self.allocator.deref().borrow_mut().destroy();
        self.graphics_command_pool.destroy();
        unsafe {
            self.device.destroy_device(None);
        }
    }
}
