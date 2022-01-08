// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

pub use nalgebra as na;
#[cfg(feature = "win")]
pub use sdl2 as sdl;

#[cfg(feature = "win")]
pub mod win;
#[cfg(feature = "win")]
pub use win::*;

pub mod ctx;
pub use ctx::*;

pub mod dev;
pub use dev::*;

pub mod buffer;
pub use buffer::*;

pub mod image;
pub use image::*;

pub mod surface;
pub use surface::*;

pub mod swapchain;
pub use swapchain::*;

pub mod commands;
pub use commands::*;

pub mod queue;
pub use queue::*;

pub mod shader;
pub use shader::*;

pub mod sampler;
pub use sampler::*;

pub mod sync;
pub use sync::*;

pub mod pass;
pub use pass::*;

pub mod pipeline;
pub use pipeline::*;

pub mod texture;
pub use texture::*;

pub mod descriptors;
pub use descriptors::*;