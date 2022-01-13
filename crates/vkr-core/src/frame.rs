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
    /// List of nodes with cameras to render. At the beginning of a frame is empty.
    /// Then it is populated with calling `udpate()`, and TODO: cleared after submitting rendering commands.
    camera_nodes: Vec<Handle<Node>>,

    /// Used to compute the model-view matrix when rendering a mesh
    pub current_view: na::Matrix4<f32>,
    pub buffer: Framebuffer,
    pub res: Frameres,
    /// A frame should be able to allocate a uniform buffer on draw
    pub allocator: Rc<RefCell<vk_mem::Allocator>>,
    pub device: Rc<Device>,
}

impl Frame {
    pub fn new(dev: &mut Dev, image: &Image, pass: &Pass) -> Self {
        let buffer = Framebuffer::new(dev, image, pass);
        let res = Frameres::new(dev);

        Frame {
            camera_nodes: vec![],
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

    pub fn bind(&mut self, pipeline: &dyn Pipeline, model: &Model, camera_node: Handle<Node>) {
        self.res
            .command_buffer
            .bind_pipeline(pipeline.get_pipeline());

        let width = self.buffer.width as f32;
        let height = self.buffer.height as f32;
        let viewport = vk::Viewport::builder()
            .width(width)
            .height(height)
            .max_depth(0.0)
            .min_depth(1.0)
            .build();
        self.res.command_buffer.set_viewport(&viewport);

        let scissor = vk::Rect2D::builder()
            .extent(
                vk::Extent2D::builder()
                    .width(self.buffer.width)
                    .height(self.buffer.height)
                    .build(),
            )
            .build();
        self.res.command_buffer.set_scissor(&scissor);

        let node = model.nodes.get(camera_node).unwrap();
        self.current_view = node.trs.get_view_matrix();
        let camera = model.cameras.get(node.camera).unwrap();

        if let Some(sets) = self
            .res
            .descriptors
            .view_sets
            .get(&(pipeline.get_set_layouts()[1], camera_node))
        {
            self.res
                .command_buffer
                .bind_descriptor_sets(pipeline.get_layout(), sets, 1);

            // If there is a descriptor set, there must be a buffer
            let view_buffer = self.res.view_buffers.get_mut(&camera_node).unwrap();
            view_buffer.upload(&self.current_view);

            let proj_buffer = self.res.proj_buffers.get_mut(&node.camera).unwrap();
            proj_buffer.upload(&camera.proj);
        } else {
            // Allocate and write desc set for camera view
            // Camera set layout is at index 1 (use a constant?)
            let sets = self
                .res
                .descriptors
                .allocate(&[pipeline.get_set_layouts()[1]]);

            if let Some(_) = self.res.view_buffers.get_mut(&camera_node) {
                // Buffer already there, just make the set pointing to it
                // TODO: write sets?
                // Camera::write_set_view(self.device.borrow(), sets[0], &view_buffer);
            } else {
                if let Some(_) = self.res.proj_buffers.get_mut(&node.camera) {
                    // Buffer already there, just make the set pointing to it
                    // TODO: Write sets?
                    // Camera::write_set_proj(self.device.borrow(), sets[0], &proj_buffer);
                } else {
                    // Create a new buffer for this camera proj matrix
                    let mut proj_buffer = Buffer::new::<na::Matrix4<f32>>(
                        &self.allocator,
                        vk::BufferUsageFlags::UNIFORM_BUFFER,
                    );
                    proj_buffer.upload(&camera.proj);
                    self.res.proj_buffers.insert(node.camera, proj_buffer);
                }

                // Create a new buffer for this node's view matrix
                let mut view_buffer = Buffer::new::<na::Matrix4<f32>>(
                    &self.allocator,
                    vk::BufferUsageFlags::UNIFORM_BUFFER,
                );
                view_buffer.upload(&self.current_view);
                self.res.view_buffers.insert(camera_node, view_buffer);

                //    camera.write_set(self.device.borrow(), sets[0], self, camera_node);
            }

            self.res
                .command_buffer
                .bind_descriptor_sets(pipeline.get_layout(), &sets, 1);

            self.res
                .descriptors
                .view_sets
                .insert((pipeline.get_set_layouts()[1], camera_node), sets);
        }
    }

    /// Ideally it should not modify the model
    pub fn update(&mut self, model: &Model, node_handle: Handle<Node>) {
        let node = model.nodes.get(node_handle).unwrap();

        let children = node.children.clone();
        for child in children {
            self.update(model, child);
        }

        // Add this camera to the list of cameras to render
        if node.camera.valid() {
            self.camera_nodes.push(node_handle);
        }

        // TODO collect more info such as pipelines for each material
    }

    pub fn bind_pipe(&mut self, pipeline: &dyn Pipeline, model: &Model, node: Handle<Node>) {
        pipeline.bind(self, model, node);
    }

    pub fn draw_pipe_recursive(
        &mut self,
        pipeline: &dyn Pipeline,
        model: &Model,
        node: Handle<Node>,
    ) {
        let children = model.nodes.get(node).unwrap().children.clone();
        for child in children {
            self.draw_pipe(pipeline, model, child);
        }

        pipeline.draw(self, model, node);
    }

    pub fn draw_pipe(&mut self, pipeline: &dyn Pipeline, model: &Model, node: Handle<Node>) {
        // TODO: This function should just prepare stuff for drawing
        // Actual drawing should happen later, which means Pipeline trait can call this function
        // and then call another one to submit render commands.
        // In this function we can collect useful information, such as the camera
        // The the second function, we bind the camera, and then draw everything enqueued for rendering
        // We can even optimize some stuff with this approach.

        let camera_nodes = self.camera_nodes.clone();
        for camera_node_handle in camera_nodes {
            pipeline.bind(self, model, camera_node_handle);
            self.draw_pipe_recursive(pipeline, model, node);
        }
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
        // TODO: move this clear somewhere else, but do not keep it in the middle of recording/submitting commands
        self.camera_nodes.clear();

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
            frame
                .res
                .descriptors
                .free(&frame.res.descriptors.present_sets);
            frame.res.descriptors.present_sets.clear();
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
