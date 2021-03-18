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

mod image;
use image::*;

mod frame;
use frame::*;

mod sync;
use sync::*;

mod queue;
use queue::*;

mod gfx;
use gfx::*;

pub fn main() {
    let mut timer = Timer::new();

    let win = Win::new();
    let (width, height) = win.window.drawable_size();

    let vkr = Vkr::new(&win);

    let surface = Surface::new(&win, &vkr.ctx);
    let mut dev = Dev::new(&vkr.ctx, &surface);

    let pass = Pass::new(&mut dev);

    let mut sfs = SwapchainFrames::new(&vkr.ctx, &surface, &mut dev, width, height, &pass);

    let line_pipeline = Pipeline::new::<Line>(
        &mut dev,
        ash::vk::PrimitiveTopology::LINE_STRIP,
        &pass,
        width,
        height,
    );

    let lines_primitive = {
        // Notice how the first line appears at the top of the picture as Vulkan Y axis is pointing downwards
        let lines_vertices = vec![
            Point::new(Vec3f::new(-0.3, -0.3, 0.0), Color::new(1.0, 1.0, 0.0, 1.0)),
            Point::new(Vec3f::new(0.3, -0.3, 0.0), Color::new(1.0, 1.0, 0.0, 1.0)),
            Point::new(Vec3f::new(0.3, 0.3, 0.0), Color::new(1.0, 0.5, 0.0, 1.0)),
            Point::new(Vec3f::new(-0.3, 0.3, 0.0), Color::new(1.0, 0.1, 0.0, 1.0)),
            Point::new(Vec3f::new(-0.3, -0.3, 0.0), Color::new(1.0, 0.0, 0.3, 1.0)),
        ];
        Primitive::new(&dev.allocator, &lines_vertices)
    };

    let triangle_pipeline = Pipeline::new::<Vertex>(
        &mut dev,
        ash::vk::PrimitiveTopology::TRIANGLE_LIST,
        &pass,
        width,
        height,
    );

    let rect_primitive = {
        let vertices = vec![
            Vertex::new(-0.2, -0.2, 0.0),
            Vertex::new(0.2, -0.2, 0.0),
            Vertex::new(-0.2, 0.2, 0.0),
            Vertex::new(0.2, 0.2, 0.0),
        ];
        let mut primitive = Primitive::new(&dev.allocator, &vertices);
        let indices = vec![0, 1, 2, 1, 3, 2];
        primitive.set_indices(&indices);
        primitive
    };

    let mut nodes = Pack::new();
    let rect = nodes.push(Node::new());
    let lines = nodes.push(Node::new());

    let mut events = win.ctx.event_pump().expect("Failed to create SDL events");

    // @todo Remove testing image upload
    let staging = Buffer::staging(&dev.allocator, "res/image/test.png");
    let mut image = Image::new(&dev.allocator, 2, 2, ash::vk::Format::R8G8B8A8_UNORM);
    image.copy_from(&staging, &dev);

    'running: loop {
        let mut resized = false;

        // Handle events
        for event in events.poll_iter() {
            match event {
                Event::Window {
                    win_event: sdl::event::WindowEvent::Resized(_, _),
                    ..
                }
                | Event::Window {
                    win_event: sdl::event::WindowEvent::SizeChanged(_, _),
                    ..
                } => {
                    resized = true;
                }
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
        nodes.get_mut(rect).unwrap().trs.rotate(&rot);
        let rot = na::UnitQuaternion::from_axis_angle(&na::Vector3::z_axis(), -delta / 2.0);
        nodes.get_mut(lines).unwrap().trs.rotate(&rot);

        if resized {
            dev.wait();
            drop(sfs.swapchain);
            // Current must be reset to avoid LAYOUT_UNDEFINED validation errors
            sfs.current = 0;
            let (width, height) = win.window.drawable_size();
            sfs.swapchain = Swapchain::new(&vkr.ctx, &surface, &dev, width, height);
            for i in 0..sfs.swapchain.images.len() {
                let frame = &mut sfs.frames[i];
                // Only this semaphore must be recreated to avoid validation errors
                // The image drawn one is still in use at the moment
                frame.res.image_ready = Semaphore::new(&dev.device);
                frame.buffer = Framebuffer::new(&mut dev, sfs.swapchain.images[i].clone(), &pass);
            }
        }

        let frame = sfs.next_frame();

        if frame.is_err() {
            let result = frame.err().unwrap();
            if result != ash::vk::Result::ERROR_OUT_OF_DATE_KHR {
                panic!("{:?}", result);
            }

            dev.wait();
            drop(sfs.swapchain);
            let (width, height) = win.window.drawable_size();
            sfs.swapchain = Swapchain::new(&vkr.ctx, &surface, &dev, width, height);
            for i in 0..sfs.swapchain.images.len() {
                let frame = &mut sfs.frames[i];
                // Only this semaphore must be recreated to avoid validation errors
                // The image drawn one is still in use at the moment
                frame.res.image_ready = Semaphore::new(&dev.device);
                frame.buffer = Framebuffer::new(&mut dev, sfs.swapchain.images[i].clone(), &pass);
            }

            continue 'running;
        };

        let frame = frame.unwrap();

        let (width, height) = win.window.drawable_size();
        frame.begin(&pass, width, height);
        frame.draw(&triangle_pipeline, &nodes, &rect_primitive, rect);
        frame.draw(&line_pipeline, &nodes, &lines_primitive, lines);
        frame.end();

        match sfs.present(&dev) {
            // Recreate swapchain
            Err(ash::vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                dev.wait();
                drop(sfs.swapchain);
                let (width, height) = win.window.drawable_size();
                sfs.swapchain = Swapchain::new(&vkr.ctx, &surface, &dev, width, height);
                for i in 0..sfs.swapchain.images.len() {
                    let frame = &mut sfs.frames[i];
                    // Semaphores must be recreated to avoid validation errors
                    frame.res.image_ready = Semaphore::new(&dev.device);
                    frame.res.image_drawn = Semaphore::new(&dev.device);
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
