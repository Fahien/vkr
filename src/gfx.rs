// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::{extensions::ext::DebugReport, vk::Handle};
use sdl2 as sdl;
use std::{
    borrow::{Borrow, BorrowMut},
    ffi::{c_void, CStr, CString},
    os::raw::c_char,
    rc::Rc,
};

#[repr(C)]
pub struct Vec3f {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3f {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3f { x, y, z }
    }
}

#[repr(C)]
pub struct Vertex {
    pos: Vec3f,
}

impl Vertex {
    pub fn new(x: f32, y: f32, z: f32) -> Vertex {
        Vertex {
            pos: Vec3f::new(x, y, z),
        }
    }
}

pub unsafe extern "system" fn vk_debug(
    _: ash::vk::DebugReportFlagsEXT,
    _: ash::vk::DebugReportObjectTypeEXT,
    _: u64,
    _: usize,
    _: i32,
    _: *const c_char,
    message: *const c_char,
    _: *mut c_void,
) -> u32 {
    eprintln!("{:?}", CStr::from_ptr(message));
    ash::vk::FALSE
}

pub struct Win {
    pub window: sdl::video::Window,
    pub video: sdl::VideoSubsystem,
    pub ctx: sdl::Sdl,
}

impl Win {
    pub fn new() -> Self {
        let ctx = sdl::init().expect("Failed to initialize SDL");
        let video = ctx.video().expect("Failed to initialize SDL video");
        let window = video
            .window("Test", 480, 480)
            .vulkan()
            .position_centered()
            .build()
            .expect("Failed to build SDL window");

        Self { window, video, ctx }
    }
}

pub struct Debug {
    loader: DebugReport,
    callback: ash::vk::DebugReportCallbackEXT,
}

impl Debug {
    fn new(ctx: &Ctx) -> Self {
        let debug_info = ash::vk::DebugReportCallbackCreateInfoEXT::builder()
            .flags(
                ash::vk::DebugReportFlagsEXT::ERROR
                    | ash::vk::DebugReportFlagsEXT::WARNING
                    | ash::vk::DebugReportFlagsEXT::PERFORMANCE_WARNING,
            )
            .pfn_callback(Some(vk_debug));

        let loader = { DebugReport::new(&ctx.entry, &ctx.instance) };
        let callback = unsafe {
            loader
                .create_debug_report_callback(&debug_info, None)
                .expect("Failed to create Vulkan debug callback")
        };

        Self { loader, callback }
    }
}

impl Drop for Debug {
    fn drop(&mut self) {
        unsafe {
            self.loader
                .destroy_debug_report_callback(self.callback, None);
        }
    }
}

pub struct Ctx {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
}

impl Ctx {
    pub fn new(win: &Win) -> Self {
        let extensions = win
            .window
            .vulkan_instance_extensions()
            .expect("Failed to get SDL vulkan extensions");
        let mut extensions_names = vec![DebugReport::name().as_ptr()];
        for ext in extensions.iter() {
            extensions_names.push(ext.as_ptr() as *const i8);
        }
        let layers = [CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
        let layer_names: Vec<*const i8> = layers.iter().map(|name| name.as_ptr()).collect();

        let entry = unsafe { ash::Entry::new() }.expect("Failed to create ash entry");
        let app_info = ash::vk::ApplicationInfo {
            p_application_name: "Test" as *const str as _,
            api_version: ash::vk::make_api_version(0, 1, 2, 0),
            ..Default::default()
        };
        let create_info = ash::vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&extensions_names)
            .enabled_layer_names(&layer_names);
        let instance = unsafe { entry.create_instance(&create_info, None) }
            .expect("Failed to create Vulkan instance");

        Self { entry, instance }
    }
}

pub struct Vkr {
    pub debug: Debug,
    pub ctx: Ctx,
}

impl Vkr {
    pub fn new(win: &Win) -> Self {
        let ctx = Ctx::new(win);
        let debug = Debug::new(&ctx);

        Self { ctx, debug }
    }
}

