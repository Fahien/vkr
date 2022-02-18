// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::any::Any;

use vkr_core::{Buffer, Ctx, DescriptorPool, Image, ImageView, Sampler, PipelinePool, VertexInputDescription, Vertex};
use vkr_pipe::*;

pipewriter!("../vkr-pipe-test/shader/simple");

impl PipelineUniform {
    fn bind_impl(&self, _frame: &mut Frame, _model: &Model, _node: Handle<Node>) {
        println!("Bind");
    }

    fn draw_impl(&self, _frame: &mut Frame, _model: &Model, _node: Handle<Node>) {
        println!("Draw");
    }
}

impl PipelineMain {
    fn bind_impl(&self, _frame: &mut Frame, _model: &Model, _node: Handle<Node>) {
        println!("Bind");
    }

    fn draw_impl(&self, _frame: &mut Frame, _model: &Model, _node: Handle<Node>) {
        println!("Draw");
    }
}

impl PipelineSecondary {
    fn bind_impl(&self, _frame: &mut Frame, _model: &Model, _node: Handle<Node>) {
        println!("Bind");
    }

    fn draw_impl(&self, _frame: &mut Frame, _model: &Model, _node: Handle<Node>) {
        println!("Draw");
    }
}

#[test]
fn load_simple_shader() {
    const SHADERS: &[u8] = include_bytes!(env!("simple_shader.spv"));
    assert!(!SHADERS.is_empty());
}

fn as_uni(pipeline: &mut dyn Any) -> &mut PipelineUniform {
    pipeline.downcast_mut().expect("Failed")
}

#[test]
fn build_simple_shader() {
    let ctx = Ctx::builder().debug(true).build();
    let dev = Dev::new(&ctx, None);

    let mut cache = PipelinePoolSimpleShader::new(&dev);

    let vertex_input = VertexInputDescription::new::<Vertex>();

    let main_pipeline = cache.get(&vertex_input, ShaderSimpleShader::Main.into(), 0);
    assert!(main_pipeline.get_name() == "Main");

    let secondary_pipeline = cache.get(&vertex_input, ShaderSimpleShader::Secondary.into(), 0);
    eprintln!("{}", secondary_pipeline.get_name());
    assert!(secondary_pipeline.get_name() == "Secondary");

    let uniform_pipeline = cache.get_mut(&vertex_input, ShaderSimpleShader::Uniform.into(), 0);
    assert!(uniform_pipeline.get_name() == "Uniform");

    let uniform_pipeline = as_uni(uniform_pipeline.as_any_mut());

    let mut pool = DescriptorPool::new(&dev.device, 4, 3, 1, 2);
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

    let _cache = uniform_pipeline.get_cache(0);

    dev.wait();
}
