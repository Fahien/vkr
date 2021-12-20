// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

#![cfg_attr(
    target_arch = "spirv",
    feature(register_attr),
    register_attr(spirv),
    no_std
)]
// HACK(eddyb) can't easily see warnings otherwise from `spirv-builder` builds.
#![deny(warnings)]

use glam::UVec3;
use spirv_std::glam;
#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

// LocalSize/numthreads of (x = 32, y = 32, z = 1)
// The idea is to create a 32x32 image
#[spirv(compute(threads(32, 32)))]
pub fn main_cs(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] out: &mut [[u32; 32]; 32],
) {
    let x = id.x as usize;
    let y = id.y as usize;

    // Draw the diagonal
    out[x][y] = (x == y) as u32;
}
