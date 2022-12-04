// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

pub struct Win {
    pub window: sdl2::video::Window,
    pub video: sdl2::VideoSubsystem,
    pub ctx: sdl2::Sdl,
}

impl Win {
    pub fn new() -> Self {
        let ctx = sdl2::init().expect("Failed to initialize SDL");
        let video = ctx.video().expect("Failed to initialize SDL video");
        let window = video
            .window("Test", 480, 480)
            .vulkan()
            .position_centered()
            .build()
            .expect("Failed to build SDL window");

        Self { window, video, ctx }
    }
}

impl Default for Win {
    fn default() -> Self {
        Self::new()
    }
}
