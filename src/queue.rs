// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::rc::Rc;

use super::*;
use ash::{version::DeviceV1_0, *};

pub struct Queue {
    queue: vk::Queue,
    device: Rc<Device>,
}

impl Queue {
    pub fn new(device: &Rc<Device>, queue_family_index: u32) -> Self {
        let device = device.clone();

        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };

        Queue { queue, device }
    }

    pub fn submit(&self, submits: &[vk::SubmitInfo], fence: Option<&mut Fence>) {
        let fence = match fence {
            Some(fence) => {
                fence.can_wait = true;
                fence.fence
            }
            None => vk::Fence::null(),
        };

        unsafe { self.device.queue_submit(self.queue, &submits, fence) }
            .expect("Failed to submit to Vulkan queue")
    }

    pub fn submit_draw(
        &self,
        command_buffer: &CommandBuffer,
        wait: vk::Semaphore,
        signal: &Semaphore,
        fence: Option<&mut Fence>,
    ) {
        // Wait for the image to be available ..
        let waits = [wait];
        // .. at color attachment output stage
        let wait_dst_stage_mask = [ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers = [command_buffer.command_buffer];
        let signals = [signal.semaphore];

        let submits = [ash::vk::SubmitInfo::builder()
            .wait_semaphores(&waits)
            .wait_dst_stage_mask(&wait_dst_stage_mask)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signals)
            .build()];

        self.submit(&submits, fence);
    }

    pub fn present(
        &self,
        image_index: u32,
        swapchain: &Swapchain,
        wait: &Semaphore,
    ) -> Result<(), ash::vk::Result> {
        let pres_image_indices = [image_index];
        let pres_swapchains = [swapchain.swapchain];
        let pres_semaphores = [wait.semaphore];
        let present_info = ash::vk::PresentInfoKHR::builder()
            .image_indices(&pres_image_indices)
            .swapchains(&pres_swapchains)
            .wait_semaphores(&pres_semaphores);

        let ret = unsafe { swapchain.ext.queue_present(self.queue, &present_info) };

        match ret {
            Ok(false) => Ok(()),
            // Suboptimal
            Ok(true) => Err(ash::vk::Result::ERROR_OUT_OF_DATE_KHR),
            Err(result) => Err(result),
        }
    }
}
