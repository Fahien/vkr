// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use sdl::{event::Event, keyboard::Keycode};
use sdl2 as sdl;

mod pipeline;
use pipeline::*;

mod gfx;
use gfx::*;

mod frame;
use frame::*;

pub fn main() {
    let win = Win::new();
    let (width, height) = win.window.size();

    let vkr = Vkr::new(&win);

    let surface = Surface::new(&win, &vkr.ctx);
    let mut dev = Dev::new(&vkr.ctx, &surface);

    let swapchain = Swapchain::new(&vkr.ctx, &surface, &dev, width, height);

    let pass = Pass::new(&mut dev);

    // Frames: collection of per-frame resources (device, swapchain, renderpass, command pool)
    let mut frames = Vec::new();
    for image in swapchain.images.iter() {
        frames.push(Frame::new(&mut dev, &image, &pass));
    }

    let pipeline = Pipeline::new(&dev.device, &pass, width, height);

    let mut buffer = Buffer::new(&vkr.ctx, &mut dev);
    let vertices = [
        Vertex::new(-0.2, -0.2, 0.0),
        Vertex::new(0.2, -0.2, 0.0),
        Vertex::new(0.0, 0.2, 0.0),
    ];
    buffer.upload(vertices.as_ptr(), buffer.size as usize);

    let mut current_frame = 0;
    let mut events = win.ctx.event_pump().expect("Failed to create SDL events");
    'running: loop {
        // Handle events
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }

            // Wait for this frame to be ready
            let frame = &frames[current_frame];
            frame.wait();

            // Get next image
            let (image_index, _) = unsafe {
                swapchain.ext.acquire_next_image(
                    swapchain.swapchain,
                    u64::max_value(),
                    frame.image_ready,
                    ash::vk::Fence::null(),
                )
            }
            .expect("Failed to acquire Vulkan next image");

            frame.begin(&pass);
            frame.draw(&pipeline, &buffer);
            frame.end();
            frame.present(&dev, &swapchain, image_index);

            // Update current frame
            current_frame = (current_frame + 1) % swapchain.images.len();
        }
    }
}
