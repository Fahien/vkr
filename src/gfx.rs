// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::{
    extensions::ext::DebugReport,
    version::{DeviceV1_0, EntryV1_0, InstanceV1_0},
    vk::Handle,
};
use byteorder::{ByteOrder, NativeEndian};
use nalgebra as na;
use sdl2 as sdl;
use std::{
    borrow::{Borrow, BorrowMut},
    cell::RefCell,
    collections::HashMap,
    ffi::{c_void, CStr, CString},
    ops::Deref,
    os::raw::c_char,
    rc::Rc,
};

use super::model::*;

#[repr(C)]
pub struct Ubo {
    pub matrix: na::Matrix4<f32>,
}

impl Ubo {
    pub fn _new() -> Self {
        Ubo {
            matrix: na::Matrix4::identity(),
        }
    }
}

/// Per-frame resource which contains a descriptor pool and a vector
/// of descriptor sets of each pipeline layout used for rendering.
struct Descriptors {
    pub sets: HashMap<ash::vk::PipelineLayout, Vec<ash::vk::DescriptorSet>>,
    pool: ash::vk::DescriptorPool,
    device: Rc<ash::Device>,
}

impl Descriptors {
    pub fn new(dev: &mut Dev) -> Self {
        let pool = unsafe {
            let pool_size = ash::vk::DescriptorPoolSize::builder()
                // Just one for the moment
                .descriptor_count(1)
                .ty(ash::vk::DescriptorType::UNIFORM_BUFFER)
                .build();
            let pool_sizes = vec![pool_size, pool_size];
            let create_info = ash::vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(&pool_sizes)
                // Support 4 different pipeline layouts
                .max_sets(2)
                .build();
            dev.device.create_descriptor_pool(&create_info, None)
        }
        .expect("Failed to create Vulkan descriptor pool");

        Self {
            sets: HashMap::new(),
            pool,
            device: dev.device.clone(),
        }
    }

    fn allocate(
        &mut self,
        layouts: &[ash::vk::DescriptorSetLayout],
    ) -> Vec<ash::vk::DescriptorSet> {
        let create_info = ash::vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.pool)
            .set_layouts(layouts)
            .build();

        unsafe { self.device.allocate_descriptor_sets(&create_info) }
            .expect("Failed to allocate Vulkan descriptor sets")
    }
}

impl Drop for Descriptors {
    fn drop(&mut self) {
        unsafe { self.device.destroy_descriptor_pool(self.pool, None) };
    }
}

/// This is the one that is going to be recreated
/// when the swapchain goes out of date
pub struct Framebuffer {
    pub area: ash::vk::Rect2D,
    // @todo Make a map of framebuffers indexed by render-pass as key
    pub framebuffer: ash::vk::Framebuffer,
    pub image_view: ash::vk::ImageView,
    pub image: Rc<RefCell<Image>>,
    device: Rc<ash::Device>,
}

