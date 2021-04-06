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

        let depth_format = vk::Format::D32_SFLOAT;
        let mut depth_image = Image::new(
            &dev.allocator,
            image.extent.width,
            image.extent.height,
            depth_format,
        );
        depth_image.transition(&dev, vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

        let depth_view = ImageView::new(&dev.device, &depth_image);

        // Framebuffers (image_view, renderpass)
        let framebuffer = {
            let attachments = [image_view, depth_view.view];

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

type BufferCache<T> = HashMap<Handle<T>, Buffer>;

/// The frame cache contains resources that do not need to be recreated
/// when the swapchain goes out of date
pub struct FrameCache {
    /// Uniform buffers for model matrices associated to nodes
    pub model_buffers: BufferCache<Node>,

    /// Uniform buffers for view matrices associated to nodes with cameras
    pub view_buffers: BufferCache<Node>,

    // Uniform buffers for proj matrices associated to cameras
    pub proj_buffers: BufferCache<Camera>,

    pub pipeline_cache: PipelineCache,
    pub command_buffer: CommandBuffer,
    pub fence: Fence,
    pub image_ready: Semaphore,
    pub image_drawn: Semaphore,
}

impl FrameCache {
    pub fn new(dev: &mut Dev) -> Self {
        // Graphics command buffer (device, command pool)
        let command_buffer = CommandBuffer::new(&mut dev.graphics_command_pool);

        // Fence (device)
        let fence = Fence::signaled(&dev.device);

        Self {
            model_buffers: BufferCache::new(),
            view_buffers: BufferCache::new(),
            proj_buffers: BufferCache::new(),
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
            .begin_render_pass(pass, &self.buffer, area);

        let viewport = vk::Viewport::builder()
            .width(width as f32)
            .height(height as f32)
            .build();
        self.res.command_buffer.set_viewport(&viewport);

        let scissor = vk::Rect2D::builder()
            .extent(vk::Extent2D::builder().width(width).height(height).build())
            .build();
        self.res.command_buffer.set_scissor(&scissor);
    }

    pub fn bind(&mut self, pipeline: &mut Pipeline, model: &Model, camera_node: Handle<Node>) {
        self.res.command_buffer.bind_pipeline(pipeline);

        let node = model.nodes.get(camera_node).unwrap();
        assert!(model.cameras.get(node.camera).is_some());

        if let Some(sets) = self
            .res
            .pipeline_cache
            .descriptors
            .view_sets
            .get(&(pipeline.layout, camera_node))
        {
            self.res
                .command_buffer
                .bind_descriptor_sets(pipeline.layout, sets, 1);

            // If there is a descriptor set, there must be a buffer
            let view_buffer = self.res.view_buffers.get_mut(&camera_node).unwrap();
            view_buffer.upload(&node.trs.get_view_matrix());

            let camera = model.cameras.get(node.camera).unwrap();
            let proj_buffer = self.res.proj_buffers.get_mut(&node.camera).unwrap();
            proj_buffer.upload(&camera.proj);
        } else {
            // Allocate and write desc set for camera view
            // Camera set layout is at index 1 (use a constant?)
            let sets = self
                .res
                .pipeline_cache
                .descriptors
                .allocate(&[pipeline.set_layouts[1]]);

            if let Some(view_buffer) = self.res.view_buffers.get_mut(&camera_node) {
                // Buffer already there, just make the set pointing to it
                Camera::write_set_view(&self.device, sets[0], &view_buffer);
            } else {
                // Create a new buffer for this node's view matrix
                let mut view_buffer = Buffer::new::<na::Matrix4<f32>>(
                    &self.allocator,
                    vk::BufferUsageFlags::UNIFORM_BUFFER,
                );
                view_buffer.upload(&node.trs.get_view_matrix());
                Camera::write_set_view(&self.device, sets[0], &view_buffer);
                self.res.view_buffers.insert(camera_node, view_buffer);
            }

            if let Some(proj_buffer) = self.res.proj_buffers.get_mut(&node.camera) {
                // Buffer already there, just make the set pointing to it
                Camera::write_set_proj(&self.device, sets[0], &proj_buffer);
            } else {
                // Create a new buffer for this camera proj matrix
                let mut proj_buffer = Buffer::new::<na::Matrix4<f32>>(
                    &self.allocator,
                    vk::BufferUsageFlags::UNIFORM_BUFFER,
                );
                let camera = model.cameras.get(node.camera).unwrap();
                proj_buffer.upload(&camera.proj);
                Camera::write_set_proj(&self.device, sets[0], &proj_buffer);
                self.res.proj_buffers.insert(node.camera, proj_buffer);
            }

            self.res
                .command_buffer
                .bind_descriptor_sets(pipeline.layout, &sets, 1);

            self.res
                .pipeline_cache
                .descriptors
                .view_sets
                .insert((pipeline.layout, camera_node), sets);
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
        self.res.command_buffer.bind_pipeline(pipeline);

        let cnode = model.nodes.get(node).unwrap();
        if let Some(sets) = self
            .res
            .pipeline_cache
            .descriptors
            .model_sets
            .get(&(pipeline.layout, node))
        {
            self.res
                .command_buffer
                .bind_descriptor_sets(pipeline.layout, sets, 0);

            // If there is a descriptor set, there must be a uniform buffer
            let ubo = self.res.model_buffers.get_mut(&node).unwrap();
            ubo.upload(&cnode.trs.get_matrix());
        } else {
            // Create a new uniform buffer for this node's model matrix
            let mut model_buffer = Buffer::new::<na::Matrix4<f32>>(
                &self.allocator,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
            );
            model_buffer.upload(&cnode.trs.get_matrix());

            let texture = model.textures.get(texture);
            let (view, sampler) = match texture {
                Some(texture) => (
                    model.views.get(texture.view),
                    model.samplers.get(texture.sampler),
                ),
                _ => (None, None),
            };

            // Allocate and write descriptors
            // Model set layout is at index 0 (use a constant?)
            let sets = self
                .res
                .pipeline_cache
                .descriptors
                .allocate(&[pipeline.set_layouts[0]]);
            T::write_set(&self.device, sets[0], &model_buffer, view, sampler);

            self.res
                .command_buffer
                .bind_descriptor_sets(pipeline.layout, &sets, 0);

            self.res.model_buffers.insert(node, model_buffer);
            self.res
                .pipeline_cache
                .descriptors
                .model_sets
                .insert((pipeline.layout, node), sets);
        }

        self.res
            .command_buffer
            .bind_vertex_buffer(&primitive.vertices);

        if let Some(indices) = &primitive.indices {
            // Draw indexed if primitive has indices
            self.res.command_buffer.bind_index_buffer(indices);

            let index_count = indices.size as u32 / std::mem::size_of::<u16>() as u32;
            self.res.command_buffer.draw_indexed(index_count, 0, 0);
        } else {
            // Draw without indices
            self.res.command_buffer.draw(primitive.vertex_count);
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
