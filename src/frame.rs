// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::{vk, Device};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use super::*;

/// This is the one that is going to be recreated
/// when the swapchain goes out of date
pub struct Framebuffer {
    // @todo Make a map of framebuffers indexed by render-pass as key
    pub framebuffer: vk::Framebuffer,
    pub depth_view: ImageView,
    pub depth_image: Image,
    pub image_view: vk::ImageView,
    device: Rc<Device>,
}

impl Framebuffer {
    pub fn new(dev: &mut Dev, image: &Image, pass: &Pass) -> Self {
        // Image view into a swapchain images (device, image, format)
        let image_view = {
            let create_info = vk::ImageViewCreateInfo::builder()
                .image(image.image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(image.format)
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
            unsafe { dev.device.create_image_view(&create_info, None) }
                .expect("Failed to create Vulkan image view")
        };

        let depth_format = ash::vk::Format::D32_SFLOAT;
        let mut depth_image = Image::new(
            &dev.allocator,
            image.extent.width,
            image.extent.height,
            depth_format,
        );
        depth_image.transition(&dev, ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

        let depth_view = ImageView::new(&dev.device, &depth_image);

        // Framebuffers (image_view, renderpass)
        let framebuffer = {
            let attachments = [image_view];

            let create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(pass.render)
                .attachments(&attachments)
                .width(image.extent.width)
                .height(image.extent.height)
                .layers(1)
                .build();

            unsafe { dev.device.create_framebuffer(&create_info, None) }
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

/// The frame cache contains resources that do not need to be recreated
/// when the swapchain goes out of date
pub struct FrameCache {
    /// Uniform buffers for model matrix are associated to nodes
    ubos: HashMap<Handle<Node>, Buffer>,
    pub pipeline_cache: PipelineCache,
    pub command_buffer: vk::CommandBuffer,
    pub fence: Fence,
    pub image_ready: Semaphore,
    pub image_drawn: Semaphore,
}

impl FrameCache {
    pub fn new(dev: &mut Dev) -> Self {
        // Graphics command buffer (device, command pool)
        let command_buffer = {
            let alloc_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(dev.graphics_command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);

            let buffers = unsafe { dev.device.allocate_command_buffers(&alloc_info) }
                .expect("Failed to allocate command buffer");
            buffers[0]
        };

        // Fence (device)
        let fence = Fence::signaled(&dev.device);

        Self {
            ubos: HashMap::new(),
            pipeline_cache: PipelineCache::new(&dev.device),
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
    pub res: FrameCache,
    /// A frame should be able to allocate a uniform buffer on draw
    allocator: Rc<RefCell<vk_mem::Allocator>>,
    pub device: Rc<Device>,
}

impl Frame {
    pub fn new(dev: &mut Dev, image: &Image, pass: &Pass) -> Self {
        let buffer = Framebuffer::new(dev, image, pass);
        let res = FrameCache::new(dev);

        Frame {
            buffer,
            res,
            allocator: dev.allocator.clone(),
            device: Rc::clone(&dev.device),
        }
    }

    pub fn begin(&self, pass: &Pass, width: u32, height: u32) {
        let begin_info = vk::CommandBufferBeginInfo::builder().build();
        unsafe {
            self.device
                .begin_command_buffer(self.res.command_buffer, &begin_info)
        }
        .expect("Failed to begin Vulkan command buffer");

        // Needed by cmd_begin_render_pass
        let area = vk::Rect2D::builder()
            .offset(vk::Offset2D::builder().x(0).y(0).build())
            .extent(vk::Extent2D::builder().width(width).height(height).build())
            .build();

        let mut clear = vk::ClearValue::default();
        clear.color.float32 = [0.0, 10.0 / 255.0, 28.0 / 255.0, 1.0];
        let clear_values = [clear];
        let create_info = vk::RenderPassBeginInfo::builder()
            .framebuffer(self.buffer.framebuffer)
            .render_pass(pass.render)
            .render_area(area)
            .clear_values(&clear_values)
            .build();
        // Record it in the main command buffer
        let contents = vk::SubpassContents::INLINE;
        unsafe {
            self.device
                .cmd_begin_render_pass(self.res.command_buffer, &create_info, contents)
        };

        let viewports = [vk::Viewport::builder()
            .width(width as f32)
            .height(height as f32)
            .build()];
        unsafe {
            self.device
                .cmd_set_viewport(self.res.command_buffer, 0, &viewports)
        };

        let scissors = [vk::Rect2D::builder()
            .extent(vk::Extent2D::builder().width(width).height(height).build())
            .build()];
        unsafe {
            self.device
                .cmd_set_scissor(self.res.command_buffer, 0, &scissors)
        }
    }

    pub fn draw<T: VertexInput>(
        &mut self,
        pipeline: &mut Pipeline,
        model: &Model,
        primitive: &Primitive,
        node: Handle<Node>,
        texture: Handle<Texture>,
    ) {
        let graphics_bind_point = vk::PipelineBindPoint::GRAPHICS;
        unsafe {
            self.device.cmd_bind_pipeline(
                self.res.command_buffer,
                graphics_bind_point,
                pipeline.graphics,
            );
        }

        if let Some(sets) = self
            .res
            .pipeline_cache
            .descriptors
            .sets
            .get(&(pipeline.layout, node))
        {
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
            let mut ubo = Buffer::new::<na::Matrix4<f32>>(
                &self.allocator,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
            );
            ubo.upload(&model.nodes.get(node).unwrap().trs.get_matrix());

            let sets = self
                .res
                .pipeline_cache
                .descriptors
                .allocate(&[pipeline.set_layout]);

            let texture = model.textures.get(texture);
            let (view, sampler) = match texture {
                Some(texture) => (
                    model.views.get(texture.view),
                    model.samplers.get(texture.sampler),
                ),
                _ => (None, None),
            };
            T::write_set(&self.device, sets[0], &ubo, view, sampler);

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
                .pipeline_cache
                .descriptors
                .sets
                .insert((pipeline.layout, node), sets);
        }

        let first_binding = 0;
        let buffers = [primitive.vertices.buffer];
        let offsets = [vk::DeviceSize::default()];
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
                    vk::IndexType::UINT16,
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

    pub fn present(
        &mut self,
        dev: &Dev,
        swapchain: &Swapchain,
        image_index: u32,
    ) -> Result<(), vk::Result> {
        dev.graphics_queue.submit_draw(
            &self.res.command_buffer,
            &self.res.image_ready,
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

pub trait Frames {
    fn next_frame<'a>(&'a mut self) -> Result<&'a mut Frame, vk::Result>;
    fn present(&mut self, dev: &Dev) -> Result<(), vk::Result>;
}

/// Offscreen frames work on user allocated images
struct OffscreenFrames {
    _frames: Vec<Frame>,
    _images: Vec<vk::Image>,
}

impl Frames for OffscreenFrames {
    fn next_frame<'a>(&'a mut self) -> Result<&'a mut Frame, vk::Result> {
        // Unimplemented
        Err(vk::Result::ERROR_UNKNOWN)
    }

    fn present(&mut self, _dev: &Dev) -> Result<(), vk::Result> {
        // Unimplemented
        Err(vk::Result::ERROR_UNKNOWN)
    }
}

/// Swapchain frames work on swapchain images
pub struct SwapchainFrames {
    pub current: usize,
    image_index: u32,
    pub frames: Vec<Frame>,
    pub swapchain: Swapchain,
}

impl SwapchainFrames {
    pub fn new(
        ctx: &Ctx,
        surface: &Surface,
        dev: &mut Dev,
        width: u32,
        height: u32,
        pass: &Pass,
    ) -> Self {
        let swapchain = Swapchain::new(ctx, surface, dev, width, height);

        let mut frames = Vec::new();
        for image in swapchain.images.iter() {
            let frame = Frame::new(dev, image, pass);
            frames.push(frame);
        }

        Self {
            current: 0,
            image_index: 0,
            frames: frames,
            swapchain,
        }
    }
}

impl Frames for SwapchainFrames {
    fn next_frame<'a>(&'a mut self) -> Result<&'a mut Frame, vk::Result> {
        // Wait for this frame to be ready
        let frame = &mut self.frames[self.current];
        frame.res.wait();

        let acquire_res = unsafe {
            self.swapchain.ext.acquire_next_image(
                self.swapchain.swapchain,
                u64::max_value(),
                frame.res.image_ready.semaphore,
                vk::Fence::null(),
            )
        };

        match acquire_res {
            Ok((image_index, false)) => {
                self.image_index = image_index;
                Ok(frame)
            }
            // Suboptimal
            Ok((_, true)) => {
                self.current = 0;
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR)
            }
            Err(result) => {
                self.current = 0;
                Err(result)
            }
        }
    }

    fn present(&mut self, dev: &Dev) -> Result<(), vk::Result> {
        match self.frames[self.current].present(dev, &self.swapchain, self.image_index) {
            Ok(()) => {
                self.current = (self.current + 1) % self.frames.len();
                Ok(())
            }
            Err(result) => {
                self.current = 0;
                Err(result)
            }
        }
    }
}
