// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::{
    extensions::ext::DebugReport,
    version::{DeviceV1_0, DeviceV1_2, EntryV1_0, InstanceV1_0},
    vk::Handle,
};
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

use super::*;

pub struct Primitive {
    vertex_count: u32,
    vertices: Buffer,
    indices: Option<Buffer>,
}

impl Primitive {
    pub fn new<T>(allocator: &Rc<RefCell<vk_mem::Allocator>>, vv: &[T]) -> Self {
        let vertex_count = vv.len() as u32;

        let mut vertices = Buffer::new::<T>(allocator, ash::vk::BufferUsageFlags::VERTEX_BUFFER);
        vertices.upload_arr(vv);

        Self {
            vertex_count,
            vertices,
            indices: None,
        }
    }

    /// Returns a new primitive quad with side length 1 centered at the origin
    pub fn quad(allocator: &Rc<RefCell<vk_mem::Allocator>>) -> Self {
        let vertices = vec![
            Vertex {
                pos: na::Vector3::new(-0.5, -0.5, 0.0),
                color: Color::white(),
                uv: na::Vector2::new(0.0, 1.0),
            },
            Vertex {
                pos: na::Vector3::new(0.5, -0.5, 0.0),
                color: Color::white(),
                uv: na::Vector2::new(1.0, 1.0),
            },
            Vertex {
                pos: na::Vector3::new(0.5, 0.5, 0.0),
                color: Color::white(),
                uv: na::Vector2::new(1.0, 0.0),
            },
            Vertex {
                pos: na::Vector3::new(-0.5, 0.5, 0.0),
                color: Color::white(),
                uv: na::Vector2::new(0.0, 0.0),
            },
        ];
        let indices = vec![0, 1, 2, 2, 3, 0];

        let mut ret = Self::new(allocator, &vertices);
        ret.set_indices(&indices);
        ret
    }

    pub fn set_indices(&mut self, ii: &[u16]) {
        let mut indices = Buffer::new::<u16>(
            &self.vertices.allocator,
            ash::vk::BufferUsageFlags::INDEX_BUFFER,
        );
        indices.upload_arr(ii);
        self.indices = Some(indices);
    }
}

/// Per-frame resource which contains a descriptor pool and a vector
/// of descriptor sets of each pipeline layout used for rendering.
struct Descriptors {
    /// These descriptor sets are for model matrix uniforms, therefore we need NxM descriptor sets
    /// where N is the number of pipeline layouts, and M is the node with the model matrix
    sets: HashMap<(ash::vk::PipelineLayout, util::Handle<Node>), Vec<ash::vk::DescriptorSet>>,
    /// Descriptor pools should be per-pipeline layout as weel as they could differ in terms of uniforms and samplers?
    /// Or can we provide sufficient descriptors for all supported pipeline layouts? Trying this approach.
    pool: ash::vk::DescriptorPool,
    device: Rc<ash::Device>,
}

