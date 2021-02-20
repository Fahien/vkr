// Copyright © 2020
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT
use sdl::{event::Event, keyboard::Keycode};
use sdl2 as sdl;

fn main() {
    let ctx = sdl::init().expect("Failed to initialize SDL");
    let video = ctx.video().expect("Failed to initialize SDL video");
    let _window = video
        .window("Test", 480, 320)
        .vulkan()
        .position_centered()
        .build()
        .expect("Failed to build SDL window");

    let mut events = ctx.event_pump().expect("Failed to create SDL events");
    'running: loop {
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
    }
}
