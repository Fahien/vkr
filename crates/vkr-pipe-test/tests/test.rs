// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use vkr_core::{Buffer, Ctx};
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

    let uniform_pipeline = cache.get(ShaderSimpleShader::Uniform);
    assert!(uniform_pipeline.get_name() == "Uniform");

    let uniform_pipeline = uniform_pipeline
        .as_any()
        .downcast_ref::<PipelineUniform>()
        .unwrap();

    let set = vk::DescriptorSet::null();

    let view_buffer = Buffer::new::<u32>(&dev.allocator, vk::BufferUsageFlags::UNIFORM_BUFFER);
    uniform_pipeline.write_set_0(set, &view_buffer);

    let model_buffer = Buffer::new::<u32>(&dev.allocator, vk::BufferUsageFlags::UNIFORM_BUFFER);
    uniform_pipeline.write_set_1(set, &model_buffer);

    let albedo = Texture::new(vk::ImageView::null(), vk::Sampler::null());
    uniform_pipeline.write_set_2(set, &albedo);

    dev.wait();
}