impl Drop for Ctx {
    fn drop(&mut self) {
        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}

pub struct Surface {
    pub surface: ash::vk::SurfaceKHR,
    pub ext: ash::extensions::khr::Surface,
}

impl Surface {
    pub fn new(win: &Win, ctx: &Ctx) -> Self {
        let surface = win
            .window
            .vulkan_create_surface(ctx.instance.handle().as_raw() as usize)
            .expect("Failed to create surface");
        let surface: ash::vk::SurfaceKHR = ash::vk::Handle::from_raw(surface);
        let ext = ash::extensions::khr::Surface::new(&ctx.entry, &ctx.instance);

        Self { surface, ext }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.ext.destroy_surface(self.surface, None);
        }
    }
}

pub struct Image {
    pub image: ash::vk::Image,
    pub format: ash::vk::Format,
    pub color_space: ash::vk::ColorSpaceKHR,
    pub width: u32,
    pub height: u32,
}

impl Image {
    pub fn new(
        image: ash::vk::Image,
        format: ash::vk::Format,
        color_space: ash::vk::ColorSpaceKHR,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            image,
            format,
            color_space,
            width,
            height,
        }
    }
}

pub struct Swapchain {
    pub images: Vec<Image>,
    pub swapchain: ash::vk::SwapchainKHR,
    pub ext: ash::extensions::khr::Swapchain,
}

