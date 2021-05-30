// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::{vk, Device};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use super::*;
use imgui as im;

/// This is the one that is going to be recreated
/// when the swapchain goes out of date
pub struct Framebuffer {
    // @todo Make a map of framebuffers indexed by render-pass as key
    pub framebuffer: vk::Framebuffer,
    pub depth_view: ImageView,
    pub depth_image: Image,
    pub image_view: vk::ImageView,
    pub width: u32,
    pub height: u32,
    device: Rc<Device>,
}

impl Framebuffer {
    pub fn new(dev: &Dev, image: &Image, pass: &Pass) -> Self {
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
            self.device.destroy_image_view(self.image_view, None);
        }
    }
}

/// Container of fallback resources for a frame such as
/// A white 1x1 pixel texture (image, view, and sampler)
pub struct Fallback {
    _white_image: Image,
    white_view: ImageView,
    white_sampler: Sampler,
}

impl Fallback {
    fn new(dev: &Dev) -> Self {
        let white = [255, 255, 255, 255];
        let white_image = Image::from_data(&dev, &white, 1, 1, vk::Format::R8G8B8A8_SRGB);

        let white_view = ImageView::new(&dev.device, &white_image);

        let white_sampler = Sampler::new(&dev.device);

        Self {
            _white_image: white_image,
            white_view,
            white_sampler,
        }
    }
}

type BufferCache<T> = HashMap<Handle<T>, Buffer>;

/// The frame cache contains resources that do not need to be recreated
/// when the swapchain goes out of date
pub struct FrameCache {
    pub gui_vertex_buffer: Buffer,
    pub gui_index_buffer: Buffer,

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

    pub fallback: Fallback,
}