impl Descriptors {
    pub fn new(dev: &mut Dev) -> Self {
        let pool = unsafe {
            // 2 uniform buffers, one for the line pipeline andd one for the main pipeline
            let uniform_pool_size = ash::vk::DescriptorPoolSize::builder()
                .descriptor_count(2)
                .ty(ash::vk::DescriptorType::UNIFORM_BUFFER)
                .build();
            // 1 sampler for the main pipeline
            let sampler_pool_size = ash::vk::DescriptorPoolSize::builder()
                .descriptor_count(1)
                .ty(ash::vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .build();

            let pool_sizes = vec![uniform_pool_size, sampler_pool_size];
            let create_info = ash::vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(&pool_sizes)
                // Support 2 frames?
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
    // @todo Make a map of framebuffers indexed by render-pass as key
    pub framebuffer: ash::vk::Framebuffer,
    pub depth_view: ImageView,
    pub depth_image: Image,
    pub image_view: ash::vk::ImageView,
    device: Rc<ash::Device>,
}

impl Framebuffer {
    pub fn new(dev: &mut Dev, image: &Image, pass: &Pass) -> Self {
        // Image view into a swapchain images (device, image, format)
        let image_view = {
            let create_info = ash::vk::ImageViewCreateInfo::builder()
                .image(image.image)
                .view_type(ash::vk::ImageViewType::TYPE_2D)
                .format(image.format)
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

        let depth_format = ash::vk::Format::D32_SFLOAT;
        let depth_usage = ash::vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
        let mut depth_image = Image::new(
            &dev.allocator,
            image.extent.width,
            image.extent.height,
            depth_format,
            depth_usage,
        );
        depth_image.transition(&dev, ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

        let depth_view = ImageView::new(&dev.device, &depth_image);

        // Framebuffers (image_view, renderpass)
        let framebuffer = {
            let attachments = [image_view, depth_view.view];

            let create_info = ash::vk::FramebufferCreateInfo::builder()
                .render_pass(pass.render)
                .attachments(&attachments)
                .width(image.extent.width)
                .height(image.extent.height)
                .layers(1)
                .build();

            unsafe {
                dev.device
                    .borrow_mut()
                    .create_framebuffer(&create_info, None)
            }
            .expect("Failed to create Vulkan framebuffer")
        };

        Self {
            framebuffer,
            depth_view,
            depth_image,
            image_view,
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

pub struct Semaphore {
    pub semaphore: ash::vk::Semaphore,
    device: Rc<ash::Device>,
}

impl Semaphore {
    pub fn new(device: &Rc<ash::Device>) -> Self {
        let create_info = ash::vk::SemaphoreCreateInfo::builder().build();
        let semaphore = unsafe { device.create_semaphore(&create_info, None) }
            .expect("Failed to create Vulkan semaphore");

        Self {
            semaphore,
            device: device.clone(),
        }
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe { self.device.destroy_semaphore(self.semaphore, None) };
    }
}

/// Frame resources that do not need to be recreated
/// when the swapchain goes out of date
pub struct Frameres {
    /// Uniform buffers for model matrix are associated to nodes
    ubos: HashMap<util::Handle<Node>, Buffer>,
    descriptors: Descriptors,
    pub command_buffer: ash::vk::CommandBuffer,
    pub fence: Fence,
    pub image_ready: Semaphore,
    pub image_drawn: Semaphore,
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
        let fence = Fence::signaled(&dev.device);

        Self {
            ubos: HashMap::new(),
            descriptors: Descriptors::new(dev),
            command_buffer,
            fence,
            image_ready: Semaphore::new(&dev.device),
            image_drawn: Semaphore::new(&dev.device),
        }
    }

    pub fn wait(&mut self) {
        if self.fence.can_wait {
            self.fence.wait();
            self.fence.reset();
        }
    }
}

pub struct Frame {
    pub buffer: Framebuffer,
    pub res: Frameres,
    /// A frame should be able to allocate a uniform buffer on draw
    allocator: Rc<RefCell<vk_mem::Allocator>>,
    pub device: Rc<ash::Device>,
}

impl Frame {
    pub fn new(dev: &mut Dev, image: &Image, pass: &Pass) -> Self {
        let buffer = Framebuffer::new(dev, image, pass);
        let res = Frameres::new(dev);

        Frame {
            buffer,
            res,
            allocator: dev.allocator.clone(),
            device: Rc::clone(&dev.device),
        }
    }

    pub fn begin(&self, pass: &Pass, width: u32, height: u32) {
        let begin_info = ash::vk::CommandBufferBeginInfo::builder().build();
        unsafe {
            self.device
                .begin_command_buffer(self.res.command_buffer, &begin_info)
        }
        .expect("Failed to begin Vulkan command buffer");

        // Needed by cmd_begin_render_pass
        let area = ash::vk::Rect2D::builder()
            .offset(ash::vk::Offset2D::builder().x(0).y(0).build())
            .extent(
                ash::vk::Extent2D::builder()
                    .width(width)
                    .height(height)
                    .build(),
            )
            .build();

        let mut color_clear = ash::vk::ClearValue::default();
        color_clear.color.float32 = [0.0, 10.0 / 255.0, 28.0 / 255.0, 1.0];

        let mut depth_clear = ash::vk::ClearValue::default();
        depth_clear.depth_stencil.depth = 1.0;
        depth_clear.depth_stencil.stencil = 0;

        let clear_values = [color_clear, depth_clear];
        let create_info = ash::vk::RenderPassBeginInfo::builder()
            .framebuffer(self.buffer.framebuffer)
            .render_pass(pass.render)
            .render_area(area)
            .clear_values(&clear_values)
            .build();
        // Record it in the main command buffer
        let contents = ash::vk::SubpassContents::INLINE;
        unsafe {
            self.device
                .cmd_begin_render_pass(self.res.command_buffer, &create_info, contents)
        };

        let viewports = [ash::vk::Viewport::builder()
            .width(width as f32)
            .height(height as f32)
            .build()];
        unsafe {
            self.device
                .cmd_set_viewport(self.res.command_buffer, 0, &viewports)
        };

        let scissors = [ash::vk::Rect2D::builder()
            .extent(
                ash::vk::Extent2D::builder()
                    .width(width)
                    .height(height)
                    .build(),
            )
            .build()];
        unsafe {
            self.device
                .cmd_set_scissor(self.res.command_buffer, 0, &scissors)
        }
    }

    pub fn draw<T: VertexInput>(
        &mut self,
        pipeline: &Pipeline,
        model: &Model,
        primitive: &Primitive,
        node: util::Handle<Node>,
        texture: util::Handle<Texture>,
    ) {
        let graphics_bind_point = ash::vk::PipelineBindPoint::GRAPHICS;
        unsafe {
            self.device.cmd_bind_pipeline(
                self.res.command_buffer,
                graphics_bind_point,
                pipeline.graphics,
            );
        }

        if let Some(sets) = self.res.descriptors.sets.get(&(pipeline.layout, node)) {
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

            // If there is a descriptor set, there must be a uniform buffer
            let ubo = self.res.ubos.get_mut(&node).unwrap();
            ubo.upload(&model.nodes.get(node).unwrap().trs.get_matrix());
        } else {
            // Create a new uniform buffer for this node's model matrix
            let mut ubo =
                Buffer::new::<Ubo>(&self.allocator, ash::vk::BufferUsageFlags::UNIFORM_BUFFER);
            ubo.upload(&model.nodes.get(node).unwrap().trs.get_matrix());

            let sets = self.res.descriptors.allocate(&[pipeline.set_layout]);

            let texture = model.textures.get(texture);
            let (view, sampler) = match texture {
                Some(texture) => (
                    model.views.get(texture.view),
                    model.samplers.get(texture.sampler),
                ),
                _ => (None, None),
            };
            T::write_set(self.device.borrow(), sets[0], &ubo, view, sampler);

            unsafe {
                self.device.cmd_bind_descriptor_sets(
                    self.res.command_buffer,
                    graphics_bind_point,
                    pipeline.layout,
                    0,
                    &sets,
                    &[],
                );
            }

            self.res.ubos.insert(node, ubo);
            self.res
                .descriptors
                .sets
                .insert((pipeline.layout, node), sets);
        }

        let first_binding = 0;
        let buffers = [primitive.vertices.buffer];
        let offsets = [ash::vk::DeviceSize::default()];
        unsafe {
            self.device.cmd_bind_vertex_buffers(
                self.res.command_buffer,
                first_binding,
                &buffers,
                &offsets,
            );
        }

        if let Some(indices) = &primitive.indices {
            // Draw indexed if primitive has indices
            unsafe {
                self.device.cmd_bind_index_buffer(
                    self.res.command_buffer,
                    indices.buffer,
                    0,
                    ash::vk::IndexType::UINT16,
                );
            }
            let index_count = indices.size as u32 / std::mem::size_of::<u16>() as u32;
            unsafe {
                self.device
                    .cmd_draw_indexed(self.res.command_buffer, index_count, 1, 0, 0, 0);
            }
        } else {
            // Draw without indices
            unsafe {
                self.device
                    .cmd_draw(self.res.command_buffer, primitive.vertex_count, 1, 0, 0);
            }
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

    pub fn submit(&mut self, dev: &Dev) {
        dev.graphics_queue.submit_draw(
            &self.res.command_buffer,
            None,
            &self.res.image_drawn,
            Some(&mut self.res.fence),
        );
    }

    pub fn present(
        &mut self,
        dev: &Dev,
        swapchain: &Swapchain,
        image_index: u32,
    ) -> Result<(), ash::vk::Result> {
        dev.graphics_queue.submit_draw(
            &self.res.command_buffer,
            Some(&self.res.image_ready),
            &self.res.image_drawn,
            Some(&mut self.res.fence),
        );

        dev.graphics_queue
            .present(image_index, swapchain, &self.res.image_drawn)
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
            .window("Test", 480, 480)
            .allow_highdpi()
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
    pub fn headless() -> Self {
        // @todo Refactor common code
        let extensions_names = vec![DebugReport::name().as_ptr()];
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
    pub fn headless() -> Self {
        let ctx = Ctx::headless();
        let debug = Debug::new(&ctx);

        Self { ctx, debug }
    }

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
    pub graphics_queue: Queue,
    /// Needs to be public if we want to create buffers outside this module.
    /// The allocator is shared between the various buffers to release resources on drop.
    /// Moreover it needs to be inside a RefCell, so we can mutably borrow it on destroy.
    pub allocator: Rc<RefCell<vk_mem::Allocator>>,
    pub device: Rc<ash::Device>,
    physical: ash::vk::PhysicalDevice,
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
            // @todo Extract method
            let supports_presentation = if let Some(surface) = surface {
                unsafe {
                    surface.ext.get_physical_device_surface_support(
                        physical,
                        i as u32,
                        surface.surface,
                    )
                }
                .expect("Failed to check presentation support for Vulkan physical device")
            } else {
                true
            };
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

        // Enable some extensions
        let mut enabled_extensions: Vec<*const i8> = vec![];

        let extension_properties =
            unsafe { ctx.instance.enumerate_device_extension_properties(physical) }
                .expect("Failed to enumerate Vulkan device extension properties");

        for prop in extension_properties.iter() {
            let name = unsafe { CStr::from_ptr(prop.extension_name.as_ptr()) }
                .to_str()
                .unwrap();
            println!("\t{}", name);
        }
        if surface.is_some() {
            enabled_extensions.push(ash::extensions::khr::Swapchain::name().as_ptr());
        }

        let device_create_info = ash::vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_infos)
            .enabled_extension_names(&enabled_extensions);

        let device = unsafe {
            ctx.instance
                .create_device(physical, &device_create_info, None)
                .expect("Failed to create Vulkan logical device")
        };
        let device = Rc::new(device);

        let graphics_queue = Queue::new(&device, graphics_queue_index);

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
        let surface_format = if let Some(surface) = surface {
            let surface_formats = unsafe {
                surface
                    .ext
                    .get_physical_device_surface_formats(physical, surface.surface)
            }
            .expect("Failed to get Vulkan physical device surface formats");

            surface_formats[1]
        } else {
            ash::vk::SurfaceFormatKHR::builder()
                .color_space(ash::vk::ColorSpaceKHR::SRGB_NONLINEAR)
                .format(ash::vk::Format::R8G8B8A8_SRGB)
                .build()
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
        let color_attachment = ash::vk::AttachmentDescription::builder()
            // @todo This format should come from a "framebuffer" object
            .format(dev.surface_format.format)
            .samples(ash::vk::SampleCountFlags::TYPE_1)
            .load_op(ash::vk::AttachmentLoadOp::CLEAR)
            .store_op(ash::vk::AttachmentStoreOp::STORE)
            .stencil_load_op(ash::vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(ash::vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(ash::vk::ImageLayout::UNDEFINED)
            .final_layout(ash::vk::ImageLayout::PRESENT_SRC_KHR)
            .build();

        let depth_attachment = ash::vk::AttachmentDescription::builder()
            // @todo This format should come from a "framebuffer" object
            .format(ash::vk::Format::D32_SFLOAT)
            .samples(ash::vk::SampleCountFlags::TYPE_1)
            .load_op(ash::vk::AttachmentLoadOp::CLEAR)
            .store_op(ash::vk::AttachmentStoreOp::DONT_CARE)
            .stencil_load_op(ash::vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(ash::vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(ash::vk::ImageLayout::UNDEFINED)
            .final_layout(ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .build();

        let attachments = [color_attachment, depth_attachment];

        let color_ref = ash::vk::AttachmentReference::builder()
            .attachment(0)
            .layout(ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();

        let depth_ref = ash::vk::AttachmentReference::builder()
            .attachment(1)
            .layout(ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .build();

        let color_refs = [color_ref];

        // Just one subpass
        let subpasses = [ash::vk::SubpassDescription::builder()
            .pipeline_bind_point(ash::vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_refs)
            .depth_stencil_attachment(&depth_ref)
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
            .attachments(&attachments)
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
        vert: ash::vk::PipelineShaderStageCreateInfo,
        frag: ash::vk::PipelineShaderStageCreateInfo,
        topology: ash::vk::PrimitiveTopology,
        pass: &Pass,
        width: u32,
        height: u32,
    ) -> Self {
        let layout_bindings = T::get_set_layout_bindings();
        let set_layout_info = ash::vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&layout_bindings)
            .build();

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

            let depth_state = ash::vk::PipelineDepthStencilStateCreateInfo::builder()
                .depth_test_enable(true)
                .depth_write_enable(true)
                .depth_compare_op(ash::vk::CompareOp::LESS)
                .depth_bounds_test_enable(false)
                .stencil_test_enable(false)
                .build();

            let blend_attachment = [ash::vk::PipelineColorBlendAttachmentState::builder()
                .blend_enable(false)
                .color_write_mask(ash::vk::ColorComponentFlags::all())
                .build()];

            let blend_state = ash::vk::PipelineColorBlendStateCreateInfo::builder()
                .logic_op_enable(false)
                .attachments(&blend_attachment)
                .build();

            let stages = [vert, frag];

            let create_info = [ash::vk::GraphicsPipelineCreateInfo::builder()
                .stages(&stages)
                .vertex_input_state(&vertex_input)
                .input_assembly_state(&input_assembly)
                .viewport_state(&view_state)
                .rasterization_state(&raster_state)
                .multisample_state(&multisample_state)
                .depth_stencil_state(&depth_state)
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

            pipelines[0]
        };

        Self {
            graphics,
            set_layout,
            layout,
            device: Rc::clone(&dev.device),
        }
    }

    pub fn line(dev: &mut Dev, pass: &Pass, width: u32, height: u32) -> Self {
        let shader = ShaderModule::new(&dev.device);
        let vs = CString::new("line_vs").expect("Failed to create entrypoint");
        let fs = CString::new("line_fs").expect("Failed to create entrypoint");
        Self::new::<Line>(
            dev,
            shader.get_vert(&vs),
            shader.get_frag(&fs),
            ash::vk::PrimitiveTopology::LINE_STRIP,
            pass,
            width,
            height,
        )
    }

    pub fn main(dev: &mut Dev, pass: &Pass, width: u32, height: u32) -> Self {
        let shader = ShaderModule::new(&dev.device);
        let vs = CString::new("main_vs").expect("Failed to create entrypoint");
        let fs = CString::new("main_fs").expect("Failed to create entrypoint");
        Self::new::<Vertex>(
            dev,
            shader.get_vert(&vs),
            shader.get_frag(&fs),
            ash::vk::PrimitiveTopology::TRIANGLE_LIST,
            pass,
            width,
            height,
        )
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
    pub buffer: ash::vk::Buffer,
    usage: ash::vk::BufferUsageFlags,
    size: ash::vk::DeviceSize,
    pub allocator: Rc<RefCell<vk_mem::Allocator>>,
}

impl Buffer {
    pub fn create_buffer(
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

    pub fn map<'a>(&'a mut self) -> &'a [u8] {
        let alloc = self.allocator.deref().borrow();
        unsafe {
            let data = alloc
                .map_memory(&self.allocation)
                .expect("Failed to map Vulkan memory");
            std::slice::from_raw_parts_mut(data, self.size as usize)
        }
    }

    pub fn unmap(&self) {
        let alloc = self.allocator.deref().borrow();
        alloc.unmap_memory(&self.allocation);
    }

    /// Loads data from a png image in `path` directly into a staging buffer
    pub fn load(allocator: &Rc<RefCell<vk_mem::Allocator>>, png: &mut Png) -> Self {
        let size = png.info.buffer_size();
        let usage = ash::vk::BufferUsageFlags::TRANSFER_SRC;

        // Create staging buffer
        let (buffer, allocation) = Self::create_buffer(
            &allocator.deref().borrow(),
            size as ash::vk::DeviceSize,
            usage,
        );

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
            size: size as ash::vk::DeviceSize,
            allocator: allocator.clone(),
        }
    }

    pub fn new_with_size(
        allocator: &Rc<RefCell<vk_mem::Allocator>>,
        size: ash::vk::DeviceSize,
        usage: ash::vk::BufferUsageFlags,
    ) -> Self {
        let (buffer, allocation) = Self::create_buffer(&allocator.deref().borrow(), size, usage);

        Self {
            allocation,
            buffer,
            size,
            usage,
            allocator: allocator.clone(),
        }
    }

    pub fn new<T>(
        allocator: &Rc<RefCell<vk_mem::Allocator>>,
        usage: ash::vk::BufferUsageFlags,
    ) -> Self {
        let size = std::mem::size_of::<T>() as ash::vk::DeviceSize;
        Self::new_with_size(allocator, size, usage)
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

    pub fn upload_arr<T>(&mut self, arr: &[T]) {
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

    pub fn copy_from(dev: &Dev, image: &mut Image) -> Self {
        let size = (image.extent.width * image.extent.height * 4) as ash::vk::DeviceSize; // bytes
        let usage = ash::vk::BufferUsageFlags::TRANSFER_DST;
        let mut buffer = Self::new_with_size(&dev.allocator, size, usage);

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

        image.transition_barrier(
            ash::vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            dev,
            command_buffer,
        );

        // Copy
        unsafe {
            let regions = vec![ash::vk::BufferImageCopy::builder()
                .image_subresource(
                    ash::vk::ImageSubresourceLayers::builder()
                        .aspect_mask(ash::vk::ImageAspectFlags::COLOR)
                        .layer_count(1)
                        .build(),
                )
                .image_extent(image.extent)
                .build()];
            dev.device.cmd_copy_image_to_buffer(
                command_buffer,
                image.image,
                image.layout,
                buffer.buffer,
                &regions,
            );
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

        buffer
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
