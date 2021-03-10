// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use nalgebra as na;
use sdl::{event::Event, keyboard::Keycode};
use sdl2 as sdl;

mod util;
use util::*;

mod model;
use model::*;

mod pipeline;
use pipeline::*;

mod gfx;
use gfx::*;

mod descriptor;
use descriptor::*;

mod frame;
use frame::*;

pub fn main() {
    let mut timer = Timer::new();

    let win = Win::new();
    let (width, height) = win.window.size();

    let vkr = Vkr::new(&win);

    let surface = Surface::new(&win, &vkr.ctx);
    let mut dev = Dev::new(&vkr.ctx, &surface);

    let pass = Pass::new(&mut dev);

    let mut sfs = SwapchainFrames::new(&vkr.ctx, &surface, &mut dev, width, height, &pass);

    let mut line_pipeline = Pipeline::new::<Line>(
        &dev.device,
        ash::vk::PrimitiveTopology::LINE_LIST,
        &pass,
        width,
        height,
    );

    let lines = vec![
        // Notice how this line appears at the top of the picture as Vulkan Y axis is pointing downwards
        Line::new(
            Point::new(Vec3f::new(-0.3, -0.3, 0.0), Color::new(1.0, 1.0, 0.0, 1.0)),
            Point::new(Vec3f::new(0.3, -0.3, 0.0), Color::new(1.0, 1.0, 0.0, 1.0)),
        ),
        Line::new(
            Point::new(Vec3f::new(0.3, -0.3, 0.0), Color::new(1.0, 0.5, 0.0, 1.0)),
            Point::new(Vec3f::new(0.3, 0.3, 0.0), Color::new(1.0, 0.5, 0.0, 1.0)),
        ),
        Line::new(
            Point::new(Vec3f::new(0.3, 0.3, 0.0), Color::new(1.0, 0.1, 0.0, 1.0)),
            Point::new(Vec3f::new(-0.3, 0.3, 0.0), Color::new(1.0, 0.1, 0.0, 1.0)),
        ),
        Line::new(
            Point::new(Vec3f::new(-0.3, 0.3, 0.0), Color::new(1.0, 0.0, 0.3, 1.0)),
            Point::new(Vec3f::new(-0.3, -0.3, 0.0), Color::new(1.0, 0.0, 0.3, 1.0)),
        ),
    ];

    let mut line_buffer =
        Buffer::new::<Vertex>(&dev.allocator, ash::vk::BufferUsageFlags::VERTEX_BUFFER);
    line_buffer.upload_arr(&lines);

    let mut triangle_pipeline = Pipeline::new::<Vertex>(
        &dev.device,
        ash::vk::PrimitiveTopology::TRIANGLE_LIST,
        &pass,
        width,
        height,
    );

    let mut buffer =
        Buffer::new::<Vertex>(&dev.allocator, ash::vk::BufferUsageFlags::VERTEX_BUFFER);
    let vertices = vec![
        Vertex::new(-0.2, -0.2, 0.0),
        Vertex::new(0.2, -0.2, 0.0),
        Vertex::new(-0.2, 0.2, 0.0),
        Vertex::new(0.2, -0.2, 0.0),
        Vertex::new(0.2, 0.2, 0.0),
        Vertex::new(-0.2, 0.2, 0.0),
    ];
    buffer.upload_arr(&vertices);

    let mut node = Node::new();

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
        }

        let delta = timer.get_delta().as_secs_f32();
        let rot = na::UnitQuaternion::from_axis_angle(&na::Vector3::z_axis(), delta / 2.0);
        node.trs.rotate(&rot);

        let frame = match sfs.next_frame() {
            Ok(frame) => frame,
            // Recreate swapchain
            Err(ash::vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                drop(sfs.swapchain);
                let (width, height) = win.window.size();
                sfs.swapchain = Swapchain::new(&vkr.ctx, &surface, &dev, width, height);
                for i in 0..sfs.swapchain.images.len() {
                    let frame = &mut sfs.frames[i];
                    frame.buffer =
                        Framebuffer::new(&mut dev, sfs.swapchain.images[i].clone(), &pass);
                }
                continue 'running;
            }
            Err(result) => panic!("{:?}", result),
        };

        frame.begin(&pass);
        frame.ubo.upload(&node.trs.get_matrix());
        frame.draw(&mut triangle_pipeline, &buffer);
        frame.draw(&mut line_pipeline, &line_buffer);
        frame.end();

        match sfs.present(&dev) {
            // Recreate swapchain
            Err(ash::vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                drop(sfs.swapchain);
                let (width, height) = win.window.size();
                sfs.swapchain = Swapchain::new(&vkr.ctx, &surface, &dev, width, height);
                for i in 0..sfs.swapchain.images.len() {
                    let frame = &mut sfs.frames[i];
                    frame.buffer =
                        Framebuffer::new(&mut dev, sfs.swapchain.images[i].clone(), &pass);
                }
                continue 'running;
            }
            Err(result) => panic!("{:?}", result),
            _ => (),
        }
    }

    // Make sure device is idle before releasing Vulkan resources
    dev.wait();
}
