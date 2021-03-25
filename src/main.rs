// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{fs::File, io::BufWriter, path::Path};

use nalgebra as na;
use sdl::{event::Event, keyboard::Keycode};
use sdl2 as sdl;

mod util;
use util::*;

mod model;
use model::*;

mod image;
use image::*;

mod frame;
use frame::*;

mod sync;
use sync::*;

mod queue;
use queue::*;

mod shader;
use shader::*;

mod sampler;
use sampler::*;

mod gfx;
use gfx::*;

pub fn main() {
    let mut timer = Timer::new();

    //let win = Win::new();
    let (width, height) = (600, 600); //win.window.drawable_size();

    let vkr = Vkr::headless();

    let mut dev = Dev::new(&vkr.ctx, None);

    let pass = Pass::new(&mut dev);

    let mut sfs = OffscreenFrames::new(&mut dev, width, height, &pass);
    println!("Offscreen created");

    let line_pipeline = Pipeline::line(&mut dev, &pass, width, height);

    let lines_primitive = {
        // Notice how the first line appears at the top of the picture as Vulkan Y axis is pointing downwards
        let lines_vertices = vec![
            Point::new(
                na::Vector3::new(-0.5, -0.5, 0.0),
                Color::new(1.0, 1.0, 0.0, 1.0),
            ),
            Point::new(
                na::Vector3::new(0.5, -0.5, 0.0),
                Color::new(0.2, 1.0, 1.0, 1.0),
            ),
            Point::new(
                na::Vector3::new(0.5, 0.5, 0.0),
                Color::new(0.1, 1.0, 0.0, 1.0),
            ),
            Point::new(
                na::Vector3::new(-0.5, 0.5, 0.0),
                Color::new(1.0, 0.1, 1.0, 1.0),
            ),
            Point::new(
                na::Vector3::new(-0.5, -0.5, 0.0),
                Color::new(1.0, 1.0, 0.0, 1.0),
            ),
        ];
        Primitive::new(&dev.allocator, &lines_vertices)
    };

    let triangle_pipeline = Pipeline::main(&mut dev, &pass, width, height);

    let rect_primitive = Primitive::quad(&dev.allocator);

    let mut model = Model::new();

    let mut rect = Node::new();
    rect.trs.translate(&na::Vector3::new(0.0, 0.0, 0.6));
    let rect = model.nodes.push(rect);

    let mut lines = Node::new();
    lines.trs.scale(&na::Vector3::new(0.5, 0.5, 0.5));
    let lines = model.nodes.push(lines);

    let image = Image::load(&dev, "res/image/test.png");

    let view = ImageView::new(&dev.device, &image);

    model.images.push(image);

    let view = model.views.push(view);

    let sampler = model.samplers.push(Sampler::new(&dev.device));

    let texture = Texture::new(view, sampler);
    let texture = model.textures.push(texture);

    let delta = timer.get_delta().as_secs_f32();
    let rot = na::UnitQuaternion::from_axis_angle(&na::Vector3::z_axis(), delta / 2.0);
    model.nodes.get_mut(rect).unwrap().trs.rotate(&rot);
    let rot = na::UnitQuaternion::from_axis_angle(&na::Vector3::z_axis(), -delta / 2.0);
    model.nodes.get_mut(lines).unwrap().trs.rotate(&rot);

    println!("Next frame");
    let frame = sfs.next_frame();
    let frame = frame.unwrap();

    frame.begin(&pass, width, height);
    frame.draw::<Line>(
        &line_pipeline,
        &model,
        &lines_primitive,
        lines,
        Handle::none(),
    );
    frame.draw::<Vertex>(&triangle_pipeline, &model, &rect_primitive, rect, texture);
    frame.end();
    println!("Submitting");
    frame.submit(&dev);

    println!("Waiting for fence");
    frame.res.fence.wait();

    // Read image
    let image = &mut sfs.images[0];
    let mut buffer = Buffer::copy_from(&dev, image);

    let path = Path::new(r"res/image/screenshot.png");
    let file = File::create(path).unwrap();
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, image.extent.width, image.extent.height);
    encoder.set_color(png::ColorType::RGBA);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();

    let data = buffer.map();
    writer.write_image_data(data).unwrap();
    buffer.unmap();

    // Make sure device is idle before releasing Vulkan resources
    dev.wait();
}
