// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use sdl::{event::Event, keyboard::Keycode};
use sdl2 as sdl;

mod model;
use model::*;

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

    let pass = Pass::new(&mut dev);

    let mut sfs = SwapchainFrames::new(&vkr.ctx, &surface, &mut dev, width, height, &pass);

    let pipeline = Pipeline::new(&mut dev, &pass, width, height);

    let mut buffer = Buffer::new(&vkr.ctx, &mut dev);
    let vertices = [
        Vertex::new(-0.2, -0.2, 0.0),
        Vertex::new(0.2, -0.2, 0.0),
        Vertex::new(0.0, 0.2, 0.0),
    ];
    buffer.upload(vertices.as_ptr(), buffer.size as usize);

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
                Err(ash::vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    drop(sfs);
                    let (width, height) = win.window.size();
                    sfs = SwapchainFrames::new(&vkr.ctx, &surface, &mut dev, width, height, &pass);
                    continue 'running;
                }
                Err(result) => panic!("{:?}", result),
            };

            frame.begin(&pass);
            frame.draw(&pipeline, &buffer);
            frame.end();

            match sfs.present(&dev) {
                // Recreate swapchain
                Err(ash::vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    drop(sfs);
                    let (width, height) = win.window.size();
                    sfs = SwapchainFrames::new(&vkr.ctx, &surface, &mut dev, width, height, &pass);
                    continue 'running;
                }
                Err(result) => panic!("{:?}", result),
                _ => (),
            }
        }
    }
}
