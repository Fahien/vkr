// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use sdl2 as sdl;

pub struct Win {
    pub events: sdl::EventPump,
    pub window: sdl::video::Window,
    pub video: sdl::VideoSubsystem,
    pub ctx: sdl::Sdl,
}

impl Win {
    pub fn new(name: &str, width: u32, height: u32) -> Self {
        let ctx = sdl::init().expect("Failed to initialize SDL");
        let video = ctx.video().expect("Failed to initialize SDL video");
        let window = video
            .window(name, width, height)
            .allow_highdpi()
            .vulkan()
            .position_centered()
            .resizable()
            .build()
            .expect("Failed to build SDL window");

        let events = ctx.event_pump().expect("Failed to create SDL events");

        Self {
            events,
            window,
            video,
            ctx,
        }
    }
}
