// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

#![feature(portable_simd)]

pub use ash;
pub use sdl2;

pub mod math;
pub use math::*;

pub mod util;
pub use util::*;

pub mod debug;
pub use debug::*;

#[cfg(feature = "win")]
pub mod win;
#[cfg(feature = "win")]
pub use win::*;

pub mod ctx;
pub use ctx::*;

pub mod vertex;
pub use vertex::*;

pub mod surface;
pub use surface::*;

pub mod image;

pub mod dev;
pub use dev::*;

pub mod swapchain;
pub use swapchain::*;

pub mod pass;
pub use pass::*;

pub mod buffer;
pub use buffer::*;

pub mod pipeline;
pub use pipeline::*;

pub mod descriptor;
pub use descriptor::*;

pub mod frame;
pub use frame::*;

pub mod model;
pub use model::*;

pub mod primitive;
pub use primitive::*;