impl Framebuffer {
    pub fn new(dev: &mut Dev, image: Rc<RefCell<Image>>, pass: &Pass) -> Self {
        let image_ref = image.deref().borrow();

        // Image view into a swapchain images (device, image, format)
        let image_view = {
            let create_info = ash::vk::ImageViewCreateInfo::builder()
                .image(image_ref.image)
                .view_type(ash::vk::ImageViewType::TYPE_2D)
                .format(image_ref.format)
                .components(
                    ash::vk::ComponentMapping::builder()
                        .r(ash::vk::ComponentSwizzle::IDENTITY)
                        .g(ash::vk::ComponentSwizzle::IDENTITY)
                        .b(ash::vk::ComponentSwizzle::IDENTITY)
                        .a(ash::vk::ComponentSwizzle::IDENTITY)
                        .build(),
                )
                .subresource_range(
                    ash::vk::ImageSubresourceRange::builder()
                        .aspect_mask(ash::vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1)
                        .build(),
                );
            unsafe {
                dev.device
                    .borrow_mut()
                    .create_image_view(&create_info, None)
            }
            .expect("Failed to create Vulkan image view")
        };

        // Framebuffers (image_view, renderpass)
        let framebuffer = {
            let attachments = [image_view];

            let create_info = ash::vk::FramebufferCreateInfo::builder()
                .render_pass(pass.render)
                .attachments(&attachments)
                .width(image_ref.width)
                .height(image_ref.height)
                .layers(1)
                .build();

            unsafe {
                dev.device
                    .borrow_mut()
                    .create_framebuffer(&create_info, None)
            }
            .expect("Failed to create Vulkan framebuffer")
        };

        // Needed by cmd_begin_render_pass
        let area = ash::vk::Rect2D::builder()
            .offset(ash::vk::Offset2D::builder().x(0).y(0).build())
            .extent(
                ash::vk::Extent2D::builder()
                    .width(image_ref.width)
                    .height(image_ref.height)
                    .build(),
            )
            .build();

        drop(image_ref);

        Self {
            area,
            framebuffer,
            image_view,
            image,
            device: Rc::clone(&dev.device),
        }
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            self.device
                .device_wait_idle()
                .expect("Failed to wait for device");
            self.device.destroy_framebuffer(self.framebuffer, None);
            self.device.destroy_image_view(self.image_view, None);
        }
    }
}

/// Frame resources that do not need to be recreated
/// when the swapchain goes out of date
pub struct Frameres {
    descriptors: Descriptors,
    pub command_buffer: ash::vk::CommandBuffer,
    pub fence: ash::vk::Fence,
    pub can_wait: bool,
    pub image_ready: ash::vk::Semaphore,
    pub image_drawn: ash::vk::Semaphore,
    device: Rc<ash::Device>,
}

impl Frameres {
    pub fn new(dev: &mut Dev) -> Self {
        // Graphics command buffer (device, command pool)
        let command_buffer = {
            let alloc_info = ash::vk::CommandBufferAllocateInfo::builder()
                .command_pool(dev.graphics_command_pool)
                .level(ash::vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);

            let buffers = unsafe {
                dev.device
                    .borrow_mut()
                    .allocate_command_buffers(&alloc_info)
            }
            .expect("Failed to allocate command buffer");
            buffers[0]
        };

        // Fence (device)
        let fence = {
            let create_info = ash::vk::FenceCreateInfo::builder()
                .flags(ash::vk::FenceCreateFlags::SIGNALED)
                .build();
            unsafe { dev.device.borrow_mut().create_fence(&create_info, None) }
        }
        .expect("Failed to create Vulkan fence");

        // Semaphores (device)
        let (image_ready, image_drawn) = {
            let create_info = ash::vk::SemaphoreCreateInfo::builder().build();
            unsafe {
                (
                    dev.device
                        .borrow_mut()
                        .create_semaphore(&create_info, None)
                        .expect("Failed to create Vulkan semaphore"),
                    dev.device
                        .borrow_mut()
                        .create_semaphore(&create_info, None)
                        .expect("Failed to create Vulkan semaphore"),
                )
            }
        };

        Self {
            descriptors: Descriptors::new(dev),
            command_buffer,
            fence,
            can_wait: true,
            image_ready,
            image_drawn,
            device: Rc::clone(&dev.device),
        }
    }

    pub fn wait(&mut self) {
        if !self.can_wait {
            return;
        }

        let device: &ash::Device = self.device.borrow();
        unsafe {
            device
                .wait_for_fences(&[self.fence], true, u64::max_value())
                .expect("Failed to wait for Vulkan frame fence");
            device
                .reset_fences(&[self.fence])
                .expect("Failed to reset Vulkan frame fence");
        }
        self.can_wait = false;
    }
}

impl Drop for Frameres {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_semaphore(self.image_drawn, None);
            self.device.destroy_semaphore(self.image_ready, None);
            self.device.destroy_fence(self.fence, None)
        }
    }
}