impl FrameCache {
    pub fn new(dev: &mut Dev) -> Self {
        // Graphics command buffer (device, command pool)
        let command_buffer = CommandBuffer::new(&mut dev.graphics_command_pool);

        // Fence (device)
        let fence = Fence::signaled(&dev.device);

        let gui_vertex_buffer =
            Buffer::new::<im::DrawVert>(&dev.allocator, vk::BufferUsageFlags::VERTEX_BUFFER);
        let gui_index_buffer =
            Buffer::new::<u16>(&dev.allocator, vk::BufferUsageFlags::INDEX_BUFFER);

        Self {
            gui_vertex_buffer,
            gui_index_buffer,
            model_buffers: BufferCache::new(),
            view_buffers: BufferCache::new(),
            proj_buffers: BufferCache::new(),
            pipeline_cache: PipelineCache::new(&dev.device),
            command_buffer,
            fence,
            image_ready: Semaphore::new(&dev.device),
            image_drawn: Semaphore::new(&dev.device),
            fallback: Fallback::new(&dev),
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
            .max_depth(0.0)
            .min_depth(1.0)
            .build();
        self.res.command_buffer.set_viewport(&viewport);

        let scissor = vk::Rect2D::builder()
            .extent(vk::Extent2D::builder().width(width).height(height).build())
            .build();
        self.res.command_buffer.set_scissor(&scissor);
    }

    pub fn bind(&mut self, pipeline: &mut Pipeline, model: &Model, camera_node: Handle<Node>) {
        self.res.command_buffer.bind_pipeline(pipeline);

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
        let camera = model.cameras.get(node.camera).unwrap();

        let pipeline_layout = pipeline.layout;

        if let Some(sets) = self
            .res
            .pipeline_cache
            .descriptors
            .view_sets
            .get(&(pipeline_layout, camera_node))
        {
            self.res
                .command_buffer
                .bind_descriptor_sets(pipeline_layout, sets, 1);

            // If there is a descriptor set, there must be a buffer
            let view_buffer = self.res.view_buffers.get_mut(&camera_node).unwrap();
            view_buffer.upload(&node.trs.get_view_matrix());

            let proj_buffer = self.res.proj_buffers.get_mut(&node.camera).unwrap();
            proj_buffer.upload(&camera.proj);
        } else {
            // Allocate and write desc set for camera view
            // Camera set layout is at index 1 (use a constant?)
            let set_layouts = [pipeline.set_layouts[1]];
            let sets = self.res.pipeline_cache.descriptors.allocate(&set_layouts);

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
                .insert((pipeline_layout, camera_node), sets);
        }
    }

    pub fn draw<T: VertexInput>(
        &mut self,
        pipeline: &mut Pipeline,
        nodes: &Pack<Node>,
        meshes: &Pack<Mesh>,
        primitives: &Pack<Primitive>,
        samplers: &Pack<Sampler>,
        views: &Pack<ImageView>,
        textures: &Pack<Texture>,
        node: Handle<Node>,
    ) {
        self.res.command_buffer.bind_pipeline(pipeline);

        let children = nodes.get(node).unwrap().children.clone();
        for child in children {
            self.draw::<T>(
                pipeline, nodes, meshes, primitives, samplers, views, textures, child,
            );
        }

        let cnode = nodes.get(node).unwrap();

        let mesh = meshes.get(cnode.mesh);
        if mesh.is_none() {
            return ();
        }
        let mesh = mesh.unwrap();

        let pipeline_layout = pipeline.layout;
        if let Some(sets) = self
            .res
            .pipeline_cache
            .descriptors
            .model_sets
            .get(&(pipeline_layout, node))
        {
            self.res
                .command_buffer
                .bind_descriptor_sets(pipeline_layout, sets, 0);

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

            // Allocate and write descriptors
            let set_layouts = [pipeline.set_layouts[0]];
            let sets = self.res.pipeline_cache.descriptors.allocate(&set_layouts);
            T::write_set_model(&self.device, sets[0], &model_buffer);

            if let Some(texture) = textures.get(mesh.texture) {
                let view = views.get(texture.view);
                let sampler = samplers.get(texture.sampler);
                T::write_set_image(&self.device, sets[0], view.unwrap(), sampler.unwrap());
            } else {
                // Bind a white 1x1 pixel texture
                let view = &self.res.fallback.white_view;
                let sampler = &self.res.fallback.white_sampler;
                T::write_set_image(&self.device, sets[0], view, sampler);
            }

            self.res
                .command_buffer
                .bind_descriptor_sets(pipeline.layout, &sets, 0);

            self.res.model_buffers.insert(node, model_buffer);
            self.res
                .pipeline_cache
                .descriptors
                .model_sets
                .insert((pipeline_layout, node), sets);
        }

        for hprimitive in &mesh.primitives {
            let primitive = primitives.get(*hprimitive).unwrap();

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
    fn next_frame(&mut self, win: &Win, surface: &Surface, dev: &Dev, pass: &Pass)
        -> Option<Frame>;
    fn present(&mut self, frame: Frame, win: &Win, surface: &Surface, dev: &Dev, pass: &Pass);
}

/// Offscreen frames work on user allocated images
struct OffscreenFrames {
    _frames: Vec<Frame>,
    _images: Vec<vk::Image>,
}

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

    pub fn recreate(&mut self, win: &Win, surface: &Surface, dev: &Dev, pass: &Pass) {
        dev.wait();
        //drop(self.swapchain);
        self.current = 0;
        let (width, height) = win.window.drawable_size();
        self.swapchain.recreate(&surface, &dev, width, height);
        for i in 0..self.swapchain.images.len() {
            let frame = &mut self.frames[i];
            // Only this semaphore must be recreated to avoid validation errors
            // The image drawn one is still in use at the moment
            frame.res.image_ready = Semaphore::new(&dev.device);
            frame.buffer = Framebuffer::new(&dev, &self.swapchain.images[i], &pass);
        }
    }
}

impl Frames for SwapchainFrames {
    fn next_frame(
        &mut self,
        win: &Win,
        surface: &Surface,
        dev: &Dev,
        pass: &Pass,
    ) -> Option<Frame> {
        // Wait for this frame to be ready
        self.frames[0].res.wait();

        let acquire_res = unsafe {
            self.swapchain.ext.acquire_next_image(
                self.swapchain.swapchain,
                u64::max_value(),
                self.frames[0].res.image_ready.semaphore,
                vk::Fence::null(),
            )
        };

        match acquire_res {
            Ok((image_index, false)) => {
                self.image_index = image_index;
                Some(self.frames.remove(0))
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
        self.frames.push(frame);

        match self
            .frames
            .last_mut()
            .unwrap()
            .present(dev, &self.swapchain, self.image_index)
        {
            Ok(()) => (),
            // Recreate swapchain
            Err(ash::vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.recreate(win, surface, dev, pass);
            }
            Err(result) => panic!("{:?}", result),
        }
    }
}
