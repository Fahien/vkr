// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

pub mod pipeline;
pub use pipeline::*;

pub use vkr_core::*;

pub struct Vkr {
    pub debug: Debug,
    pub ctx: Ctx,
}

impl Vkr {
    pub fn new(win: &Win) -> Self {
        let ctx = Ctx::builder().win(win).build();
        let debug = Debug::new(&ctx.entry, &ctx.instance);

        Self { ctx, debug }
    }
}
