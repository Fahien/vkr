// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use vkr::{
    ash::vk,
    sdl2::{event::Event, keyboard::Keycode},
    Buffer, Color, Dev, Framebuffer, Frames, Line, LinePipeline, MainPipeline, Node, Pack, Pass,
    Point3, Quat, Surface, Swapchain, SwapchainFrames, Timer, Vec3, Vertex, Vkr, Win,
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

    let lines = vec![
        // Notice how this line appears at the top of the picture as Vulkan Y axis is pointing downwards
        Line::new(
            Point3::new(Vec3::new(-0.3, -0.3, 0.0), Color::new(1.0, 1.0, 0.0, 1.0)),
            Point3::new(Vec3::new(0.3, -0.3, 0.0), Color::new(1.0, 1.0, 0.0, 1.0)),
        ),
        Line::new(
            Point3::new(Vec3::new(0.3, -0.3, 0.0), Color::new(1.0, 0.5, 0.0, 1.0)),
            Point3::new(Vec3::new(0.3, 0.3, 0.0), Color::new(1.0, 0.5, 0.0, 1.0)),
        ),
        Line::new(
            Point3::new(Vec3::new(0.3, 0.3, 0.0), Color::new(1.0, 0.1, 0.0, 1.0)),
            Point3::new(Vec3::new(-0.3, 0.3, 0.0), Color::new(1.0, 0.1, 0.0, 1.0)),
        ),
        Line::new(
            Point3::new(Vec3::new(-0.3, 0.3, 0.0), Color::new(1.0, 0.0, 0.3, 1.0)),
            Point3::new(Vec3::new(-0.3, -0.3, 0.0), Color::new(1.0, 0.0, 0.3, 1.0)),
        ),
    ];
    let mut lines_vertex_buffer =
        Buffer::new::<Vertex>(&dev.allocator, vk::BufferUsageFlags::VERTEX_BUFFER);
    lines_vertex_buffer.upload_arr(&lines);

    let triangle_pipeline = MainPipeline::new(&mut dev, &pass, width, height);

    let mut rect_vertex_buffer =
        Buffer::new::<Vertex>(&dev.allocator, vk::BufferUsageFlags::VERTEX_BUFFER);
    let vertices = [
        Vertex::new(-0.2, -0.2, 0.0),
        Vertex::new(0.2, -0.2, 0.0),
        Vertex::new(-0.2, 0.2, 0.0),
        Vertex::new(0.2, -0.2, 0.0),
        Vertex::new(0.2, 0.2, 0.0),
        Vertex::new(-0.2, 0.2, 0.0),
    ];
    rect_vertex_buffer.upload_arr(&vertices);

    let mut nodes = Pack::new();
    let rect = nodes.push(Node::new());
    let lines = nodes.push(Node::new());

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

        let rot = Quat::axis_angle(Vec3::new(0.0, 0.0, 1.0), delta / 2.0);
        nodes.get_mut(rect).unwrap().trs.rotate(&rot);

        let rot = Quat::axis_angle(Vec3::new(0.0, 0.0, 1.0), -delta / 2.0);
        nodes.get_mut(lines).unwrap().trs.rotate(&rot);

        let frame = match sfs.next_frame() {
            Ok(frame) => frame,
            // Recreate swapchain
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                drop(sfs.swapchain);
                let (width, height) = win.window.size();
                sfs.swapchain = Swapchain::new(&vkr.ctx, &surface, &dev, width, height);
                for i in 0..sfs.swapchain.images.len() {
                    let frame = &mut sfs.frames[i];
                    frame.buffer = Framebuffer::new(&mut dev, &sfs.swapchain.images[i], &pass);
                }
                continue 'running;
            }
            Err(result) => panic!("{:?}", result),
        };

        frame.begin(&pass);
        frame.draw(&triangle_pipeline, &nodes, &rect_vertex_buffer, rect);
        frame.draw(&line_pipeline, &nodes, &lines_vertex_buffer, lines);
        frame.end();

        match sfs.present(&dev) {
            // Recreate swapchain
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                drop(sfs.swapchain);
                let (width, height) = win.window.size();
                sfs.swapchain = Swapchain::new(&vkr.ctx, &surface, &dev, width, height);
                for i in 0..sfs.swapchain.images.len() {
                    let frame = &mut sfs.frames[i];
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