pub struct Frame {
    pub buffer: Framebuffer,
    pub res: Frameres,
    // @todo An ubo for each node
    pub ubo: Buffer,
    pub device: Rc<ash::Device>,
}

impl Frame {
    pub fn new(dev: &mut Dev, image: Rc<RefCell<Image>>, pass: &Pass) -> Self {
        let buffer = Framebuffer::new(dev, image, pass);
        let res = Frameres::new(dev);

        Frame {
            buffer,
            res,
            ubo: Buffer::new::<Ubo>(dev, ash::vk::BufferUsageFlags::UNIFORM_BUFFER),
            device: Rc::clone(&dev.device),
        }
    }

    pub fn begin(&self, pass: &Pass) {
        let begin_info = ash::vk::CommandBufferBeginInfo::builder().build();
        unsafe {
            self.device
                .begin_command_buffer(self.res.command_buffer, &begin_info)
        }
        .expect("Failed to begin Vulkan command buffer");

        let mut clear = ash::vk::ClearValue::default();
        clear.color.float32 = [0.025, 0.025, 0.025, 1.0];
        let clear_values = [clear];
        let create_info = ash::vk::RenderPassBeginInfo::builder()
            .framebuffer(self.buffer.framebuffer)
            .render_pass(pass.render)
            .render_area(self.buffer.area)
            .clear_values(&clear_values)
            .build();
        // Record it in the main command buffer
        let contents = ash::vk::SubpassContents::INLINE;
        unsafe {
            self.device
                .cmd_begin_render_pass(self.res.command_buffer, &create_info, contents)
        };
    }

    pub fn draw(&mut self, pipeline: &Pipeline, buffer: &Buffer) {
        let graphics_bind_point = ash::vk::PipelineBindPoint::GRAPHICS;
        unsafe {
            self.device.cmd_bind_pipeline(
                self.res.command_buffer,
                graphics_bind_point,
                pipeline.graphics,
            );
        }

        if let Some(sets) = self.res.descriptors.sets.get(&pipeline.layout) {
            unsafe {
                self.device.cmd_bind_descriptor_sets(
                    self.res.command_buffer,
                    graphics_bind_point,
                    pipeline.layout,
                    0,
                    sets,
                    &[],
                );
            }
        } else {
            let sets = self.res.descriptors.allocate(&[pipeline.set_layout]);

            // Update immediately the descriptor sets
            let buffer_info = ash::vk::DescriptorBufferInfo::builder()
                .range(std::mem::size_of::<Ubo>() as ash::vk::DeviceSize)
                .buffer(self.ubo.buffer)
                .build();

            let descriptor_write = ash::vk::WriteDescriptorSet::builder()
                .dst_set(sets[0])
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(ash::vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&[buffer_info])
                .build();

            unsafe {
                self.device.update_descriptor_sets(&[descriptor_write], &[]);

                self.device.cmd_bind_descriptor_sets(
                    self.res.command_buffer,
                    graphics_bind_point,
                    pipeline.layout,
                    0,
                    &sets,
                    &[],
                );
            }
            self.res.descriptors.sets.insert(pipeline.layout, sets);
        }

        let first_binding = 0;
        let buffers = [buffer.buffer];
        let offsets = [ash::vk::DeviceSize::default()];
        unsafe {
            self.device.cmd_bind_vertex_buffers(
                self.res.command_buffer,
                first_binding,
                &buffers,
                &offsets,
            );
        }

        let vertex_count = buffer.size as u32 / std::mem::size_of::<Vertex>() as u32;
        unsafe {
            self.device
                .cmd_draw(self.res.command_buffer, vertex_count, 1, 0, 0);
        }
    }

    pub fn end(&self) {
        unsafe {
            self.device.cmd_end_render_pass(self.res.command_buffer);
            self.device
                .end_command_buffer(self.res.command_buffer)
                .expect("Failed to end command buffer");
        }
    }