impl Swapchain {
    pub fn new(ctx: &Ctx, surface: &Surface, dev: &Dev, width: u32, height: u32) -> Self {
        // Swapchain (instance, logical device, surface formats)
        let device: &ash::Device = dev.device.borrow();
        let ext = ash::extensions::khr::Swapchain::new(&ctx.instance, device);

        // This needs to be queried to prevent validation layers complaining
        let surface_capabilities = unsafe {
            surface
                .ext
                .get_physical_device_surface_capabilities(dev.physical, surface.surface)
        }
        .expect("Failed to get Vulkan physical device surface capabilities");
        println!(
            "Surface transform: {:?}",
            surface_capabilities.current_transform
        );

        let swapchain = {
            let create_info = ash::vk::SwapchainCreateInfoKHR::builder()
                .surface(surface.surface)
                .min_image_count(2)
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
        };

        let swapchain_images = unsafe { ext.get_swapchain_images(swapchain) }
            .expect("Failed to get Vulkan swapchain images");

        let mut images = Vec::new();
        for image in swapchain_images.into_iter() {
            images.push(Image::new(
                image,
                dev.surface_format.format,
                dev.surface_format.color_space,
                width,
                height,
            ));
        }

        Self {
            images,
            swapchain,
            ext,
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

pub struct Dev {
    pub surface_format: ash::vk::SurfaceFormatKHR,
    pub graphics_command_pool: ash::vk::CommandPool,
    pub graphics_queue: ash::vk::Queue,
    physical: ash::vk::PhysicalDevice,
    pub device: Rc<ash::Device>,
}

impl Dev {
    fn get_graphics_queue_index(
        instance: &ash::Instance,
        physical: ash::vk::PhysicalDevice,
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
        let queue_infos = vec![ash::vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(graphics_queue_index)
            // Highest priority for a single graphics queue
            .queue_priorities(&[1.0])
            .build()];
        let device_extensions = [ash::extensions::khr::Swapchain::name().as_ptr()];
        let device_create_info = ash::vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_infos)
            .enabled_extension_names(&device_extensions);

        let device = unsafe {
            ctx.instance
                .create_device(physical, &device_create_info, None)
                .expect("Failed to create Vulkan logical device")
        };

        let graphics_queue = unsafe { device.get_device_queue(graphics_queue_index, 0) };

        // Command pool
        let create_info = ash::vk::CommandPoolCreateInfo::builder()
            .flags(ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
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

pub struct Pass {
    pub render: ash::vk::RenderPass,
    device: Rc<ash::Device>,
}

impl Pass {
    pub fn new(dev: &mut Dev) -> Self {
        // Render pass (swapchain surface format, device)
        let attachment = [ash::vk::AttachmentDescription::builder()
            .format(dev.surface_format.format)
            .samples(ash::vk::SampleCountFlags::TYPE_1)
            .load_op(ash::vk::AttachmentLoadOp::CLEAR)
            .store_op(ash::vk::AttachmentStoreOp::STORE)
            .stencil_load_op(ash::vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(ash::vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(ash::vk::ImageLayout::UNDEFINED)
            .final_layout(ash::vk::ImageLayout::PRESENT_SRC_KHR)
            .build()];

        let attach_refs = [ash::vk::AttachmentReference::builder()
            .attachment(0)
            .layout(ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build()];

        // Just one subpass
        let subpasses = [ash::vk::SubpassDescription::builder()
            .pipeline_bind_point(ash::vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&attach_refs)
            .build()];

        let present_dependency = ash::vk::SubpassDependency::builder()
            .src_subpass(ash::vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(ash::vk::AccessFlags::empty())
            .dst_stage_mask(ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(ash::vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .build();

        let dependencies = [present_dependency];

        // Build the render pass
        let create_info = ash::vk::RenderPassCreateInfo::builder()
            .attachments(&attachment)
            .subpasses(&subpasses)
            .dependencies(&dependencies)
            .build();
        let render = unsafe {
            dev.device
                .borrow_mut()
                .create_render_pass(&create_info, None)
        }
        .expect("Failed to create Vulkan render pass");

        Self {
            render,
            device: Rc::clone(&dev.device),
        }
    }
}

impl Drop for Pass {
    fn drop(&mut self) {
        unsafe {
            self.device
                .borrow_mut()
                .destroy_render_pass(self.render, None);
        }
    }
}

pub struct Buffer {
    memory: ash::vk::DeviceMemory,
    pub buffer: ash::vk::Buffer,
    pub size: u64,
    device: Rc<ash::Device>,
}

impl Buffer {
    pub fn new(ctx: &Ctx, dev: &mut Dev) -> Self {
        // Vertex buffer of triangle to draw
        let size = std::mem::size_of::<Vertex>() as u64 * 3;
        let buffer_create_info = ash::vk::BufferCreateInfo::builder()
            .size(size)
            .usage(ash::vk::BufferUsageFlags::VERTEX_BUFFER)
            .sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
            .build();
        let buffer = unsafe {
            dev.device
                .borrow_mut()
                .create_buffer(&buffer_create_info, None)
        }
        .expect("Failed to create Vulkan vertex buffer");

        let requirements = unsafe {
            dev.device
                .borrow_mut()
                .get_buffer_memory_requirements(buffer)
        };

        let memory_type_index: u32 = {
            let mut mem_index: u32 = 0;
            let memory_properties = unsafe {
                ctx.instance
                    .get_physical_device_memory_properties(dev.physical)
            };
            for (i, memtype) in memory_properties.memory_types.iter().enumerate() {
                let res: ash::vk::MemoryPropertyFlags = memtype.property_flags
                    & (ash::vk::MemoryPropertyFlags::HOST_VISIBLE
                        | ash::vk::MemoryPropertyFlags::HOST_COHERENT);
                if (requirements.memory_type_bits & (1 << i) != 0) && res.as_raw() != 0 {
                    mem_index = i as u32;
                }
            }
            mem_index
        };
        if memory_type_index == 0 {
            panic!("Failed to find Vulkan memory type index");
        }

        let mem_allocate_info = ash::vk::MemoryAllocateInfo::builder()
            .allocation_size(requirements.size)
            .memory_type_index(memory_type_index)
            .build();
        let memory = unsafe {
            dev.device
                .borrow_mut()
                .allocate_memory(&mem_allocate_info, None)
        }
        .expect("Failed to allocate Vulkan memory");

        let offset = ash::vk::DeviceSize::default();
        unsafe {
            dev.device
                .borrow_mut()
                .bind_buffer_memory(buffer, memory, offset)
        }
        .expect("Failed to bind Vulkan memory to buffer");

        Self {
            memory,
            buffer,
            size,
            device: Rc::clone(&dev.device),
        }
    }

    pub fn upload<T>(&mut self, src: *const T, size: usize) {
        let device_size = ash::vk::DeviceSize::from(self.size);
        let flags = ash::vk::MemoryMapFlags::default();
        let data = unsafe { self.device.map_memory(self.memory, 0, device_size, flags) }
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
            self.device.borrow_mut().free_memory(self.memory, None);
            self.device.borrow_mut().destroy_buffer(self.buffer, None);
        }
    }
}
