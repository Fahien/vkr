// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT;

use super::gfx::*;
use super::image::*;

use ash::*;

pub trait Frames {
    fn next_frame<'a>(&'a mut self) -> Result<&'a mut Frame, ash::vk::Result>;
    fn present(&mut self, dev: &Dev) -> Result<(), ash::vk::Result>;
}

/// Offscreen frames work on user allocated images
pub struct OffscreenFrames {
    frames: Vec<Frame>,
    pub images: Vec<Image>,
}

impl OffscreenFrames {
    pub fn new(dev: &mut Dev, width: u32, height: u32, pass: &Pass) -> Self {
        let format = vk::Format::R8G8B8A8_SRGB;
        let usage = vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC;
        let image = Image::new(&dev.allocator, width, height, format, usage);
        let frame = Frame::new(dev, &image, pass);

        Self {
            frames: vec![frame],
            images: vec![image],
        }
    }
}

impl Frames for OffscreenFrames {
    fn next_frame<'a>(&'a mut self) -> Result<&'a mut Frame, ash::vk::Result> {
        // Wait for this frame to be ready
        let frame = &mut self.frames[0];
        frame.res.wait();
        Ok(frame)
    }

    fn present(&mut self, dev: &Dev) -> Result<(), ash::vk::Result> {
        self.frames[0].submit(dev);
        Ok(())
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
    fn next_frame<'a>(&'a mut self) -> Result<&'a mut Frame, ash::vk::Result> {
        // Wait for this frame to be ready
        let frame = &mut self.frames[self.current];
        frame.res.wait();

        let acquire_res = unsafe {
            self.swapchain.ext.acquire_next_image(
                self.swapchain.swapchain,
                u64::max_value(),
                frame.res.image_ready.semaphore,
                ash::vk::Fence::null(),
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
                Err(ash::vk::Result::ERROR_OUT_OF_DATE_KHR)
            }
            Err(result) => {
                self.current = 0;
                Err(result)
            }
        }
    }

    fn present(&mut self, dev: &Dev) -> Result<(), ash::vk::Result> {
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
