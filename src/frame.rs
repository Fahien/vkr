// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::*;
use std::{borrow::Borrow, cell::RefCell, collections::HashMap, rc::Rc};

use super::*;
use imgui as im;

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

/// Container of fallback resources for a frame such as
/// A white 1x1 pixel texture (image, view, and sampler)
pub struct Fallback {
    _white_image: Image,
    white_view: ImageView,
    /// A default sampler
    pub white_sampler: Sampler,
    white_material: Material,
    /// A triangle that covers the whole screen
    pub present_buffer: Buffer,
}

impl Fallback {
    fn new(dev: &Dev) -> Self {
        let white = [255, 255, 255, 255];
        let white_image = Image::from_data(&dev, &white, 1, 1, vk::Format::R8G8B8A8_SRGB);

        let white_view = ImageView::new(&dev.device, &white_image);

        let white_sampler = Sampler::new(&dev.device);

        let white_material = Material::new(Color::white());

        // Y pointing down
        let present_vertices = vec![
            PresentVertex::new(-1.0, -1.0),
            PresentVertex::new(-1.0, 3.0),
            PresentVertex::new(3.0, -1.0),
        ];
        let present_buffer = Buffer::new_arr(
            &dev.allocator,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            &present_vertices,
        );

        Self {
            _white_image: white_image,
            white_view,
            white_sampler,
            white_material,
            present_buffer,
        }
    }
}

type BufferCache<T> = HashMap<Handle<T>, Buffer>;

/// Frame resources that do not need to be recreated
/// when the swapchain goes out of date
pub struct Frameres {
    pub gui_vertex_buffer: Buffer,
    pub gui_index_buffer: Buffer,

    /// Uniform buffers for model matrices associated to nodes
    pub model_buffers: BufferCache<Node>,

    /// Uniform buffers for model-view matrices associated to nodes
    pub model_view_buffers: BufferCache<Node>,

    /// Uniform buffers for view matrices associated to nodes with cameras
    pub view_buffers: BufferCache<Node>,

    // Uniform buffers for proj matrices associated to cameras
    pub proj_buffers: BufferCache<Camera>,

    // Uniform buffers for materials
    pub material_buffers: BufferCache<Material>,

    pub descriptors: Descriptors,
    pub command_buffer: CommandBuffer,

    pub fence: Fence,

    // The image ready semaphore is used by the acquire next image function and it will be signaled
    // then the image is ready to be rendered onto. Indeed it is also used by the submit draw
    // function which will wait for the image to be ready before submitting draw commands
    pub image_ready: Semaphore,

    // Image drawn sempahore is used when submitting draw commands to a back-buffer
    // and it will be signaled when rendering is finished. Indeed the present function
    // is waiting on this sempahore before presenting the back-buffer to screen.
    pub image_drawn: Semaphore,

    pub fallback: Fallback,
}

impl Frameres {
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
            model_view_buffers: BufferCache::new(),
            view_buffers: BufferCache::new(),
            proj_buffers: BufferCache::new(),
            material_buffers: BufferCache::new(),
            descriptors: Descriptors::new(dev),
            command_buffer,
            fence,
            image_ready: Semaphore::new(&dev.device),
            image_drawn: Semaphore::new(&dev.device),
            fallback: Fallback::new(&dev),
        }
    }

    pub fn wait(&mut self) {
        self.fence.wait();
        self.fence.reset();
    }
}

pub struct Frame {
    /// Used to compute the model-view matrix when rendering a mesh
    pub current_view: na::Matrix4<f32>,
    pub buffer: Framebuffer,
    pub res: Frameres,
    /// A frame should be able to allocate a uniform buffer on draw
    allocator: Rc<RefCell<vk_mem::Allocator>>,
    pub device: Rc<Device>,
}

