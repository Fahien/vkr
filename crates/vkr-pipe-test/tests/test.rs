// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use vkr_core::Ctx;
use vkr_pipe::*;

pipewriter!("crates/vkr-pipe-test/shader/simple");

#[test]
fn load_simple_shader() {
    const SHADERS: &[u8] = include_bytes!(env!("simple_shader.spv"));
    assert!(!SHADERS.is_empty());
}

#[test]
fn build_simple_shader() {
    let ctx = Ctx::builder().build();
    let dev = Dev::new(&ctx, None);

    let shader_crate = CrateSimpleShader::new(&dev);
    let _main_pipeline = &shader_crate.main;
    let _secondary_pipeline = &shader_crate.secondary;

    assert!(1 == 1);

    dev.wait();
}
