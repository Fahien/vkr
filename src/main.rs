// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{
    borrow::Cow,
    ffi::{CStr, CString},
};

use byteorder::{ByteOrder, NativeEndian};

use sdl::{event::Event, keyboard::Keycode};
use sdl2 as sdl;

use ash::{
    extensions::{ext::DebugUtils, khr},
    vk::Handle,
    vk::{self},
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
    _image_view: vk::ImageView,
    // @todo Make a map of framebuffers indexed by render-pass as key
    framebuffer: vk::Framebuffer,
    command_buffer: vk::CommandBuffer,
    fence: vk::Fence,
    image_ready: vk::Semaphore,
    image_drawn: vk::Semaphore,
}

impl Frame {
    fn new(
        device: &ash::Device,
        image: &vk::Image,
        surface_format: vk::Format,
        render_pass: vk::RenderPass,
        width: u32,
        height: u32,
        command_pool: vk::CommandPool,
    ) -> Self {
        // Image view into a swapchain images (device, image, format)
        let image_view = {
            let create_info = vk::ImageViewCreateInfo::builder()
                .image(*image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(surface_format)
                .components(
                    vk::ComponentMapping::builder()
                        .r(vk::ComponentSwizzle::IDENTITY)
                        .g(vk::ComponentSwizzle::IDENTITY)
                        .b(vk::ComponentSwizzle::IDENTITY)
                        .a(vk::ComponentSwizzle::IDENTITY)
                        .build(),
                )
                .subresource_range(
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
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

            let create_info = vk::FramebufferCreateInfo::builder()
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
            let alloc_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);

            let buffers = unsafe { device.allocate_command_buffers(&alloc_info) }
                .expect("Failed to allocate command buffer");
            buffers[0]
        };

        // Fence (device)
        let fence = {
            let create_info = vk::FenceCreateInfo::builder()
                .flags(vk::FenceCreateFlags::SIGNALED)
                .build();
            unsafe { device.create_fence(&create_info, None) }
        }
        .expect("Failed to create Vulkan fence");

        // Semaphores (device)
        let (image_ready, image_drawn) = {
            let create_info = vk::SemaphoreCreateInfo::builder().build();
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
            _image_view: image_view,
            framebuffer,
            command_buffer,
            fence,
            image_ready,
            image_drawn,
        }
    }
}

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

    let portabilty_enumeration_name = CString::new("VK_KHR_portability_enumeration").unwrap();
    let mut extensions_names = vec![
        DebugUtils::name().as_ptr(),
        portabilty_enumeration_name.as_ptr(),
    ];
    for ext in extensions.iter() {
        extensions_names.push(ext.as_ptr() as *const i8);
    }
    let layers = [CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
    let layer_names: Vec<*const i8> = layers.iter().map(|name| name.as_ptr()).collect();

    let entry = unsafe { ash::Entry::load() }.expect("Failed to create ash entry");
    let app_info = vk::ApplicationInfo {
        p_application_name: "Test" as *const str as _,
        api_version: vk::make_api_version(0, 1, 2, 0),
        ..Default::default()
    };
    let create_info = vk::InstanceCreateInfo::builder()
        .application_info(&app_info)
        .enabled_extension_names(&extensions_names)
        .enabled_layer_names(&layer_names)
        .flags(vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR);
    let instance = unsafe { entry.create_instance(&create_info, None) }
        .expect("Failed to create Vulkan instance");

    // Debugging callback
    let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
        .message_severity(
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING,
        )
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
        )
        .pfn_user_callback(Some(vk_debug));

    let debug_report_loader = DebugUtils::new(&entry, &instance);
    let _ = unsafe {
        debug_report_loader
            .create_debug_utils_messenger(&debug_info, None)
            .expect("Failed to create Vulkan debug callback")
    };

