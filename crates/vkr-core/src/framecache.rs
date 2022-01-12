// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use ash::vk;
use vkr_util::Handle;
use crate::*;

/// Container of fallback resources for a frame such as
/// A white 1x1 pixel texture (image, view, and sampler)
pub struct Fallback {
    pub white_texture: Texture,
    _white_image: Image,
    _white_view: ImageView,

    /// A default sampler
    pub white_sampler: Sampler,
    pub white_material: Material,
    /// A triangle that covers the whole screen
    pub present_buffer: Buffer,
}

impl Fallback {
    fn new(dev: &Dev) -> Self {
        let white = [255, 255, 255, 255];
        let white_image = Image::from_data(&dev, &white, 1, 1, vk::Format::R8G8B8A8_SRGB);

        let white_view = ImageView::new(&dev.device, &white_image);
        let white_sampler = Sampler::new(&dev.device);
        let white_texture = Texture::new(white_view.view, white_sampler.sampler);

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
            white_texture,
            _white_image: white_image,
            _white_view: white_view,
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

        // TODO gui vertex??
        let gui_vertex_buffer =
            Buffer::new::<u128>(&dev.allocator, vk::BufferUsageFlags::VERTEX_BUFFER);
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