impl Frame {
    pub fn new(dev: &mut Dev, image: &Image, pass: &Pass) -> Self {
        let buffer = Framebuffer::new(dev, image, pass);
        let res = Frameres::new(dev);

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

    pub fn bind(&mut self, pipeline: &Pipeline, model: &Model, camera_node: Handle<Node>) {
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
        self.current_view = node.trs.get_view_matrix();
        let camera = model.cameras.get(node.camera).unwrap();

        if let Some(sets) = self
            .res
            .descriptors
            .view_sets
            .get(&(pipeline.set_layouts[1], camera_node))
        {
            self.res
                .command_buffer
                .bind_descriptor_sets(pipeline, sets, 1);

            // If there is a descriptor set, there must be a buffer
            let view_buffer = self.res.view_buffers.get_mut(&camera_node).unwrap();
            view_buffer.upload(&self.current_view);

            let proj_buffer = self.res.proj_buffers.get_mut(&node.camera).unwrap();
            proj_buffer.upload(&camera.proj);
        } else {
            // Allocate and write desc set for camera view
            // Camera set layout is at index 1 (use a constant?)
            let sets = self.res.descriptors.allocate(&[pipeline.set_layouts[1]]);

            if let Some(view_buffer) = self.res.view_buffers.get_mut(&camera_node) {
                // Buffer already there, just make the set pointing to it
                Camera::write_set_view(self.device.borrow(), sets[0], &view_buffer);
            } else {
                // Create a new buffer for this node's view matrix
                let mut view_buffer = Buffer::new::<na::Matrix4<f32>>(
                    &self.allocator,
                    vk::BufferUsageFlags::UNIFORM_BUFFER,
                );
                view_buffer.upload(&self.current_view);
                Camera::write_set_view(self.device.borrow(), sets[0], &view_buffer);
                self.res.view_buffers.insert(camera_node, view_buffer);
            }

            if let Some(proj_buffer) = self.res.proj_buffers.get_mut(&node.camera) {
                // Buffer already there, just make the set pointing to it
                Camera::write_set_proj(self.device.borrow(), sets[0], &proj_buffer);
            } else {
                // Create a new buffer for this camera proj matrix
                let mut proj_buffer = Buffer::new::<na::Matrix4<f32>>(
                    &self.allocator,
                    vk::BufferUsageFlags::UNIFORM_BUFFER,
                );
                proj_buffer.upload(&camera.proj);
                Camera::write_set_proj(self.device.borrow(), sets[0], &proj_buffer);
                self.res.proj_buffers.insert(node.camera, proj_buffer);
            }

            self.res
                .command_buffer
                .bind_descriptor_sets(pipeline, &sets, 1);

            self.res
                .descriptors
                .view_sets
                .insert((pipeline.set_layouts[1], camera_node), sets);
        }
    }

    pub fn draw<T: VertexInput>(
        &mut self,
        pipelines: &DefaultPipelines,
        model: &Model,
        node: Handle<Node>,
    ) {
        let pipeline = pipelines.get_for::<T>();
        self.res.command_buffer.bind_pipeline(pipeline);

        let children = model.nodes.get(node).unwrap().children.clone();
        for child in children {
            self.draw::<T>(pipelines, model, child);
        }

        let cnode = model.nodes.get(node).unwrap();

        let mesh = model.meshes.get(cnode.mesh);
        if mesh.is_none() {
            return ();
        }
        let mesh = mesh.unwrap();

        let model_view_matrix = (self.current_view * cnode.trs.get_matrix())
            .try_inverse()
            .unwrap()
            .transpose();

        if let Some(sets) = self
            .res
            .descriptors
            .model_sets
            .get(&(pipeline.set_layouts[0], node))
        {
            // If there is a descriptor set, there must be a uniform buffer
            let ubo = self.res.model_buffers.get_mut(&node).unwrap();
            ubo.upload(&cnode.trs.get_matrix());

            let model_view_buffer = self.res.model_view_buffers.get_mut(&node).unwrap();
            model_view_buffer.upload(&model_view_matrix);

            self.res
                .command_buffer
                .bind_descriptor_sets(pipeline, sets, 0);
        } else {
            // Check the model buffer already exists
            let model_buffer = match self.res.model_buffers.get_mut(&node) {
                Some(b) => b,
                None => {
                    // Create a new uniform buffer for this node's model matrix
                    let buffer = Buffer::new::<na::Matrix4<f32>>(
                        &self.allocator,
                        vk::BufferUsageFlags::UNIFORM_BUFFER,
                    );
                    self.res.model_buffers.insert(node, buffer);
                    self.res.model_buffers.get_mut(&node).unwrap()
                }
            };
            model_buffer.upload(&cnode.trs.get_matrix());

            // Check whether the view-model buffer already exists
            let model_view_buffer = match self.res.model_view_buffers.get_mut(&node) {
                Some(b) => b,
                None => {
                    // Create a new uniform buffer for this node's model view matrix
                    let buffer = Buffer::new::<na::Matrix4<f32>>(
                        &self.allocator,
                        vk::BufferUsageFlags::UNIFORM_BUFFER,
                    );
                    self.res.model_view_buffers.insert(node, buffer);
                    self.res.model_view_buffers.get_mut(&node).unwrap()
                }
            };

            // Allocate and write descriptors
            let sets = self.res.descriptors.allocate(&[pipeline.set_layouts[0]]);
            T::write_set_model(self.device.borrow(), sets[0], &model_buffer);
            T::write_set_model_view(self.device.borrow(), sets[0], &model_view_buffer);

            self.res
                .command_buffer
                .bind_descriptor_sets(pipeline, &sets, 0);

            self.res
                .descriptors
                .model_sets
                .insert((pipeline.set_layouts[0], node), sets);
        }

        for hprimitive in &mesh.primitives {
            let primitive = model.primitives.get(*hprimitive).unwrap();

            // Does this pipeline support materials at all?
            if pipeline.set_layouts.len() > 2 {
                // How about grouping by material?
                let material = match model.materials.get(primitive.material) {
                    Some(m) => m,
                    None => &self.res.fallback.white_material,
                };

                if let Some(sets) = self
                    .res
                    .descriptors
                    .material_sets
                    .get(&(pipeline.set_layouts[2], primitive.material))
                {
                    // If there is a descriptor set, there must be a uniform buffer
                    let ubo = self
                        .res
                        .material_buffers
                        .get_mut(&primitive.material)
                        .unwrap();
                    ubo.upload(material);

                    // @todo Use a constant or something that is not a magic number (2)
                    self.res
                        .command_buffer
                        .bind_descriptor_sets(pipeline, sets, 2);
                } else {
                    // Check if material uniform buffer already exists
                    let material_buffer =
                        match self.res.material_buffers.get_mut(&primitive.material) {
                            Some(buffer) => buffer,
                            None => {
                                // Create a new uniform buffer for this material
                                let material_buffer = Buffer::new::<Color>(
                                    &self.allocator,
                                    vk::BufferUsageFlags::UNIFORM_BUFFER,
                                );

                                self.res
                                    .material_buffers
                                    .insert(primitive.material, material_buffer);

                                self.res
                                    .material_buffers
                                    .get_mut(&primitive.material)
                                    .unwrap()
                            }
                        };

                    material_buffer.upload(material);

                    let (albedo_view, albedo_sampler) = match model.textures.get(material.albedo) {
                        Some(texture) => {
                            let view = model.views.get(texture.view).unwrap();
                            let sampler = model.samplers.get(texture.sampler).unwrap();
                            (view, sampler)
                        }
                        _ => (
                            // Bind a default white albedo
                            &self.res.fallback.white_view,
                            &self.res.fallback.white_sampler,
                        ),
                    };

                    let sets = self.res.descriptors.allocate(&[pipeline.set_layouts[2]]); // 1 is for material
                    Material::write_set(
                        &self.device,
                        sets[0],
                        &material_buffer,
                        albedo_view,
                        albedo_sampler,
                    );

                    self.res
                        .command_buffer
                        .bind_descriptor_sets(pipeline, &sets, 2);

                    self.res
                        .descriptors
                        .material_sets
                        .insert((pipeline.set_layouts[2], primitive.material), sets);
                }
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

    pub fn recreate(&mut self, win: &Win, surface: &Surface, dev: &Dev, pass: &Pass) {
        dev.wait();
        self.current = 0;
        let (width, height) = win.window.drawable_size();
        self.swapchain.recreate(&surface, &dev, width, height);
        for i in 0..self.swapchain.images.len() {
            let frame = self.frames[i].as_mut().unwrap();
            frame.res.descriptors.free(&frame.res.descriptors.present_sets);
            frame.res.descriptors.present_sets.clear();
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
