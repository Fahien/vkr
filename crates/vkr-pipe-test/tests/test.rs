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

    let mut cache = PipelineCache::new(&dev);

    let main_pipeline = cache.get(ShaderSimpleShader::Main);
    assert!(main_pipeline.get_name() == "Main");

    let secondary_pipeline = cache.get(ShaderSimpleShader::Secondary);
    eprintln!("{}", secondary_pipeline.get_name());
    assert!(secondary_pipeline.get_name() == "Secondary");

    dev.wait();
}