    pub fn present(
        &mut self,
        dev: &Dev,
        swapchain: &Swapchain,
        image_index: u32,
    ) -> Result<(), ash::vk::Result> {
        // Wait for the image to be available ..
        let wait_semaphores = [self.res.image_ready];
        // .. at color attachment output stage
        let wait_dst_stage_mask = [ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers = [self.res.command_buffer];
        let signal_semaphores = [self.res.image_drawn];
        let submits = [ash::vk::SubmitInfo::builder()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_dst_stage_mask)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores)
            .build()];
        unsafe {
            self.device
                .queue_submit(dev.graphics_queue, &submits, self.res.fence)
        }
        .expect("Failed to submit to Vulkan queue");

        self.res.can_wait = true;

        // Present result
        let pres_image_indices = [image_index];
        let pres_swapchains = [swapchain.swapchain];
        let pres_semaphores = [self.res.image_drawn];
        let present_info = ash::vk::PresentInfoKHR::builder()
            .image_indices(&pres_image_indices)
            .swapchains(&pres_swapchains)
            .wait_semaphores(&pres_semaphores);

        match unsafe {
            swapchain
                .ext
                .queue_present(dev.graphics_queue, &present_info)
        } {
            Ok(_subotimal) => Ok(()),
            Err(result) => Err(result),
        }
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        unsafe {
            self.device
                .device_wait_idle()
                .expect("Failed to wait for device");
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
            .window("Test", 480, 320)
            .vulkan()
            .position_centered()
            .resizable()
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

        let entry = ash::Entry::new().expect("Failed to create ash entry");
        let app_info = ash::vk::ApplicationInfo {
            p_application_name: "Test" as *const str as _,
            api_version: ash::vk::make_version(1, 0, 0),
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
    pub images: Vec<Rc<RefCell<Image>>>,
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
            images.push(Rc::new(RefCell::new(Image::new(
                image,
                dev.surface_format.format,
                dev.surface_format.color_space,
                width,
                height,
            ))));
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
    graphics_command_pool: ash::vk::CommandPool,
    graphics_queue: ash::vk::Queue,
    /// Needs to be public if we want to create buffers outside this module.
    /// The allocator is shared between the various buffers to release resources on drop.
    /// Moreover it needs to be inside a RefCell, so we can mutably borrow it on destroy.
    pub allocator: Rc<RefCell<vk_mem::Allocator>>,
    device: Rc<ash::Device>,
    physical: ash::vk::PhysicalDevice,
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

        let allocator = {
            let create_info = vk_mem::AllocatorCreateInfo {
                physical_device: physical,
                device: device.clone(),
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
            device: Rc::new(device),
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
        unsafe {
            self.device
                .destroy_command_pool(self.graphics_command_pool, None);
            self.device.destroy_device(None);
        }
    }
}

pub struct Pass {
    render: ash::vk::RenderPass,
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

pub struct Pipeline {
    graphics: ash::vk::Pipeline,
    layout: ash::vk::PipelineLayout,
    set_layout: ash::vk::DescriptorSetLayout,
    device: Rc<ash::Device>,
}

impl Pipeline {
    pub fn new<T: VertexInput>(
        dev: &mut Dev,
        topology: ash::vk::PrimitiveTopology,
        pass: &Pass,
        width: u32,
        height: u32,
    ) -> Self {
        let set_layout_bindings = ash::vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(ash::vk::DescriptorType::UNIFORM_BUFFER) // delta time?
            .descriptor_count(1) // Referring the shader
            .stage_flags(ash::vk::ShaderStageFlags::VERTEX)
            .build();
        let arr_bindings = vec![set_layout_bindings];

        let set_layout_info =
            ash::vk::DescriptorSetLayoutCreateInfo::builder().bindings(&arr_bindings);

        let set_layout = unsafe {
            dev.device
                .borrow_mut()
                .create_descriptor_set_layout(&set_layout_info, None)
        }
        .expect("Failed to create Vulkan descriptor set layout");

        let set_layouts = vec![set_layout];

        // Pipeline layout (device, descriptorset layouts, shader reflection?)
        let layout = {
            let create_info = ash::vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&set_layouts)
                .build();
            unsafe {
                dev.device
                    .borrow_mut()
                    .create_pipeline_layout(&create_info, None)
            }
            .expect("Failed to create Vulkan pipeline layout")
        };

        // Graphics pipeline (shaders, renderpass)
        let graphics = {
            const SHADERS: &[u8] = include_bytes!(env!("vkr_shaders.spv"));
            let mut rs_code = vec![0; SHADERS.len() / std::mem::size_of::<u32>()];
            NativeEndian::read_u32_into(SHADERS, rs_code.as_mut_slice());

            let create_info = ash::vk::ShaderModuleCreateInfo::builder()
                .code(rs_code.as_slice())
                .build();
            let rs_mod = unsafe {
                dev.device
                    .borrow_mut()
                    .create_shader_module(&create_info, None)
            }
            .expect("Failed to create Vulkan shader module");

            let entrypoint = CString::new("main_vs").expect("Failed to create main entrypoint");
            let vert_stage = ash::vk::PipelineShaderStageCreateInfo::builder()
                .stage(ash::vk::ShaderStageFlags::VERTEX)
                .module(rs_mod)
                .name(&entrypoint)
                .build();
            let entrypoint = CString::new("main_fs").expect("Failed to create main entrypoint");
            let frag_stage = ash::vk::PipelineShaderStageCreateInfo::builder()
                .stage(ash::vk::ShaderStageFlags::FRAGMENT)
                .module(rs_mod)
                .name(&entrypoint)
                .build();

            let vertex_binding = T::get_bindings();
            let vertex_attributes = T::get_attributes();

            let vertex_binding = [vertex_binding];

            let vertex_input = ash::vk::PipelineVertexInputStateCreateInfo::builder()
                .vertex_attribute_descriptions(&vertex_attributes)
                .vertex_binding_descriptions(&vertex_binding)
                .build();

            let input_assembly = ash::vk::PipelineInputAssemblyStateCreateInfo::builder()
                .topology(topology)
                .primitive_restart_enable(false)
                .build();

            let raster_state = ash::vk::PipelineRasterizationStateCreateInfo::builder()
                .depth_clamp_enable(false)
                .rasterizer_discard_enable(false)
                .polygon_mode(ash::vk::PolygonMode::FILL)
                .cull_mode(ash::vk::CullModeFlags::NONE)
                .front_face(ash::vk::FrontFace::COUNTER_CLOCKWISE)
                .depth_bias_enable(false)
                .line_width(1.0)
                .build();

            let viewport = [ash::vk::Viewport::builder()
                .x(0.0)
                .y(0.0)
                .width(width as f32)
                .height(height as f32)
                .min_depth(0.0)
                .max_depth(1.0)
                .build()];

            let scissor = [ash::vk::Rect2D::builder()
                .offset(ash::vk::Offset2D::builder().x(0).y(0).build())
                .extent(
                    ash::vk::Extent2D::builder()
                        .width(width)
                        .height(height)
                        .build(),
                )
                .build()];

            let view_state = ash::vk::PipelineViewportStateCreateInfo::builder()
                .viewports(&viewport)
                .scissors(&scissor)
                .build();

            let multisample_state = ash::vk::PipelineMultisampleStateCreateInfo::builder()
                .rasterization_samples(ash::vk::SampleCountFlags::TYPE_1)
                .sample_shading_enable(false)
                .alpha_to_coverage_enable(false)
                .alpha_to_one_enable(false)
                .build();

            let blend_attachment = [ash::vk::PipelineColorBlendAttachmentState::builder()
                .blend_enable(false)
                .color_write_mask(ash::vk::ColorComponentFlags::all())
                .build()];

            let blend_state = ash::vk::PipelineColorBlendStateCreateInfo::builder()
                .logic_op_enable(false)
                .attachments(&blend_attachment)
                .build();

            let stages = [vert_stage, frag_stage];

            let create_info = [ash::vk::GraphicsPipelineCreateInfo::builder()
                .stages(&stages)
                .vertex_input_state(&vertex_input)
                .input_assembly_state(&input_assembly)
                .viewport_state(&view_state)
                .rasterization_state(&raster_state)
                .multisample_state(&multisample_state)
                .color_blend_state(&blend_state)
                .render_pass(pass.render)
                .subpass(0)
                .layout(layout)
                .build()];
            let pipelines = unsafe {
                dev.device.borrow_mut().create_graphics_pipelines(
                    ash::vk::PipelineCache::null(),
                    &create_info,
                    None,
                )
            }
            .expect("Failed to create Vulkan graphics pipeline");
            unsafe {
                dev.device.borrow_mut().destroy_shader_module(rs_mod, None);
            }
            pipelines[0]
        };

        Self {
            graphics,
            set_layout,
            layout,
            device: Rc::clone(&dev.device),
        }
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            self.device
                .destroy_descriptor_set_layout(self.set_layout, None);
            self.device.destroy_pipeline_layout(self.layout, None);
            self.device.destroy_pipeline(self.graphics, None);
        }
    }
}

pub struct Buffer {
    allocation: vk_mem::Allocation,
    buffer: ash::vk::Buffer,
    usage: ash::vk::BufferUsageFlags,
    size: ash::vk::DeviceSize,
    allocator: Rc<RefCell<vk_mem::Allocator>>,
}

impl Buffer {
    fn create_buffer(
        allocator: &vk_mem::Allocator,
        size: ash::vk::DeviceSize,
        usage: ash::vk::BufferUsageFlags,
    ) -> (ash::vk::Buffer, vk_mem::Allocation) {
        let buffer_info = ash::vk::BufferCreateInfo::builder()
            .size(size)
            .usage(usage)
            .sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
            .build();

        // Vulkan memory
        let mut create_info = vk_mem::AllocationCreateInfo::default();
        create_info.usage = vk_mem::MemoryUsage::CpuToGpu;
        create_info.required_flags = ash::vk::MemoryPropertyFlags::HOST_VISIBLE;
        create_info.preferred_flags =
            ash::vk::MemoryPropertyFlags::HOST_COHERENT | ash::vk::MemoryPropertyFlags::HOST_CACHED;

        let (buffer, allocation, _) = allocator
            .create_buffer(&buffer_info, &create_info)
            .expect("Failed to create Vulkan buffer");

        (buffer, allocation)
    }

    pub fn new<T>(
        allocator: &Rc<RefCell<vk_mem::Allocator>>,
        usage: ash::vk::BufferUsageFlags,
    ) -> Self {
        let size = std::mem::size_of::<T>() as ash::vk::DeviceSize;

        let (buffer, allocation) = Self::create_buffer(&allocator.deref().borrow(), size, usage);

        Self {
            allocation,
            buffer,
            size,
            usage,
            allocator: allocator.clone(),
        }
    }

    pub fn upload<T>(&mut self, data: &T) {
        self.upload_raw(
            data as *const T,
            std::mem::size_of::<T>() as ash::vk::DeviceSize,
        );
    }

    pub fn upload_raw<T>(&mut self, src: *const T, size: ash::vk::DeviceSize) {
        let alloc = self.allocator.deref().borrow();
        let data = alloc
            .map_memory(&self.allocation)
            .expect("Failed to map Vulkan memory");
        unsafe { data.copy_from(src as _, size as usize) };
        alloc.unmap_memory(&self.allocation);
    }

    pub fn upload_arr<T>(&mut self, arr: &Vec<T>) {
        // Create a new buffer if not enough size for the vector
        let size = (arr.len() * std::mem::size_of::<T>()) as ash::vk::DeviceSize;
        if size as ash::vk::DeviceSize != self.size {
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
