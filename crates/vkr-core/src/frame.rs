// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::*;
use std::{cell::RefCell, rc::Rc};
use vkr_util::Handle;

use crate::*;

/// This is the one that is going to be recreated
/// when the swapchain goes out of date
pub struct Framebuffer {
    // @todo Make a map of framebuffers indexed by render-pass as key
    pub framebuffer: vk::Framebuffer,
    pub normal_view: ImageView,
    pub normal_image: Image,
    pub depth_view: ImageView,
    pub depth_image: Image,
    pub albedo_view: ImageView,
    pub albedo_image: Image,
    /// Image view into a swapchain image
    pub swapchain_view: vk::ImageView,
    pub width: u32,
    pub height: u32,
    device: Rc<Device>,
}

impl Framebuffer {
    pub fn new(dev: &Dev, image: &Image, pass: &Pass) -> Self {
        // Image view into a swapchain images (device, image, format)
        let swapchain_view = {
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

        // Albedo image with the same settings as the swapchain image
        let mut albedo_image = Image::attachment(
            &dev.allocator,
            image.extent.width,
            image.extent.height,
            image.format,
        );
        albedo_image.transition(&dev, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let albedo_view = ImageView::new(&dev.device, &albedo_image);

        // Depth image
        let depth_format = vk::Format::D32_SFLOAT;
        let mut depth_image = Image::attachment(
            &dev.allocator,
            image.extent.width,
            image.extent.height,
            depth_format,
        );
        depth_image.transition(&dev, vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

        let depth_view = ImageView::new(&dev.device, &depth_image);

        // Normal image
        let normal_format = vk::Format::A2R10G10B10_UNORM_PACK32;
        let mut normal_image = Image::attachment(
            &dev.allocator,
            image.extent.width,
            image.extent.height,
            normal_format,
        );
        normal_image.transition(&dev, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let normal_view = ImageView::new(&dev.device, &normal_image);

        // Framebuffers (image_views, renderpass)
        let framebuffer = {
            // Swapchain, depth, albedo
            let attachments = [
                swapchain_view,
                depth_view.view,
                albedo_view.view,
                normal_view.view,
            ];

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
            normal_view,
            normal_image,
            depth_view,
            depth_image,
            albedo_view,
            albedo_image,
            swapchain_view,
            width: image.extent.width,
            height: image.extent.height,
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
            self.device.destroy_image_view(self.swapchain_view, None);
        }
    }
}

pub struct Frame {
    /// Used to compute the model-view matrix when rendering a mesh
    pub current_view: na::Matrix4<f32>,
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
            current_view: na::Matrix4::identity(),
            buffer,
            res,
            allocator: dev.allocator.clone(),
            device: Rc::clone(&dev.device),
        }
    }

    pub fn begin(&self, pass: &Pass, width: u32, height: u32) {
        self.res
            .command_buffer
            .begin(vk::CommandBufferUsageFlags::default());

        // Needed by cmd_begin_render_pass
        let area = vk::Rect2D::builder()
            .offset(vk::Offset2D::builder().x(0).y(0).build())
            .extent(vk::Extent2D::builder().width(width).height(height).build())
            .build();

        self.res
            .command_buffer
            .begin_render_pass(pass.render, self.buffer.framebuffer, area);

        let viewport = vk::Viewport::builder()
            .width(width as f32)
            .height(height as f32)
            .max_depth(0.0)
            .min_depth(1.0)
            .build();
        self.res.command_buffer.set_viewport(&viewport);

        let scissor = vk::Rect2D::builder()
            .extent(vk::Extent2D::builder().width(width).height(height).build())
            .build();
        self.res.command_buffer.set_scissor(&scissor);
    }

    pub fn draw_pipe(&mut self, pipeline: &dyn Pipeline, model: &Model, node: Handle<Node>) {
        let children = model.nodes.get(node).unwrap().children.clone();
        for child in children {
            self.draw_pipe(pipeline, model, child);
        }

        pipeline.draw(self, model, node);
    }

    pub fn end(&self) {
        self.res.command_buffer.end_render_pass();
        self.res.command_buffer.end()
    }

    pub fn present(
        &mut self,
        dev: &Dev,
        swapchain: &Swapchain,
        image_index: u32,
    ) -> Result<(), vk::Result> {
        dev.graphics_queue.submit_draw(
            &self.res.command_buffer,
            self.res.image_ready.semaphore,
            self.res.image_drawn.semaphore,
            Some(&mut self.res.fence),
        );

        dev.graphics_queue
            .present(image_index, swapchain, self.res.image_drawn.semaphore)
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

#[cfg(feature = "win")]
pub trait Frames {
    fn next_frame(&mut self, win: &Win, surface: &Surface, dev: &Dev, pass: &Pass)
        -> Option<Frame>;
    fn present(&mut self, frame: Frame, win: &Win, surface: &Surface, dev: &Dev, pass: &Pass);
}

/// Offscreen frames work on user allocated images
struct OffscreenFrames {
    _frames: Vec<Frame>,
    _images: Vec<vk::Image>,
}

#[cfg(feature = "win")]
impl Frames for OffscreenFrames {
    fn next_frame(
        &mut self,
        _win: &Win,
        _surface: &Surface,
        _dev: &Dev,
        _pass: &Pass,
    ) -> Option<Frame> {
        unimplemented!("Offscreen next frame");
    }

    fn present(&mut self, _frame: Frame, _win: &Win, _surface: &Surface, _dev: &Dev, _pass: &Pass) {
        unimplemented!("Offscreen present");
    }
}

/// Swapchain frames work on swapchain images
pub struct SwapchainFrames {
    /// This is the frame index. It is updated in a round-robin fashon.
    /// It is not necessarily the same as the image index.
    pub current: usize,

    /// This is the index of the swapchain image currently in use.
    /// Not necessarily the same as the frame index.
    image_index: u32,

    /// We use option here because this vector should not change size.
    /// This means when a frame is retrieved for drawing, we take the frame and replace it with None.
    /// When the frame is returned for presenting, we put it back in its original position.
    pub frames: Vec<Option<Frame>>,
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
            frames.push(Some(frame));
        }

        Self {
            current: 0,
            image_index: 0,
            frames,
            swapchain,
        }
    }

    #[cfg(feature = "win")]
    pub fn recreate(&mut self, win: &Win, surface: &Surface, dev: &Dev, pass: &Pass) {
        dev.wait();
        self.current = 0;
        let (width, height) = win.window.drawable_size();
        self.swapchain.recreate(&surface, &dev, width, height);
        for i in 0..self.swapchain.images.len() {
            let frame = self.frames[i].as_mut().unwrap();
            frame.buffer = Framebuffer::new(&dev, &self.swapchain.images[i], &pass);
        }
    }
}

#[cfg(feature = "win")]
impl Frames for SwapchainFrames {
    fn next_frame(
        &mut self,
        win: &Win,
        surface: &Surface,
        dev: &Dev,
        pass: &Pass,
    ) -> Option<Frame> {
        // Let us create a new semaphore for next image
        let image_ready = Semaphore::new(&dev.device);

        let acquire_res = unsafe {
            self.swapchain.ext.acquire_next_image(
                self.swapchain.swapchain,
                u64::MAX,
                image_ready.semaphore,
                vk::Fence::null(),
            )
        };

        match acquire_res {
            Ok((image_index, false)) => {
                self.image_index = image_index;
                self.current = image_index as usize;
                let mut frame = self.frames[self.current].take().unwrap();
                // When previous draw is finished, the fence is signaled, let us wait for it.
                frame.res.wait();
                // At this point the image should be ready and we can safely overwrite previous semaphore.
                frame.res.image_ready = image_ready;
                Some(frame)
            }
            // Suboptimal
            Ok((_, true)) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.recreate(win, surface, dev, pass);
                None
            }
            Err(result) => {
                panic!("{:?}", result);
            }
        }
    }

    fn present(&mut self, frame: Frame, win: &Win, surface: &Surface, dev: &Dev, pass: &Pass) {
        assert!(self.frames[self.current as usize].is_none());
        self.frames[self.current as usize].replace(frame);

        match self.frames[self.current as usize]
            .as_mut()
            .unwrap()
            .present(dev, &self.swapchain, self.image_index)
        {
            Ok(()) => {}
            // Recreate swapchain
            Err(ash::vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.recreate(win, surface, dev, pass);
            }
            Err(result) => panic!("{:?}", result),
        }
    }
}
