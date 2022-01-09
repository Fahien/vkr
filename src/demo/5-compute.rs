// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::vk;
use vkr::*;

pub fn main() {
    let win = Win::new("Compute", 480, 480);
    let mut vkr = Vkr::new(win);

    let mut storage_buffer =
        Buffer::new::<[[u32; 32]; 32]>(&vkr.dev.allocator,
            vk::BufferUsageFlags::STORAGE_BUFFER);

    let model = Model::new();

    'running: loop {
        if !vkr.handle_events() {
            break 'running;
        }

        //let delta = vkr.timer.get_delta().as_secs_f32();

        let frame = vkr.begin_frame();
        if frame.is_none() {
            continue;
        }

        let mut frame = frame.unwrap();

        // Create storage buffer and sampling texture
        // aliasing the same memory

        // Bind compute pipeline
        frame.bind_compute(vkr.pipelines.get(Pipelines::COMPUTE));

        // Bind output storage buffer (texture)
        // Execute compute workload
        // Bind graphics pipeline
        // Draw waiting for compute to finish before (texture)

        vkr.end_scene(&mut frame);
        vkr.end_frame(frame);
    }

    vkr.dev.wait();
}
