// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::{version::DeviceV1_0, *};
use std::{
    borrow::{Borrow, BorrowMut},
    rc::Rc,
};

use super::*;

pub struct Frame {
    pub area: vk::Rect2D,
    pub image_view: vk::ImageView,
    // @todo Make a map of framebuffers indexed by render-pass as key
    pub framebuffer: vk::Framebuffer,
    pub command_buffer: vk::CommandBuffer,
    pub fence: vk::Fence,
    pub image_ready: vk::Semaphore,
    pub image_drawn: vk::Semaphore,
    device: Rc<Device>,
}

impl Frame {
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

            let create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(pass.render)
                .attachments(&attachments)
                .width(image.width)
                .height(image.height)
                .layers(1)
                .build();

            unsafe {
                dev.device
                    .borrow_mut()
                    .create_framebuffer(&create_info, None)
            }
            .expect("Failed to create Vulkan framebuffer")
        };

        // Graphics command buffer (device, command pool)
        let command_buffer = {
            let alloc_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(dev.graphics_command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
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
            let create_info = vk::FenceCreateInfo::builder()
                .flags(vk::FenceCreateFlags::SIGNALED)
                .build();
            unsafe { dev.device.borrow_mut().create_fence(&create_info, None) }
        }
        .expect("Failed to create Vulkan fence");

        // Semaphores (device)
        let (image_ready, image_drawn) = {
            let create_info = vk::SemaphoreCreateInfo::builder().build();
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

        // Needed by cmd_begin_render_pass
        let area = vk::Rect2D::builder()
            .offset(vk::Offset2D::builder().x(0).y(0).build())
            .extent(
                vk::Extent2D::builder()
                    .width(image.width)
                    .height(image.height)
                    .build(),
            )
            .build();

        Frame {
            area,
            image_view,
            framebuffer,
            command_buffer,
            fence,
            image_ready,
            image_drawn,
            device: Rc::clone(&dev.device),
        }
    }

    pub fn wait(&self) {
        unsafe {
            let device: &Device = self.device.borrow();
            device
                .wait_for_fences(&[self.fence], true, u64::max_value())
                .expect("Failed to wait for Vulkan frame fence");
            device
                .reset_fences(&[self.fence])
                .expect("Failed to reset Vulkan frame fence");
        }
    }

    pub fn begin(&self, pass: &Pass) {
        let begin_info = vk::CommandBufferBeginInfo::builder().build();
        unsafe {
            self.device
                .begin_command_buffer(self.command_buffer, &begin_info)
        }
        .expect("Failed to begin Vulkan command buffer");

        let mut clear = vk::ClearValue::default();
        clear.color.float32 = [0.025, 0.025, 0.025, 1.0];
        let clear_values = [clear];
        let create_info = vk::RenderPassBeginInfo::builder()
            .framebuffer(self.framebuffer)
            .render_pass(pass.render)
            .render_area(self.area)
            .clear_values(&clear_values)
            .build();
        // Record it in the main command buffer
        let contents = vk::SubpassContents::INLINE;
        unsafe {
            self.device
                .cmd_begin_render_pass(self.command_buffer, &create_info, contents)
        };
    }

    pub fn draw(&self, pipeline: &Pipeline, buffer: &Buffer) {
        let graphics_bind_point = vk::PipelineBindPoint::GRAPHICS;
        unsafe {
            self.device.cmd_bind_pipeline(
                self.command_buffer,
                graphics_bind_point,
                pipeline.graphics,
            )
        };

        let first_binding = 0;
        let buffers = [buffer.buffer];
        let offsets = [vk::DeviceSize::default()];
        unsafe {
            self.device.cmd_bind_vertex_buffers(
                self.command_buffer,
                first_binding,
                &buffers,
                &offsets,
            );
            self.device.cmd_draw(self.command_buffer, 3, 1, 0, 0);
        }
    }

    pub fn end(&self) {
        unsafe {
            self.device.cmd_end_render_pass(self.command_buffer);
            self.device
                .end_command_buffer(self.command_buffer)
                .expect("Failed to end command buffer");
        }
    }

    pub fn present(&self, dev: &Dev, swapchain: &Swapchain, image_index: u32) {
        // Wait for the image to be available ..
        let wait_semaphores = [self.image_ready];
        // .. at color attachment output stage
        let wait_dst_stage_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers = [self.command_buffer];
        let signal_semaphores = [self.image_drawn];
        let submits = [vk::SubmitInfo::builder()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_dst_stage_mask)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores)
            .build()];
        unsafe {
            self.device
                .queue_submit(dev.graphics_queue, &submits, self.fence)
        }
        .expect("Failed to submit to Vulkan queue");

        // Present result
        let pres_image_indices = [image_index];
        let pres_swapchains = [swapchain.swapchain];
        let pres_semaphores = [self.image_drawn];
        let present_info = vk::PresentInfoKHR::builder()
            .image_indices(&pres_image_indices)
            .swapchains(&pres_swapchains)
            .wait_semaphores(&pres_semaphores);
        unsafe {
            swapchain
                .ext
                .queue_present(dev.graphics_queue, &present_info)
        }
        .expect("Failed to present to Vulkan swapchain");
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        let dev = self.device.borrow_mut();
        unsafe {
            dev.destroy_semaphore(self.image_drawn, None);
            dev.destroy_semaphore(self.image_ready, None);
            dev.destroy_fence(self.fence, None);
            dev.destroy_framebuffer(self.framebuffer, None);
            dev.destroy_image_view(self.image_view, None);
        }
    }
}