    // Surface (window, instance)
    let surface = window
        .vulkan_create_surface(instance.handle().as_raw() as usize)
        .expect("Failed to create surface");
    let surface: vk::SurfaceKHR = vk::Handle::from_raw(surface);
    let surface_ext = khr::Surface::new(&entry, &instance);

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
        if queue.queue_flags.contains(vk::QueueFlags::GRAPHICS) && supports_presentation {
            graphics_queue_index = i as u32;
            break;
        }
    }

    assert!(
        graphics_queue_index != std::u32::MAX,
        "Failed to find graphics queue"
    );

    // Logical device (physical device, surface, device required extensions (swapchain), queue information)
    let queue_infos = vec![vk::DeviceQueueCreateInfo::builder()
        .queue_family_index(graphics_queue_index)
        // Highest priority for a single graphics queue
        .queue_priorities(&[1.0])
        .build()];

    let portability_subset_name = CString::new("VK_KHR_portability_subset").unwrap();
    let device_extensions = [
        khr::Swapchain::name().as_ptr(),
        portability_subset_name.as_ptr(),
    ];
    let device_create_info = vk::DeviceCreateInfo::builder()
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
    let create_info = vk::CommandPoolCreateInfo::builder()
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
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
        surface_formats[1]
    };

    // Swapchain (instance, logical device, surface formats)
    let swapchain_ext = khr::Swapchain::new(&instance, &device);

    // This needs to be queried to prevent validation layers complaining
    let surface_capabilities =
        unsafe { surface_ext.get_physical_device_surface_capabilities(physical_dev, surface) }
            .expect("Failed to get Vulkan physical device surface capabilities");

    let swapchain = {
        let create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface)
            .min_image_count(2)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(
                vk::Extent2D::builder()
                    .width(window.size().0)
                    .height(window.size().1)
                    .build(),
            )
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::FIFO)
            .clipped(true);
        unsafe { swapchain_ext.create_swapchain(&create_info, None) }
            .expect("Failed to create Vulkan swapchain")
    };

    let swapchain_images = unsafe { swapchain_ext.get_swapchain_images(swapchain) }
        .expect("Failed to get Vulkan swapchain images");

    // Render pass (swapchain surface format, device)
    let render_pass = {
        let attachment = [vk::AttachmentDescription::builder()
            .format(surface_format.format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .build()];

        let attach_refs = [vk::AttachmentReference::builder()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build()];

        // Just one subpass
        let subpasses = [vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&attach_refs)
            .build()];

        let present_dependency = vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .build();

        let dependencies = [present_dependency];

        // Build the render pass
        let create_info = vk::RenderPassCreateInfo::builder()
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
            image,
            surface_format.format,
            render_pass,
            window.size().0,
            window.size().1,
            command_pool,
        ));
    }

    // Pipeline layout (device, shader reflection?)
    let pipeline_layout = {
        let create_info = vk::PipelineLayoutCreateInfo::builder().build();
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

        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code(frag_code.as_slice())
            .build();
        let frag_mod = unsafe { device.create_shader_module(&create_info, None) }
            .expect("Failed to create Vulkan shader module");

        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code(vert_code.as_slice())
            .build();
        let vert_mod = unsafe { device.create_shader_module(&create_info, None) }
            .expect("Failed to create Vulkan shader module");

        let entrypoint = CString::new("main").expect("Failed to create main entrypoint");
        let vert_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_mod)
            .name(&entrypoint)
            .build();
        let frag_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_mod)
            .name(&entrypoint)
            .build();

        let vertex_binding = vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build();

        let vertex_attribute = vk::VertexInputAttributeDescription::builder()
            .location(0)
            .binding(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build();

        let vertex_binding = [vertex_binding];
        let vertex_attribute = [vertex_attribute];

        let vertex_input = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_attribute_descriptions(&vertex_attribute)
            .vertex_binding_descriptions(&vertex_binding)
            .build();

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false)
            .build();

        let raster_state = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false)
            .line_width(1.0)
            .build();

        let viewport = [vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(window.size().0 as f32)
            .height(window.size().1 as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .build()];

        let scissor = [vk::Rect2D::builder()
            .offset(vk::Offset2D::builder().x(0).y(0).build())
            .extent(
                vk::Extent2D::builder()
                    .width(window.size().0)
                    .height(window.size().1)
                    .build(),
            )
            .build()];

        let view_state = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&viewport)
            .scissors(&scissor)
            .build();

        let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .sample_shading_enable(false)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false)
            .build();

        let blend_attachment = [vk::PipelineColorBlendAttachmentState::builder()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .build()];

        let blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .attachments(&blend_attachment)
            .build();

        let stages = [vert_stage, frag_stage];

        let create_info = [vk::GraphicsPipelineCreateInfo::builder()
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
            device.create_graphics_pipelines(vk::PipelineCache::null(), &create_info, None)
        }
        .expect("Failed to create Vulkan graphics pipeline");
        unsafe {
            device.destroy_shader_module(vert_mod, None);
            device.destroy_shader_module(frag_mod, None);
        }
        pipelines[0]
    };

    // Needed by cmd_begin_render_pass
    let render_area = vk::Rect2D::builder()
        .offset(vk::Offset2D::builder().x(0).y(0).build())
        .extent(
            vk::Extent2D::builder()
                .width(window.size().0)
                .height(window.size().1)
                .build(),
        )
        .build();

    // Vertex buffer of triangle to draw
    let vertex_buffer_size = std::mem::size_of::<Vertex>() as u64 * 3;
    let buffer_create_info = vk::BufferCreateInfo::builder()
        .size(vertex_buffer_size)
        .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .build();
    let vertex_buffer = unsafe { device.create_buffer(&buffer_create_info, None) }
        .expect("Failed to create Vulkan vertex buffer");

    let requirements = unsafe { device.get_buffer_memory_requirements(vertex_buffer) };

    let memory_type_index: u32 = {
        let mut mem_index: u32 = 0;
        let memory_properties =
            unsafe { instance.get_physical_device_memory_properties(physical_dev) };
        for (i, memtype) in memory_properties.memory_types.iter().enumerate() {
            let res: vk::MemoryPropertyFlags = memtype.property_flags
                & (vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT);
            if (requirements.memory_type_bits & (1 << i) != 0) && res.as_raw() != 0 {
                mem_index = i as u32;
            }
        }
        mem_index
    };
    if memory_type_index == 0 {
        panic!("Failed to find Vulkan memory type index");
    }

    let mem_allocate_info = vk::MemoryAllocateInfo::builder()
        .allocation_size(requirements.size)
        .memory_type_index(memory_type_index)
        .build();
    let buffer_memory = unsafe { device.allocate_memory(&mem_allocate_info, None) }
        .expect("Failed to allocate Vulkan memory");

    let offset = vk::DeviceSize::default();
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
                    vk::Fence::null(),
                )
            }
            .expect("Failed to acquire Vulkan next image");

            let begin_info = vk::CommandBufferBeginInfo::builder().build();
            unsafe { device.begin_command_buffer(frame.command_buffer, &begin_info) }
                .expect("Failed to begin Vulkan command buffer");

            let mut clear = vk::ClearValue::default();
            clear.color.float32 = [0.2, 0.3, 0.4, 1.0];
            let clear_values = [clear];
            let create_info = vk::RenderPassBeginInfo::builder()
                .framebuffer(frame.framebuffer)
                .render_pass(render_pass)
                .render_area(render_area)
                .clear_values(&clear_values)
                .build();
            // Record it in the main command buffer
            let contents = vk::SubpassContents::INLINE;
            unsafe { device.cmd_begin_render_pass(frame.command_buffer, &create_info, contents) };

            let graphics_bind_point = vk::PipelineBindPoint::GRAPHICS;
            unsafe {
                device.cmd_bind_pipeline(
                    frame.command_buffer,
                    graphics_bind_point,
                    graphics_pipeline,
                )
            };

            let first_binding = 0;
            let buffers = [vertex_buffer];
            let offsets = [vk::DeviceSize::default()];
            unsafe {
                device.cmd_bind_vertex_buffers(
                    frame.command_buffer,
                    first_binding,
                    &buffers,
                    &offsets,
                )
            }

            let flags = vk::MemoryMapFlags::default();
            let data = unsafe { device.map_memory(buffer_memory, 0, vertex_buffer_size, flags) }
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
            let wait_dst_stage_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let command_buffers = [frame.command_buffer];
            let signal_semaphores = [frame.image_drawn];
            let submits = [vk::SubmitInfo::builder()
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
            let present_info = vk::PresentInfoKHR::builder()
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
