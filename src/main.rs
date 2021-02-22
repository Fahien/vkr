// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{
    ffi::{c_void, CStr, CString},
    os::raw::c_char,
};

use byteorder::{ByteOrder, NativeEndian};

use sdl::{event::Event, keyboard::Keycode};
use sdl2 as sdl;

use ash::{
    extensions::ext::DebugReport,
    version::{DeviceV1_0, EntryV1_0, InstanceV1_0},
    vk::Handle,
};

#[repr(C)]
struct Vec3f {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3f {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3f { x, y, z }
    }
}

#[repr(C)]
struct Vertex {
    pos: Vec3f,
}

struct Frame {
    image_view: ash::vk::ImageView,
    // @todo Make a map of framebuffers indexed by render-pass as key
    framebuffer: ash::vk::Framebuffer,
    command_buffer: ash::vk::CommandBuffer,
    fence: ash::vk::Fence,
    image_ready: ash::vk::Semaphore,
    image_drawn: ash::vk::Semaphore,
}

impl Frame {
    fn new<T: ash::version::DeviceV1_0>(
        device: &T,
        image: &ash::vk::Image,
        surface_format: ash::vk::Format,
        render_pass: ash::vk::RenderPass,
        width: u32,
        height: u32,
        command_pool: ash::vk::CommandPool,
    ) -> Self {
        // Image view into a swapchain images (device, image, format)
        let image_view = {
            let create_info = ash::vk::ImageViewCreateInfo::builder()
                .image(*image)
                .view_type(ash::vk::ImageViewType::TYPE_2D)
                .format(surface_format)
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
            unsafe { device.create_image_view(&create_info, None) }
                .expect("Failed to create Vulkan image view")
        };

        // Framebuffers (image_view, renderpass)
        let framebuffer = {
            let attachments = [image_view];

            let create_info = ash::vk::FramebufferCreateInfo::builder()
                .render_pass(render_pass)
                .attachments(&attachments)
                .width(width)
                .height(height)
                .layers(1)
                .build();

            unsafe { device.create_framebuffer(&create_info, None) }
                .expect("Failed to create Vulkan framebuffer")
        };

        // Graphics command buffer (device, command pool)
        let command_buffer = {
            let alloc_info = ash::vk::CommandBufferAllocateInfo::builder()
                .command_pool(command_pool)
                .level(ash::vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);

            let buffers = unsafe { device.allocate_command_buffers(&alloc_info) }
                .expect("Failed to allocate command buffer");
            buffers[0]
        };

        // Fence (device)
        let fence = {
            let create_info = ash::vk::FenceCreateInfo::builder()
                .flags(ash::vk::FenceCreateFlags::SIGNALED)
                .build();
            unsafe { device.create_fence(&create_info, None) }
        }
        .expect("Failed to create Vulkan fence");

        // Semaphores (device)
        let (image_ready, image_drawn) = {
            let create_info = ash::vk::SemaphoreCreateInfo::builder().build();
            unsafe {
                (
                    device
                        .create_semaphore(&create_info, None)
                        .expect("Failed to create Vulkan semaphore"),
                    device
                        .create_semaphore(&create_info, None)
                        .expect("Failed to create Vulkan semaphore"),
                )
            }
        };

        Frame {
            image_view,
            framebuffer,
            command_buffer,
            fence,
            image_ready,
            image_drawn,
        }
    }
}

