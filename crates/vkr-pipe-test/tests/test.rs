// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use vkr_core::{Buffer, Ctx, DescriptorPool, Image, ImageView, Sampler};
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

    let mut pool = DescriptorPool::new(&dev.device, 3, 3, 1, 1);
    let sets = pool.allocate(&uniform_pipeline.set_layouts);

    let view_buffer =
        Buffer::new::<[f32; 16]>(&dev.allocator, vk::BufferUsageFlags::UNIFORM_BUFFER);
    uniform_pipeline.write_set_0(sets[0], &view_buffer);

    let model_buffer =
        Buffer::new::<[f32; 16]>(&dev.allocator, vk::BufferUsageFlags::UNIFORM_BUFFER);
    uniform_pipeline.write_set_1(sets[1], &model_buffer);

    let color_buffer =
        Buffer::new::<[f32; 4]>(&dev.allocator, vk::BufferUsageFlags::UNIFORM_BUFFER);

    let white = [255, 255, 255, 255];
    let white_image = Image::from_data(&dev, &white, 1, 1, vk::Format::R8G8B8A8_SRGB);

    let white_view = ImageView::new(&dev.device, &white_image);
    let white_sampler = Sampler::new(&dev.device);
    let albedo = Texture::new(white_view.view, white_sampler.sampler);
    uniform_pipeline.write_set_2(sets[2], &color_buffer, &albedo);

    dev.wait();
}
