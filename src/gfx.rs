// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::{
    extensions::ext::DebugReport,
    version::{DeviceV1_0, EntryV1_0, InstanceV1_0},
    vk::Handle,
};
use sdl2 as sdl;
use std::{
    borrow::Borrow,
    cell::RefCell,
    ffi::{c_void, CStr, CString},
    ops::Deref,
    os::raw::c_char,
    rc::Rc,
};

use super::*;

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

        Self {
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
                frame.begin(&self.pass);
                Some(frame)
            }
            None => None,
        }
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
        enabled_extensions.push(ash::extensions::khr::Swapchain::name().as_ptr());

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

pub struct Pipeline {
    pub graphics: ash::vk::Pipeline,
    pub layout: ash::vk::PipelineLayout,
    pub set_layouts: Vec<ash::vk::DescriptorSetLayout>,
    device: Rc<ash::Device>,
}

impl Pipeline {
    pub fn new<T: VertexInput>(
        dev: &Dev,
        vert: ash::vk::PipelineShaderStageCreateInfo,
        frag: ash::vk::PipelineShaderStageCreateInfo,
        topology: ash::vk::PrimitiveTopology,
        pass: &Pass,
        width: u32,
        height: u32,
    ) -> Self {
        let set_layouts = T::get_set_layouts(&dev.device);
        let constants = T::get_constants();

        // Pipeline layout (device, descriptorset layouts, shader reflection?)
        let layout = {
            let create_info = ash::vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&set_layouts)
                .push_constant_ranges(&constants)
                .build();
            unsafe { dev.device.create_pipeline_layout(&create_info, None) }
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
                .min_depth(1.0)
                .max_depth(0.0)
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

            let depth_state = T::get_depth_state();

            let blend_attachment = T::get_color_blend();

            let blend_state = ash::vk::PipelineColorBlendStateCreateInfo::builder()
                .logic_op_enable(false)
                .attachments(&blend_attachment)
                .build();

            let states = vec![
                ash::vk::DynamicState::VIEWPORT,
                ash::vk::DynamicState::SCISSOR,
            ];
            let dynamic_state = ash::vk::PipelineDynamicStateCreateInfo::builder()
                .dynamic_states(&states)
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
                .dynamic_state(&dynamic_state)
                .render_pass(pass.render)
                .subpass(0)
                .layout(layout)
                .build()];

            let pipelines = unsafe {
                dev.device.create_graphics_pipelines(
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
            set_layouts,
            layout,
            device: Rc::clone(&dev.device),
        }
    }

    pub fn line(dev: &Dev, pass: &Pass, width: u32, height: u32) -> Self {
        let shader = ShaderModule::main(&dev.device);
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

    pub fn main(dev: &Dev, pass: &Pass, width: u32, height: u32) -> Self {
        let shader = ShaderModule::main(&dev.device);
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

    /// Returns a graphics pipeline which draws the normals of primitive's surfaces as a color
    pub fn normal(dev: &Dev, pass: &Pass, width: u32, height: u32) -> Self {
        let shader = ShaderModule::main(&dev.device);
        let vs = CString::new("main_vs").expect("Failed to create entrypoint");
        let fs = CString::new("normal_fs").expect("Failed to create entrypoint");
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
            for set_layout in &self.set_layouts {
                self.device.destroy_descriptor_set_layout(*set_layout, None);
            }
            self.device.destroy_pipeline_layout(self.layout, None);
            self.device.destroy_pipeline(self.graphics, None);
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
        usage: ash::vk::BufferUsageFlags,
        size: ash::vk::DeviceSize,
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
        Self::new_with_size(allocator, usage, size)
    }

    pub fn from_data(
        allocator: &Rc<RefCell<vk_mem::Allocator>>,
        data: &[u8],
        usage: ash::vk::BufferUsageFlags,
    ) -> Self {
        let mut buffer = Self::new_with_size(allocator, usage, data.len() as ash::vk::DeviceSize);
        buffer.upload_arr(data);
        buffer
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
}

impl Drop for Buffer {
    fn drop(&mut self) {
        self.allocator
            .deref()
            .borrow()
            .destroy_buffer(self.buffer, &self.allocation);
    }
}
