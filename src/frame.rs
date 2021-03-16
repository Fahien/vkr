// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::{version::DeviceV1_0, *};
use std::{cell::RefCell, collections::HashMap, ops::Deref, rc::Rc};

use super::*;

/// This is the one that is going to be recreated
/// when the swapchain goes out of date
pub struct Framebuffer {
    pub area: vk::Rect2D,
    // @todo Make a map of framebuffers indexed by render-pass as key
    pub framebuffer: vk::Framebuffer,
    pub image_view: vk::ImageView,
    pub image: Rc<RefCell<Image>>,
    device: Rc<Device>,
}

impl Framebuffer {
    pub fn new(dev: &mut Dev, image: Rc<RefCell<Image>>, pass: &Pass) -> Self {
        let image_ref = image.deref().borrow();

        // Image view into a swapchain images (device, image, format)
        let image_view = {
            let create_info = vk::ImageViewCreateInfo::builder()
                .image(image_ref.image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(image_ref.format)
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

        // Framebuffers (image_view, renderpass)
        let framebuffer = {
            let attachments = [image_view];

            let create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(pass.render)
                .attachments(&attachments)
                .width(image_ref.width)
                .height(image_ref.height)
                .layers(1)
                .build();

            unsafe { dev.device.create_framebuffer(&create_info, None) }
                .expect("Failed to create Vulkan framebuffer")
        };

        // Needed by cmd_begin_render_pass
        let area = vk::Rect2D::builder()
            .offset(vk::Offset2D::builder().x(0).y(0).build())
            .extent(
                vk::Extent2D::builder()
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
    /// Uniform buffers for model matrix are associated to nodes
    ubos: HashMap<Handle<Node>, Buffer>,
    descriptors: Descriptors,
    pub command_buffer: vk::CommandBuffer,
    pub fence: vk::Fence,
    pub can_wait: bool,
    pub image_ready: vk::Semaphore,
    pub image_drawn: vk::Semaphore,
    device: Rc<Device>,
}

impl Frameres {
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
        let fence = {
            let create_info = vk::FenceCreateInfo::builder()
                .flags(vk::FenceCreateFlags::SIGNALED)
                .build();
            unsafe { dev.device.create_fence(&create_info, None) }
        }
        .expect("Failed to create Vulkan fence");

        // Semaphores (device)
        let (image_ready, image_drawn) = {
            let create_info = vk::SemaphoreCreateInfo::builder().build();
            unsafe {
                (
                    dev.device
                        .create_semaphore(&create_info, None)
                        .expect("Failed to create Vulkan semaphore"),
                    dev.device
                        .create_semaphore(&create_info, None)
                        .expect("Failed to create Vulkan semaphore"),
                )
            }
        };

        Self {
            ubos: HashMap::new(),
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

        unsafe {
            self.device
                .wait_for_fences(&[self.fence], true, u64::max_value())
                .expect("Failed to wait for Vulkan frame fence");
            self.device
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
    /// A frame should be able to allocate a uniform buffer on draw
    allocator: Rc<RefCell<vk_mem::Allocator>>,
    pub device: Rc<Device>,
}

impl Frame {
    pub fn new(dev: &mut Dev, image: Rc<RefCell<Image>>, pass: &Pass) -> Self {
        let buffer = Framebuffer::new(dev, image, pass);
        let res = Frameres::new(dev);

        Frame {
            buffer,
            res,
            allocator: dev.allocator.clone(),
            device: Rc::clone(&dev.device),
        }
    }

    pub fn begin(&self, pass: &Pass) {
        let begin_info = vk::CommandBufferBeginInfo::builder().build();
        unsafe {
            self.device
                .begin_command_buffer(self.res.command_buffer, &begin_info)
        }
        .expect("Failed to begin Vulkan command buffer");

        let mut clear = vk::ClearValue::default();
        clear.color.float32 = [0.025, 0.025, 0.025, 1.0];
        let clear_values = [clear];
        let create_info = vk::RenderPassBeginInfo::builder()
            .framebuffer(self.buffer.framebuffer)
            .render_pass(pass.render)
            .render_area(self.buffer.area)
            .clear_values(&clear_values)
            .build();
        // Record it in the main command buffer
        let contents = vk::SubpassContents::INLINE;
        unsafe {
            self.device
                .cmd_begin_render_pass(self.res.command_buffer, &create_info, contents)
        };
    }

    pub fn draw(
        &mut self,
        pipeline: &Pipeline,
        nodes: &Pack<Node>,
        primitive: &Primitive,
        node: Handle<Node>,
    ) {
        let graphics_bind_point = vk::PipelineBindPoint::GRAPHICS;
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
            ubo.upload(&nodes.get(node).unwrap().trs.get_matrix());
        } else {
            // Create a new uniform buffer for this node's model matrix
            let mut ubo = Buffer::new::<na::Matrix4<f32>>(
                &self.allocator,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
            );
            ubo.upload(&nodes.get(node).unwrap().trs.get_matrix());

            let sets = self.res.descriptors.allocate(&[pipeline.set_layout]);

            // Update immediately the descriptor sets
            let buffer_info = vk::DescriptorBufferInfo::builder()
                .range(std::mem::size_of::<na::Matrix4<f32>>() as vk::DeviceSize)
                .buffer(ubo.buffer)
                .build();

            let descriptor_write = vk::WriteDescriptorSet::builder()
                .dst_set(sets[0])
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
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

            self.res.ubos.insert(node, ubo);
            self.res
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

    pub fn present(
        &mut self,
        dev: &Dev,
        swapchain: &Swapchain,
        image_index: u32,
    ) -> Result<(), vk::Result> {
        // Wait for the image to be available ..
        let wait_semaphores = [self.res.image_ready];
        // .. at color attachment output stage
        let wait_dst_stage_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers = [self.res.command_buffer];
        let signal_semaphores = [self.res.image_drawn];
        let submits = [vk::SubmitInfo::builder()
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
        let present_info = vk::PresentInfoKHR::builder()
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
            let frame = Frame::new(dev, Rc::clone(image), pass);
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
                frame.res.image_ready,
                vk::Fence::null(),
            )
        };

        match acquire_res {
            Ok((image_index, _)) => {
                self.image_index = image_index;
                Ok(frame)
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
