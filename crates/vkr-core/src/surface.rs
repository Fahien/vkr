// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::vk;

#[cfg(feature = "win")]
use crate::{ctx::Ctx, win::Win};

pub struct Surface {
    pub surface: vk::SurfaceKHR,
    pub ext: ash::extensions::khr::Surface,
}

impl Surface {
    #[cfg(feature = "win")]
    pub fn new(win: &Win, ctx: &Ctx) -> Self {
        use ash::vk::Handle;

        let surface = win
            .window
            .vulkan_create_surface(ctx.instance.handle().as_raw() as usize)
            .expect("Failed to create surface");
        let surface: vk::SurfaceKHR = vk::Handle::from_raw(surface);
        let ext = ash::extensions::khr::Surface::new(&ctx.entry, &ctx.instance);

        Self { surface, ext }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.ext.destroy_surface(self.surface, None);
        }
    }
}
