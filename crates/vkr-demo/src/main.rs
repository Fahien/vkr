// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use vkr::{
    ash::vk,
    sdl2::{
        event::{Event, WindowEvent},
        keyboard::Keycode,
    },
    Color, Dev, Framebuffer, Frames, LinePipeline, MainPipeline, Node, Pack, Pass, Point3,
    Primitive, Quat, Semaphore, Surface, Swapchain, SwapchainFrames, Timer, Vec3, Vertex, Vkr, Win,
};

pub fn main() {
    let mut timer = Timer::new();

    let win = Win::new();
    let (width, height) = win.window.drawable_size();

    let vkr = Vkr::new(&win);

    let surface = Surface::new(&win, &vkr.ctx);
    let mut dev = Dev::new(&vkr.ctx, &surface);

    let pass = Pass::new(&mut dev);

    let mut sfs = SwapchainFrames::new(&vkr.ctx, &surface, &mut dev, width, height, &pass);

    let line_pipeline = LinePipeline::new(&mut dev, &pass, width, height);

    let lines_primitive = {
        // Notice how the first line appears at the top of the picture as Vulkan Y axis is pointing downwards
        let lines_vertices = vec![
            Point3::new(Vec3::new(-0.3, -0.3, 0.0), Color::new(1.0, 1.0, 0.0, 1.0)),
            Point3::new(Vec3::new(0.3, -0.3, 0.0), Color::new(1.0, 1.0, 0.0, 1.0)),
            Point3::new(Vec3::new(0.3, 0.3, 0.0), Color::new(1.0, 0.5, 0.0, 1.0)),
            Point3::new(Vec3::new(-0.3, 0.3, 0.0), Color::new(1.0, 0.1, 0.0, 1.0)),
            Point3::new(Vec3::new(-0.3, -0.3, 0.0), Color::new(1.0, 0.0, 0.3, 1.0)),
        ];
        Primitive::new(&dev.allocator, &lines_vertices)
    };

    let triangle_pipeline = MainPipeline::new(&mut dev, &pass, width, height);

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
    'running: loop {
        let mut resized = false;

        // Handle events
        for event in events.poll_iter() {
            match event {
                Event::Window {
                    win_event: WindowEvent::Resized(_, _),
                    ..
                }
                | Event::Window {
                    win_event: WindowEvent::SizeChanged(_, _),
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

        let rot = Quat::axis_angle(Vec3::new(0.0, 0.0, 1.0), delta / 2.0);
        nodes.get_mut(rect).unwrap().trs.rotate(&rot);

        let rot = Quat::axis_angle(Vec3::new(0.0, 0.0, 1.0), -delta / 2.0);
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
                frame.buffer = Framebuffer::new(&mut dev, &sfs.swapchain.images[i], &pass);
            }
        }

        let frame = sfs.next_frame();

        if frame.is_err() {
            let result = frame.err().unwrap();
            if result != vk::Result::ERROR_OUT_OF_DATE_KHR {
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
        frame.draw(&triangle_pipeline, &nodes, &rect_primitive, rect);
        frame.draw(&line_pipeline, &nodes, &lines_primitive, lines);
        frame.end();

        match sfs.present(&dev) {
            // Recreate swapchain
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                dev.wait();
                drop(sfs.swapchain);
                let (width, height) = win.window.size();
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

    dev.wait();
}
