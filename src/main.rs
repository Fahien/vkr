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

mod queue;
use queue::*;

mod shader;
use shader::*;

mod sampler;
use sampler::*;

mod gfx;
use gfx::*;

mod descriptor;
use descriptor::*;

mod primitive;
use primitive::*;

mod sync;
use sync::*;

mod frame;
use frame::*;

pub fn main() {
    let mut timer = Timer::new();

    let win = Win::new();
    let (width, height) = win.window.drawable_size();

    let vkr = Vkr::new(&win);

    let surface = Surface::new(&win, &vkr.ctx);
    let mut dev = Dev::new(&vkr.ctx, &surface);

    let pass = Pass::new(&mut dev);

    let mut sfs = SwapchainFrames::new(&vkr.ctx, &surface, &mut dev, width, height, &pass);

    let line_pipeline = Pipeline::line(&mut dev, &pass, width, height);

    let lines_primitive = {
        // Notice how the first line appears at the top of the picture as Vulkan Y axis is pointing downwards
        let lines_vertices = vec![
            Point::new(
                na::Vector3::new(-0.5, -0.5, 0.0),
                Color::new(1.0, 1.0, 0.0, 1.0),
            ),
            Point::new(
                na::Vector3::new(0.5, -0.5, 0.0),
                Color::new(0.2, 1.0, 1.0, 1.0),
            ),
            Point::new(
                na::Vector3::new(0.5, 0.5, 0.0),
                Color::new(0.1, 1.0, 0.0, 1.0),
            ),
            Point::new(
                na::Vector3::new(-0.5, 0.5, 0.0),
                Color::new(1.0, 0.1, 1.0, 1.0),
            ),
            Point::new(
                na::Vector3::new(-0.5, -0.5, 0.0),
                Color::new(1.0, 1.0, 0.0, 1.0),
            ),
        ];
        Primitive::new(&dev.allocator, &lines_vertices)
    };

    let triangle_pipeline = Pipeline::main(&mut dev, &pass, width, height);

    let rect_primitive = Primitive::quad(&dev.allocator);

    let mut model = Model::new();

    let mut rect = Node::new();
    rect.trs.translate(&na::Vector3::new(0.0, 0.0, 0.6));
    let rect = model.nodes.push(rect);

    let mut lines = Node::new();
    lines.trs.scale(&na::Vector3::new(0.5, 0.5, 0.5));
    let lines = model.nodes.push(lines);

    let mut events = win.ctx.event_pump().expect("Failed to create SDL events");

    let image = Image::load(&dev, "res/image/test.png");

    let view = ImageView::new(&dev.device, &image);

    model.images.push(image);

    let view = model.views.push(view);

    let sampler = model.samplers.push(Sampler::new(&dev.device));

    let texture = Texture::new(view, sampler);
    let texture = model.textures.push(texture);

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
        model.nodes.get_mut(rect).unwrap().trs.rotate(&rot);
        let rot = na::UnitQuaternion::from_axis_angle(&na::Vector3::z_axis(), -delta / 2.0);
        model.nodes.get_mut(lines).unwrap().trs.rotate(&rot);

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
                frame.buffer = Framebuffer::new(&mut dev, &sfs.swapchain.images[i], &pass);
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
                frame.buffer = Framebuffer::new(&mut dev, &sfs.swapchain.images[i], &pass);
            }

            continue 'running;
        };

        let frame = frame.unwrap();

        let (width, height) = win.window.drawable_size();
        frame.begin(&pass, width, height);
        frame.draw::<Line>(
            &line_pipeline,
            &model,
            &lines_primitive,
            lines,
            Handle::none(),
        );
        frame.draw::<Vertex>(&triangle_pipeline, &model, &rect_primitive, rect, texture);
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
                    frame.buffer = Framebuffer::new(&mut dev, &sfs.swapchain.images[i], &pass);
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
