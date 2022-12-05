// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use vkr::{
    ash::vk,
    sdl2::{event::Event, keyboard::Keycode},
    Buffer, Color, Dev, Frames, Line, LinePipeline, MainPipeline, Pass, Point3, Surface,
    SwapchainFrames, Vec3, Vertex, Vkr, Win,
};

pub fn main() {
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
    let mut line_buffer = Buffer::new(&dev.allocator);
    line_buffer.upload_arr(&lines);

    let triangle_pipeline = MainPipeline::new(&mut dev, &pass, width, height);

    let mut buffer = Buffer::new(&dev.allocator);
    let vertices = [
        Vertex::new(-0.2, -0.2, 0.0),
        Vertex::new(0.2, -0.2, 0.0),
        Vertex::new(-0.2, 0.2, 0.0),
        Vertex::new(0.2, -0.2, 0.0),
        Vertex::new(0.2, 0.2, 0.0),
        Vertex::new(-0.2, 0.2, 0.0),
    ];
    buffer.upload_arr(&vertices);

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

            let frame = match sfs.next_frame() {
                Ok(frame) => frame,
                // Recreate swapchain
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    drop(sfs);
                    let (width, height) = win.window.size();
                    println!("Recreating swapchain ({}x{})", width, height);
                    sfs = SwapchainFrames::new(&vkr.ctx, &surface, &mut dev, width, height, &pass);
                    continue 'running;
                }
                Err(result) => panic!("{:?}", result),
            };

            frame.begin(&pass);
            frame.draw(&triangle_pipeline, &buffer);
            frame.draw(&line_pipeline, &line_buffer);
            frame.end();

            match sfs.present(&dev) {
                // Recreate swapchain
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    drop(sfs);
                    let (width, height) = win.window.size();
                    println!("Recreating swapchain ({}x{})", width, height);
                    sfs = SwapchainFrames::new(&vkr.ctx, &surface, &mut dev, width, height, &pass);
                    continue 'running;
                }
                Err(result) => panic!("{:?}", result),
                _ => (),
            }
        }
    }

    dev.wait();
}
