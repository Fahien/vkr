// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::{extensions::ext::DebugUtils, vk, vk::Handle};
use sdl2 as sdl;
use std::{
    borrow::{Borrow, Cow},
    cell::RefCell,
    ffi::{CStr, CString},
    ops::Deref,
    rc::Rc,
};

use super::*;

unsafe extern "system" fn vk_debug(
    _message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message = if callback_data.p_message.is_null() {
        Cow::from("No message")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };
    eprintln!("{:?}", message);
    ash::vk::FALSE
}

pub struct Win {
    pub events: sdl::EventPump,
    pub window: sdl::video::Window,
    pub video: sdl::VideoSubsystem,
    pub ctx: sdl::Sdl,
}

impl Win {
    pub fn new(name: &str, width: u32, height: u32) -> Self {
        let ctx = sdl::init().expect("Failed to initialize SDL");
        let video = ctx.video().expect("Failed to initialize SDL video");
        let window = video
            .window(name, width, height)
            .allow_highdpi()
            .vulkan()
            .position_centered()
            .resizable()
            .build()
            .expect("Failed to build SDL window");

        let events = ctx.event_pump().expect("Failed to create SDL events");

        Self {
            events,
            window,
            video,
            ctx,
        }
    }
}

pub struct Debug {
    loader: DebugUtils,
    messenger: vk::DebugUtilsMessengerEXT,
}

impl Debug {
    fn new(ctx: &Ctx) -> Self {
        let loader = DebugUtils::new(&ctx.entry, &ctx.instance);
        let messenger = unsafe {
            loader
                .create_debug_utils_messenger(
                    &vk::DebugUtilsMessengerCreateInfoEXT::builder()
                        .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
                        .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
                        .pfn_user_callback(Some(vk_debug)),
                    None,
                )
                .expect("Failed to create Vulkan debug messenger")
        };

        Self { loader, messenger }
    }
}

