// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::{version::DeviceV1_0, *};
use std::{borrow::Borrow, cell::RefCell, collections::HashMap, rc::Rc};

use super::*;
use imgui as im;

pub struct ShadowFramebuffer {
    pub framebuffer: vk::Framebuffer,
    pub view: ImageView,
    pub image: Image,
}

impl ShadowFramebuffer {
    pub fn new(dev: &Dev, extent: vk::Extent2D, pass: &Pass) -> Self {
        // Shadow image
        let shadow_format = vk::Format::D32_SFLOAT;
        let mut image = Image::new(
            &dev.allocator,
            extent.width,
            extent.height,
            shadow_format,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
        );
        image.transition(&dev, vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

        let view = ImageView::new(&dev.device, &image);

        // Framebuffers (image_views, renderpass)
        let framebuffer = {
            let attachments = [view.view];

            let create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(pass.render)
                .attachments(&attachments)
                .width(extent.width)
                .height(extent.height)
                .layers(1)
                .build();

            unsafe { dev.device.create_framebuffer(&create_info, None) }
                .expect("Failed to create Vulkan framebuffer")
        };

        Self {
            framebuffer,
            view,
            image,
        }
    }
}

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
    white_image: Image,
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
            PresentVertex::new(-1.0, -1.0, 0.0, 0.0),
            PresentVertex::new(-1.0, 3.0, 0.0, 2.0),
            PresentVertex::new(3.0, -1.0, 2.0, 0.0),
        ];
        let mut present_buffer =
            Buffer::new::<PresentVertex>(&dev.allocator, vk::BufferUsageFlags::VERTEX_BUFFER);
        present_buffer.upload_arr(&present_vertices);

        Self {
            white_image,
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

    /// Uniform buffers for normal matrices (node's inverse transpose) associated to nodes
    pub normal_buffers: BufferCache<Node>,

    /// Uniform buffers for view matrices associated to nodes with cameras
    pub view_buffers: BufferCache<Node>,

    // Uniform buffers for proj matrices associated to cameras
    pub proj_buffers: BufferCache<Camera>,

    // Uniform buffers for materials
    pub material_buffers: BufferCache<Material>,

    pub light_constants: Vec<u8>,

    pub descriptors: Descriptors,
    pub command_buffer: CommandBuffer,

    pub fence: Fence,

    // Image ready semaphores are free and unlinked from a frame as we do not know which one
    // is going to be acquired, thus can not wait for a specific fence and can not signal a
    // specific image ready semaphore.
    pub image_ready: vk::Semaphore,

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
            normal_buffers: BufferCache::new(),
            view_buffers: BufferCache::new(),
            proj_buffers: BufferCache::new(),
            material_buffers: BufferCache::new(),
            light_constants: vec![],
            descriptors: Descriptors::new(dev),
            command_buffer,
            fence,
            image_ready: vk::Semaphore::null(),
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
    /// Used to compute the model-view matrix when rendering a mesh
    pub current_view: na::Matrix4<f32>,
    pub shadow_buffer: ShadowFramebuffer,
    pub buffer: Framebuffer,
    pub res: Frameres,
    /// A frame should be able to allocate a uniform buffer on draw
    allocator: Rc<RefCell<vk_mem::Allocator>>,
    pub device: Rc<Device>,
}

impl Frame {
    pub fn new(dev: &mut Dev, image: &Image, pass: &Pass, shadow_pass: &Pass) -> Self {
        let shadow_extent = vk::Extent2D::builder().width(512).height(512).build();
        let shadow_buffer = ShadowFramebuffer::new(dev, shadow_extent, shadow_pass);
        let buffer = Framebuffer::new(dev, image, pass);
        let res = Frameres::new(dev);

        Frame {
            current_view: na::Matrix4::identity(),
            shadow_buffer,
            buffer,
            res,
            allocator: dev.allocator.clone(),
            device: Rc::clone(&dev.device),
        }
    }