unsafe extern "system" fn vk_debug(
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

fn main() {
    // Window
    let ctx = sdl::init().expect("Failed to initialize SDL");
    let video = ctx.video().expect("Failed to initialize SDL video");
    let window = video
        .window("Test", 480, 320)
        .vulkan()
        .position_centered()
        .build()
        .expect("Failed to build SDL window");

    // Instance (window)
    let extensions = window
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

    // Debugging callback
    let debug_info = ash::vk::DebugReportCallbackCreateInfoEXT::builder()
        .flags(
            ash::vk::DebugReportFlagsEXT::ERROR
                | ash::vk::DebugReportFlagsEXT::WARNING
                | ash::vk::DebugReportFlagsEXT::PERFORMANCE_WARNING,
        )
        .pfn_callback(Some(vk_debug));

    let debug_report_loader = DebugReport::new(&entry, &instance);
    let _ = unsafe {
        debug_report_loader
            .create_debug_report_callback(&debug_info, None)
            .expect("Failed to create Vulkan debug callback")
    };

    // Surface (window, instance)
    let surface = window
        .vulkan_create_surface(instance.handle().as_raw() as usize)
        .expect("Failed to create surface");
    let surface: ash::vk::SurfaceKHR = ash::vk::Handle::from_raw(surface);
    let surface_ext = ash::extensions::khr::Surface::new(&entry, &instance);

    // Physical device
    let physical_devs = unsafe {
        instance
            .enumerate_physical_devices()
            .expect("Failed to enumerate Vulkan physical devices")
    };
    println!("Devices:");
    for pdev in physical_devs.iter() {
        let properties = unsafe { instance.get_physical_device_properties(*pdev) };
        let name = unsafe { CStr::from_ptr(properties.device_name.as_ptr()) };
        println!(" - {:?}", name);
    }
    let physical_dev = physical_devs[0];

    // Queue information (instance, physical device)
    let queue_properties =
        unsafe { instance.get_physical_device_queue_family_properties(physical_dev) };

    let mut graphics_queue_index = std::u32::MAX;

    for (i, queue) in queue_properties.iter().enumerate() {
        let supports_presentation = unsafe {
            surface_ext.get_physical_device_surface_support(physical_dev, i as u32, surface)
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
        instance
            .create_device(physical_dev, &device_create_info, None)
            .expect("Failed to create Vulkan logical device")
    };

    // Graphics queue (logical device)
    let graphics_queue = unsafe { device.get_device_queue(graphics_queue_index, 0) };

    // Command pool
    let create_info = ash::vk::CommandPoolCreateInfo::builder()
        .flags(ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
        .queue_family_index(graphics_queue_index);
    let command_pool = {
        unsafe {
            device
                .create_command_pool(&create_info, None)
                .expect("Failed to create Vulkan command pool")
        }
    };

    // Surface format
    let surface_format = {
        let surface_formats =
            unsafe { surface_ext.get_physical_device_surface_formats(physical_dev, surface) }
                .expect("Failed to get Vulkan physical device surface formats");
        println!("Surface formats:");
        for format in surface_formats.iter() {
            println!("- {:?}", format.format);
        }

        surface_formats[1]
    };

    // Swapchain (instance, logical device, surface formats)
    let swapchain_ext = ash::extensions::khr::Swapchain::new(&instance, &device);

    // This needs to be queried to prevent validation layers complaining
    let surface_capabilities =
        unsafe { surface_ext.get_physical_device_surface_capabilities(physical_dev, surface) }
            .expect("Failed to get Vulkan physical device surface capabilities");
    println!(
        "Surface transform: {:?}",
        surface_capabilities.current_transform
    );

    let swapchain = {
        let create_info = ash::vk::SwapchainCreateInfoKHR::builder()
            .surface(surface)
            .min_image_count(2)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(
                ash::vk::Extent2D::builder()
                    .width(window.size().0)
                    .height(window.size().1)
                    .build(),
            )
            .image_array_layers(1)
            .image_usage(ash::vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(ash::vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(ash::vk::PresentModeKHR::FIFO)
            .clipped(true);
        unsafe { swapchain_ext.create_swapchain(&create_info, None) }
            .expect("Failed to create Vulkan swapchain")
    };

    let swapchain_images = unsafe { swapchain_ext.get_swapchain_images(swapchain) }
        .expect("Failed to get Vulkan swapchain images");

    // Render pass (swapchain surface format, device)
    let render_pass = {
        let attachment = [ash::vk::AttachmentDescription::builder()
            .format(surface_format.format)
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
        unsafe { device.create_render_pass(&create_info, None) }
            .expect("Failed to create Vulkan render pass")
    };

    // Frames: collection of per-frame resources (device, swapchain, renderpass, command pool)
    let mut frames = Vec::new();
    for image in swapchain_images.iter() {
        frames.push(Frame::new(
            &device,
            &image,
            surface_format.format,
            render_pass,
            window.size().0,
            window.size().1,
            command_pool,
        ));
    }

    // Pipeline layout (device, shader reflection?)
    let pipeline_layout = {
        let create_info = ash::vk::PipelineLayoutCreateInfo::builder().build();
        unsafe { device.create_pipeline_layout(&create_info, None) }
            .expect("Failed to create Vulkan pipeline layout")
    };

    // Graphics pipeline (shaders, renderpass)
    let graphics_pipeline = {
        let frag_spv = include_bytes!("../res/shader/frag.spv");
        let vert_spv = include_bytes!("../res/shader/vert.spv");

        let mut frag_code = vec![0; frag_spv.len() / std::mem::size_of::<u32>()];
        let mut vert_code = vec![0; vert_spv.len() / std::mem::size_of::<u32>()];

        NativeEndian::read_u32_into(vert_spv, vert_code.as_mut_slice());
        NativeEndian::read_u32_into(frag_spv, frag_code.as_mut_slice());

        let create_info = ash::vk::ShaderModuleCreateInfo::builder()
            .code(frag_code.as_slice())
            .build();
        let frag_mod = unsafe { device.create_shader_module(&create_info, None) }
            .expect("Failed to create Vulkan shader module");

        let create_info = ash::vk::ShaderModuleCreateInfo::builder()
            .code(vert_code.as_slice())
            .build();
        let vert_mod = unsafe { device.create_shader_module(&create_info, None) }
            .expect("Failed to create Vulkan shader module");

        let entrypoint = CString::new("main").expect("Failed to create main entrypoint");
        let vert_stage = ash::vk::PipelineShaderStageCreateInfo::builder()
            .stage(ash::vk::ShaderStageFlags::VERTEX)
            .module(vert_mod)
            .name(&entrypoint)
            .build();
        let frag_stage = ash::vk::PipelineShaderStageCreateInfo::builder()
            .stage(ash::vk::ShaderStageFlags::FRAGMENT)
            .module(frag_mod)
            .name(&entrypoint)
            .build();

        let vertex_binding = ash::vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Vertex>() as u32)
            .input_rate(ash::vk::VertexInputRate::VERTEX)
            .build();

        let vertex_attribute = ash::vk::VertexInputAttributeDescription::builder()
            .location(0)
            .binding(0)
            .format(ash::vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build();

        let vertex_binding = [vertex_binding];
        let vertex_attribute = [vertex_attribute];

        let vertex_input = ash::vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_attribute_descriptions(&vertex_attribute)
            .vertex_binding_descriptions(&vertex_binding)
            .build();

        let input_assembly = ash::vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(ash::vk::PrimitiveTopology::TRIANGLE_LIST)
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
            .width(window.size().0 as f32)
            .height(window.size().1 as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .build()];

        let scissor = [ash::vk::Rect2D::builder()
            .offset(ash::vk::Offset2D::builder().x(0).y(0).build())
            .extent(
                ash::vk::Extent2D::builder()
                    .width(window.size().0)
                    .height(window.size().1)
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
            .render_pass(render_pass)
            .subpass(0)
            .layout(pipeline_layout)
            .build()];
        let pipelines = unsafe {
            device.create_graphics_pipelines(ash::vk::PipelineCache::null(), &create_info, None)
        }
        .expect("Failed to create Vulkan graphics pipeline");
        unsafe {
            device.destroy_shader_module(vert_mod, None);
            device.destroy_shader_module(frag_mod, None);
        }
        pipelines[0]
    };

    // Needed by cmd_begin_render_pass
    let render_area = ash::vk::Rect2D::builder()
        .offset(ash::vk::Offset2D::builder().x(0).y(0).build())
        .extent(
            ash::vk::Extent2D::builder()
                .width(window.size().0)
                .height(window.size().1)
                .build(),
        )
        .build();

    // Vertex buffer of triangle to draw
    let vertex_buffer_size = std::mem::size_of::<Vertex>() as u64 * 3;
    let buffer_create_info = ash::vk::BufferCreateInfo::builder()
        .size(vertex_buffer_size)
        .usage(ash::vk::BufferUsageFlags::VERTEX_BUFFER)
        .sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
        .build();
    let vertex_buffer = unsafe { device.create_buffer(&buffer_create_info, None) }
        .expect("Failed to create Vulkan vertex buffer");

    let requirements = unsafe { device.get_buffer_memory_requirements(vertex_buffer) };

    let memory_type_index: u32 = {
        let mut mem_index: u32 = 0;
        let memory_properties =
            unsafe { instance.get_physical_device_memory_properties(physical_dev) };
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
    let buffer_memory = unsafe { device.allocate_memory(&mem_allocate_info, None) }
        .expect("Failed to allocate Vulkan memory");

    let offset = ash::vk::DeviceSize::default();
    unsafe { device.bind_buffer_memory(vertex_buffer, buffer_memory, offset) }
        .expect("Failed to bind Vulkan memory to buffer");

    let mut current_frame = 0;
    let mut events = ctx.event_pump().expect("Failed to create SDL events");
    'running: loop {
        // Handle events
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        // Draw
        {
            // Wait for this frame to be ready
            let frame = &frames[current_frame];
            unsafe { device.wait_for_fences(&[frame.fence], true, u64::max_value()) }
                .expect("Failed to wait for Vulkan frame fence");
            unsafe { device.reset_fences(&[frame.fence]) }
                .expect("Failed to reset Vulkan frame fence");

            // Get next image
            let (image_index, _) = unsafe {
                swapchain_ext.acquire_next_image(
                    swapchain,
                    u64::max_value(),
                    frame.image_ready,
                    ash::vk::Fence::null(),
                )
            }
            .expect("Failed to acquire Vulkan next image");

            let begin_info = ash::vk::CommandBufferBeginInfo::builder().build();
            unsafe { device.begin_command_buffer(frame.command_buffer, &begin_info) }
                .expect("Failed to begin Vulkan command buffer");

            let mut clear = ash::vk::ClearValue::default();
            clear.color.float32 = [0.2, 0.3, 0.4, 1.0];
            let clear_values = [clear];
            let create_info = ash::vk::RenderPassBeginInfo::builder()
                .framebuffer(frame.framebuffer)
                .render_pass(render_pass)
                .render_area(render_area)
                .clear_values(&clear_values)
                .build();
            // Record it in the main command buffer
            let contents = ash::vk::SubpassContents::INLINE;
            unsafe { device.cmd_begin_render_pass(frame.command_buffer, &create_info, contents) };

            let graphics_bind_point = ash::vk::PipelineBindPoint::GRAPHICS;
            unsafe {
                device.cmd_bind_pipeline(
                    frame.command_buffer,
                    graphics_bind_point,
                    graphics_pipeline,
                )
            };

            let first_binding = 0;
            let buffers = [vertex_buffer];
            let offsets = [ash::vk::DeviceSize::default()];
            unsafe {
                device.cmd_bind_vertex_buffers(
                    frame.command_buffer,
                    first_binding,
                    &buffers,
                    &offsets,
                )
            }

            let vertex_device_size = ash::vk::DeviceSize::from(vertex_buffer_size);
            let flags = ash::vk::MemoryMapFlags::default();
            let data = unsafe { device.map_memory(buffer_memory, 0, vertex_device_size, flags) }
                .expect("Failed to map Vulkan memory");

            let vertices = [
                Vertex {
                    pos: Vec3f::new(-0.2, -0.2, 0.0),
                },
                Vertex {
                    pos: Vec3f::new(0.2, -0.2, 0.0),
                },
                Vertex {
                    pos: Vec3f::new(0.0, 0.2, 0.0),
                },
            ];
            unsafe {
                data.copy_from(vertices.as_ptr() as _, vertex_buffer_size as usize);
                device.unmap_memory(buffer_memory);

                device.cmd_draw(frame.command_buffer, 3, 1, 0, 0);

                device.cmd_end_render_pass(frame.command_buffer);
                device
                    .end_command_buffer(frame.command_buffer)
                    .expect("Failed to end command buffer");
            }

            // Wait for the image to be available ..
            let wait_semaphores = [frame.image_ready];
            // .. at color attachment output stage
            let wait_dst_stage_mask = [ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let command_buffers = [frame.command_buffer];
            let signal_semaphores = [frame.image_drawn];
            let submits = [ash::vk::SubmitInfo::builder()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_dst_stage_mask)
                .command_buffers(&command_buffers)
                .signal_semaphores(&signal_semaphores)
                .build()];
            unsafe { device.queue_submit(graphics_queue, &submits, frame.fence) }
                .expect("Failed to submit to Vulkan queue");

            // Present result
            let pres_image_indices = [image_index];
            let pres_swapchains = [swapchain];
            let pres_semaphores = [frame.image_drawn];
            let present_info = ash::vk::PresentInfoKHR::builder()
                .image_indices(&pres_image_indices)
                .swapchains(&pres_swapchains)
                .wait_semaphores(&pres_semaphores);
            unsafe { swapchain_ext.queue_present(graphics_queue, &present_info) }
                .expect("Failed to present to Vulkan swapchain");

            // Update current frame
            current_frame = (current_frame + 1) % swapchain_images.len();
        }
    }
}