impl Drop for Debug {
    fn drop(&mut self) {
        unsafe {
            self.loader
                .destroy_debug_utils_messenger(self.messenger, None);
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
        let mut extensions_names = vec![DebugUtils::name().as_ptr()];
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
    pub pipelines: DefaultPipelines,
    pub gui: Gui,
    pub sfs: SwapchainFrames, // Use box of frames?
    pub pass: Pass,           // How about multiple passes?
    pub dev: Dev,
    pub surface: Surface,
    pub debug: Debug,
    pub ctx: Ctx,
    pub win: Option<Win>,
    pub resized: bool, // Whether the window has been resized or not
    pub timer: Timer,
}

impl Vkr {
    pub fn new(win: Win) -> Self {
        let timer = Timer::new();

        let (width, height) = win.window.drawable_size();

        let ctx = Ctx::new(&win);
        let debug = Debug::new(&ctx);

        let surface = Surface::new(&win, &ctx);
        let mut dev = Dev::new(&ctx, &surface);

        let pass = Pass::new(&mut dev);
        let sfs = SwapchainFrames::new(&ctx, &surface, &mut dev, width, height, &pass);

        let gui = Gui::new(&win, &dev, &pass);

        let pipelines = DefaultPipelines::new(&dev, &pass, width, height);

        Self {
            pipelines,
            gui,
            sfs,
            pass,
            dev,
            surface,
            debug,
            ctx,
            win: Some(win),
            resized: false,
            timer,
        }
    }

    pub fn handle_events(&mut self) -> bool {
        let win = self.win.as_mut().unwrap();

        self.resized = false;

        // Handle events
        for event in win.events.poll_iter() {
            match event {
                sdl::event::Event::Window {
                    win_event: sdl::event::WindowEvent::Resized(_, _),
                    ..
                }
                | sdl::event::Event::Window {
                    win_event: sdl::event::WindowEvent::SizeChanged(_, _),
                    ..
                } => {
                    self.resized = true;
                }
                sdl::event::Event::Quit { .. }
                | sdl::event::Event::KeyDown {
                    keycode: Some(sdl::keyboard::Keycode::Escape),
                    ..
                } => return false,
                sdl::event::Event::MouseButtonDown { mouse_btn, .. } => {
                    if mouse_btn != sdl::mouse::MouseButton::Unknown {
                        let index = match mouse_btn {
                            sdl::mouse::MouseButton::Left => 0,
                            sdl::mouse::MouseButton::Right => 1,
                            sdl::mouse::MouseButton::Middle => 2,
                            sdl::mouse::MouseButton::X1 => 3,
                            sdl::mouse::MouseButton::X2 => 4,
                            sdl::mouse::MouseButton::Unknown => unreachable!(),
                        };
                        self.gui.mouse_down[index] = true;
                    }
                }
                sdl::event::Event::TextInput { ref text, .. } => {
                    for chr in text.chars() {
                        self.gui.ctx.io_mut().add_input_character(chr);
                    }
                }
                sdl::event::Event::KeyDown {
                    keycode: Some(code),
                    ..
                } => {
                    let index = code as usize;
                    let io = self.gui.ctx.io_mut();
                    if index < io.keys_down.len() {
                        io.keys_down[code as usize] = true;
                    }
                }
                sdl::event::Event::KeyUp {
                    keycode: Some(code),
                    ..
                } => {
                    let index = code as usize;
                    let keys = &mut self.gui.ctx.io_mut().keys_down;
                    if index < keys.len() {
                        keys[code as usize] = false;
                    }
                }
                _ => {}
            }
        }

        self.gui.set_mouse_state(&win.events.mouse_state());

        true
    }

    /// Returns a frame if available. When not available None is returned and drawing should be skipped
    /// TODO: Another option would be to wait until the frame is available and then return it.
    pub fn begin_frame(&mut self) -> Option<Frame> {
        let win = self.win.as_ref().unwrap();

        if self.resized {
            self.gui.set_drawable_size(win);
            self.sfs.recreate(win, &self.surface, &self.dev, &self.pass);
        }

        match self
            .sfs
            .next_frame(win, &self.surface, &self.dev, &self.pass)
        {
            Some(frame) => {
                let (width, height) = self.win.as_mut().unwrap().window.drawable_size();
                frame.begin(&self.pass, width, height);
                Some(frame)
            }
            None => None,
        }
    }

    /// Finish rendering a 3D scene and starts next (present) subpass
    pub fn end_scene(&mut self, frame: &mut Frame) {
        frame.res.command_buffer.next_subpass();

        let present_pipeline = self.pipelines.get_presentation();
        frame.res.command_buffer.bind_pipeline(present_pipeline);

        if frame.res.descriptors.present_sets.is_empty() {
            frame.res.descriptors.present_sets = frame
                .res
                .descriptors
                .allocate(&present_pipeline.set_layouts);
            PresentVertex::write_set(
                &self.dev.device,
                frame.res.descriptors.present_sets[0],
                &frame.buffer.albedo_view,
                &frame.buffer.normal_view,
                &frame.res.fallback.white_sampler,
            );
        }
        frame.res.command_buffer.bind_descriptor_sets(
            present_pipeline,
            &frame.res.descriptors.present_sets,
            0,
        );
        frame
            .res
            .command_buffer
            .bind_vertex_buffer(&frame.res.fallback.present_buffer);
        frame.res.command_buffer.draw(3);
    }

    pub fn end_frame(&mut self, frame: Frame) {
        frame.end();

        self.sfs.present(
            frame,
            &self.win.as_ref().unwrap(),
            &self.surface,
            &self.dev,
            &self.pass,
        );
    }

    /// This function can be called before binding the camera to update it.
    /// Internally it checks if a resize happened before doing anything.
    pub fn update_camera(&self, model: &mut Model, camera_node: util::Handle<Node>) {
        if self.resized {
            let camera_node = model.nodes.get(camera_node).unwrap();
            let camera = model.cameras.get_mut(camera_node.camera).unwrap();
            if let Some(win) = self.win.as_ref() {
                camera.update(win);
            }
        }
    }
}

impl Drop for Vkr {
    fn drop(&mut self) {
        // Make sure device is idle before releasing Vulkan resources
        self.dev.wait();
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

pub struct Dev {
    pub surface_format: ash::vk::SurfaceFormatKHR,
    pub graphics_command_pool: CommandPool,
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
            println!("\t{}", name);
        }
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

pub struct Pass {
    pub render: ash::vk::RenderPass,
    device: Rc<ash::Device>,
}

impl Pass {
    pub fn new(dev: &mut Dev) -> Self {
        // Render pass (swapchain surface format, device)
        let present_attachment = ash::vk::AttachmentDescription::builder()
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

        let albedo_attachment = ash::vk::AttachmentDescription::builder()
            // @todo This format should come from a "framebuffer" object
            .format(dev.surface_format.format)
            .samples(ash::vk::SampleCountFlags::TYPE_1)
            .load_op(ash::vk::AttachmentLoadOp::CLEAR)
            .store_op(ash::vk::AttachmentStoreOp::DONT_CARE)
            .stencil_load_op(ash::vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(ash::vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(ash::vk::ImageLayout::UNDEFINED)
            .final_layout(ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();

        let normal_attachment = vk::AttachmentDescription::builder()
            .format(vk::Format::A2R10G10B10_UNORM_PACK32)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();

        let attachments = [
            present_attachment,
            depth_attachment,
            albedo_attachment,
            normal_attachment,
        ];

        let present_ref = ash::vk::AttachmentReference::builder()
            .attachment(0)
            .layout(ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();

        let depth_ref = ash::vk::AttachmentReference::builder()
            .attachment(1)
            .layout(ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .build();

        let albedo_ref = ash::vk::AttachmentReference::builder()
            .attachment(2)
            .layout(ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();

        let normal_ref = vk::AttachmentReference::builder()
            .attachment(3)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();

        let first_color_refs = [albedo_ref, normal_ref];
        let second_color_refs = [present_ref];

        let albedo_input_ref = ash::vk::AttachmentReference::builder()
            .attachment(2)
            .layout(ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .build();

        let normal_input_ref = ash::vk::AttachmentReference::builder()
            .attachment(3)
            .layout(ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .build();

        let input_refs = [albedo_input_ref, normal_input_ref];

        // Two subpasses
        let subpasses = [
            // First subpass writes albedo and depth
            ash::vk::SubpassDescription::builder()
                .pipeline_bind_point(ash::vk::PipelineBindPoint::GRAPHICS)
                .color_attachments(&first_color_refs)
                .depth_stencil_attachment(&depth_ref)
                .build(),
            ash::vk::SubpassDescription::builder()
                .pipeline_bind_point(ash::vk::PipelineBindPoint::GRAPHICS)
                .color_attachments(&second_color_refs)
                .input_attachments(&input_refs)
                .build(),
        ];

        // These dependencies follow the example from
        // https://github.com/SaschaWillems/Vulkan/blob/master/examples/subpasses/subpasses.cpp
        let init_dependency = ash::vk::SubpassDependency::builder()
            .src_subpass(ash::vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(ash::vk::PipelineStageFlags::BOTTOM_OF_PIPE)
            .src_access_mask(ash::vk::AccessFlags::MEMORY_READ)
            .dst_stage_mask(ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(
                ash::vk::AccessFlags::COLOR_ATTACHMENT_READ
                    | ash::vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            )
            .dependency_flags(vk::DependencyFlags::BY_REGION)
            .build();

        let output_to_input_dependency = vk::SubpassDependency::builder()
            .src_subpass(0)
            .dst_subpass(1)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .dependency_flags(vk::DependencyFlags::BY_REGION)
            .build();

        let present_dependency = vk::SubpassDependency::builder()
            .src_subpass(1)
            .dst_subpass(vk::SUBPASS_EXTERNAL)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(
                vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            )
            .dst_stage_mask(vk::PipelineStageFlags::BOTTOM_OF_PIPE)
            .dst_access_mask(vk::AccessFlags::MEMORY_READ)
            .dependency_flags(vk::DependencyFlags::BY_REGION)
            .build();

        let dependencies = [
            init_dependency,
            output_to_input_dependency,
            present_dependency,
        ];

        // Build the render pass
        let create_info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies)
            .build();
        let render = unsafe { dev.device.create_render_pass(&create_info, None) }
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
            self.device.destroy_render_pass(self.render, None);
        }
    }
}

pub struct Buffer {
    allocation: vk_mem::Allocation,
    pub buffer: ash::vk::Buffer,
    usage: ash::vk::BufferUsageFlags,
    pub size: ash::vk::DeviceSize,
    pub allocator: Rc<RefCell<vk_mem::Allocator>>,
}

impl Buffer {
    pub fn create_buffer(
        allocator: &vk_mem::Allocator,
        size: ash::vk::DeviceSize,
        usage: ash::vk::BufferUsageFlags,
    ) -> (ash::vk::Buffer, vk_mem::Allocation) {
        assert!(size >= 32);

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

    /// Loads data from a png image in `path` directly into a staging buffer
    pub fn load(allocator: &Rc<RefCell<vk_mem::Allocator>>, png: &mut Png) -> Self {
        let size = png.info.buffer_size().max(32);
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
        usage: ash::vk::BufferUsageFlags,
        size: ash::vk::DeviceSize,
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

    pub fn new<T>(
        allocator: &Rc<RefCell<vk_mem::Allocator>>,
        usage: ash::vk::BufferUsageFlags,
    ) -> Self {
        let size = std::mem::size_of::<T>() as ash::vk::DeviceSize;
        let size = size.max(32);
        Self::new_with_size(allocator, usage, size)
    }

    pub fn new_arr<T>(
        allocator: &Rc<RefCell<vk_mem::Allocator>>,
        usage: ash::vk::BufferUsageFlags,
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
        usage: ash::vk::BufferUsageFlags,
    ) -> Self {
        let size = data.len() as ash::vk::DeviceSize;
        let size = size.max(32);
        let mut buffer = Self::new_with_size(allocator, usage, size);
        buffer.upload_arr(data);
        buffer
    }

    pub fn upload<T>(&mut self, data: &T) {
        self.upload_raw(
            data as *const T,
            std::mem::size_of::<T>() as ash::vk::DeviceSize,
        );
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
        let size = size.max(32);
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