    pub fn begin_shadow(&self, pass: &Pass) {
        self.res
            .command_buffer
            .begin(vk::CommandBufferUsageFlags::default());

        let width = self.shadow_buffer.image.extent.width;
        let height = self.shadow_buffer.image.extent.height;

        // Needed by cmd_begin_render_pass
        let area = vk::Rect2D::builder()
            .offset(vk::Offset2D::builder().x(0).y(0).build())
            .extent(vk::Extent2D::builder().width(width).height(height).build())
            .build();

        self.res
            .command_buffer
            .begin_render_shadow_pass(pass, &self.shadow_buffer, area);

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

    pub fn begin(&self, pass: &Pass, width: u32, height: u32) {
        //self.res
        //    .command_buffer
        //    .begin(vk::CommandBufferUsageFlags::default());

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

    pub fn bind_light(&mut self, pipeline: &Pipeline, model: &Model, light_node: Handle<Node>) {
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

        let node = model.nodes.get(light_node).unwrap();
        self.current_view = node.trs.get_view_matrix();
        // Lights should have cameras for rendering shadowmaps
        let camera = model.cameras.get(node.camera).unwrap();

        if let Some(sets) = self
            .res
            .descriptors
            .view_sets
            .get(&(pipeline.set_layouts[1], light_node))
        {
            self.res
                .command_buffer
                .bind_descriptor_sets(pipeline, sets, 1);

            // If there is a descriptor set, there must be a buffer
            let view_buffer = self.res.view_buffers.get_mut(&light_node).unwrap();
            view_buffer.upload(&self.current_view);

            let proj_buffer = self.res.proj_buffers.get_mut(&node.camera).unwrap();
            proj_buffer.upload(&camera.proj);
        } else {
            // Allocate and write desc set for camera view
            // Camera set layout is at index 1 (use a constant?)
            let sets = self.res.descriptors.allocate(&[pipeline.set_layouts[1]]);

            if let Some(view_buffer) = self.res.view_buffers.get_mut(&light_node) {
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
                self.res.view_buffers.insert(light_node, view_buffer);
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
                .insert((pipeline.set_layouts[1], light_node), sets);
        }
    }

    pub fn draw<T: VertexInput>(&mut self, pipeline: &Pipeline, model: &Model, node: Handle<Node>) {
        self.res.command_buffer.bind_pipeline(pipeline);

        let children = model.nodes.get(node).unwrap().children.clone();
        for child in children {
            self.draw::<T>(pipeline, model, child);
        }

        let cnode = model.nodes.get(node).unwrap();

        // Check whether it has a light source
        if let Some(light) = model.lights.get(cnode.light) {
            let light_direction = -cnode.trs.get_forward();
            // For the moment we expect one light direction, therefore push light constant
            let light_constants = unsafe {
                std::slice::from_raw_parts(
                    light_direction.as_ptr() as *const u8,
                    std::mem::size_of::<na::Vector3<f32>>(),
                )
            }
            .to_vec();

            self.res.command_buffer.push_constants(
                pipeline,
                vk::ShaderStageFlags::FRAGMENT,
                0,
                &light_constants,
            );
        }

        let mesh = model.meshes.get(cnode.mesh);
        if mesh.is_none() {
            return ();
        }
        let mesh = mesh.unwrap();

        let normal_matrix = cnode.trs.get_matrix().try_inverse().unwrap().transpose();

        if let Some(sets) = self
            .res
            .descriptors
            .model_sets
            .get(&(pipeline.set_layouts[0], node))
        {
            // If there is a descriptor set, there must be a uniform buffer
            let ubo = self.res.model_buffers.get_mut(&node).unwrap();
            ubo.upload(&cnode.trs.get_matrix());

            let normal_buffer = self.res.normal_buffers.get_mut(&node).unwrap();
            normal_buffer.upload(&normal_matrix);

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
            let normal_buffer = match self.res.normal_buffers.get_mut(&node) {
                Some(b) => b,
                None => {
                    // Create a new uniform buffer for this node's model view matrix
                    let buffer = Buffer::new::<na::Matrix4<f32>>(
                        &self.allocator,
                        vk::BufferUsageFlags::UNIFORM_BUFFER,
                    );
                    self.res.normal_buffers.insert(node, buffer);
                    self.res.normal_buffers.get_mut(&node).unwrap()
                }
            };

            // Allocate and write descriptors
            let sets = self.res.descriptors.allocate(&[pipeline.set_layouts[0]]);
            T::write_set_model(self.device.borrow(), sets[0], &model_buffer);
            T::write_set_normal_matrix(self.device.borrow(), sets[0], &normal_buffer);

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
        if self.res.image_ready == vk::Semaphore::null() {
            // Something went wrong, just skip this frame
            println!("No image ready?");
            return Ok(());
        }

        self.res.image_drawn = Semaphore::new(&dev.device);

        dev.graphics_queue.submit_draw(
            &self.res.command_buffer,
            self.res.image_ready,
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
    // TODO why Pass is needed here?
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
    pub image_ready_semaphores: Vec<Semaphore>,
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
        shadow_pass: &Pass,
        pass: &Pass,
    ) -> Self {
        let swapchain = Swapchain::new(ctx, surface, dev, width, height);

        let mut frames = Vec::new();
        for image in swapchain.images.iter() {
            let frame = Frame::new(dev, image, pass, shadow_pass);
            frames.push(Some(frame));
        }

        Self {
            current: 0,
            image_index: 0,
            image_ready_semaphores: vec![],
            frames: frames,
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
            // Reset image ready semaphore handle for this frame
            // The image drawn one is still in use at the moment
            frame.res.image_ready = vk::Semaphore::null();
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
        // Image ready semaphores are not associated to single frames as we do not know which
        // image index is going to be available on acquiring next image.
        if self.image_ready_semaphores.len() >= self.frames.len() {
            self.image_ready_semaphores.pop();
        }
        let image_ready = Semaphore::new(&dev.device);
        let image_ready_handle = image_ready.semaphore;
        self.image_ready_semaphores.insert(0, image_ready);

        let acquire_res = unsafe {
            self.swapchain.ext.acquire_next_image(
                self.swapchain.swapchain,
                64,
                image_ready_handle,
                vk::Fence::null(),
            )
        };

        match acquire_res {
            Ok((image_index, false)) => {
                self.image_index = image_index;
                let mut frame = self.frames[image_index as usize].take().unwrap();
                // Wait for this frame to complete previous commands.
                frame.res.wait();
                // We still need to wait for the image to be ready before drawing onto it
                // therefore we store the semaphore in this frame to be used later.
                frame.res.image_ready = image_ready_handle;
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
        assert!(self.frames[self.image_index as usize].is_none());
        self.frames[self.image_index as usize].replace(frame);

        match self.frames[self.image_index as usize]
            .as_mut()
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
