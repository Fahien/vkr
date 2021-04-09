// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use imgui as im;
use nalgebra as na;
use sdl2 as sdl;

mod util;
use util::*;

mod model;
use model::*;

mod pipeline;
use pipeline::*;

mod commands;
use commands::*;

mod image;
use image::*;

mod queue;

mod shader;
use shader::*;

mod sampler;
use sampler::*;

mod gfx;
use gfx::*;

mod descriptor;
use descriptor::*;

mod primitive;
use primitive::*;

mod sync;
use sync::*;

mod gui;

mod frame;
use frame::*;

pub fn main() {
    let mut timer = Timer::new();

    let win = Win::new();
    let (width, height) = win.window.drawable_size();
    let mut vkr = Vkr::new(win);

    let mut line_pipeline = Pipeline::line(&vkr.dev.device, &vkr.pass, width, height);

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
        Primitive::new(&vkr.dev.allocator, &lines_vertices)
    };

    let mut triangle_pipeline = Pipeline::main(&vkr.dev.device, &vkr.pass, width, height);

    let rect_primitive = Primitive::quad(&vkr.dev.allocator);

    let mut model = Model::new();

    let camera = Camera::orthographic(-1.0, 1.0, -1.0, 1.0, 0.1, 1.0);
    let camera = model.cameras.push(camera);
    let mut camera_node = Node::new();
    camera_node.camera = camera;
    camera_node.trs.translate(&na::Vector3::new(0.3, 0.3, 0.0));
    let camera_node = model.nodes.push(camera_node);

    let mut rect = Node::new();
    rect.trs.translate(&na::Vector3::new(0.0, 0.3, -0.2));
    let rect = model.nodes.push(rect);

    let mut lines = Node::new();
    lines.trs.translate(&na::Vector3::new(0.0, 0.0, -0.5));
    let lines = model.nodes.push(lines);

    let image = Image::load(&vkr.dev, "res/image/test.png");

    let view = ImageView::new(&vkr.dev.device, &image);

    model.images.push(image);

    let view = model.views.push(view);

    let sampler = model.samplers.push(Sampler::new(&vkr.dev.device));

    let texture = Texture::new(view, sampler);
    let texture = model.textures.push(texture);

    'running: loop {
        if !vkr.handle_events() {
            break 'running;
        }

        let delta = timer.get_delta().as_secs_f32();

        // Move camera
        {
            let node = model.nodes.get_mut(camera_node).unwrap();
            let mut translation = na::Vector3::new(0.0, 0.0, 0.0);

            let speed = 4.0;

            let key = vkr.win.as_ref().unwrap().events.keyboard_state();
            if key.is_scancode_pressed(sdl::keyboard::Scancode::A) {
                translation.x -= speed * delta;
            }
            if key.is_scancode_pressed(sdl::keyboard::Scancode::D) {
                translation.x += speed * delta;
            }
            if key.is_scancode_pressed(sdl::keyboard::Scancode::W) {
                translation.y += speed * delta;
            }
            if key.is_scancode_pressed(sdl::keyboard::Scancode::S) {
                translation.y -= speed * delta;
            }

            node.trs.translate(&translation);
        }

        // Move scene
        {
            let rot = na::UnitQuaternion::from_axis_angle(&na::Vector3::z_axis(), delta / 2.0);
            model.nodes.get_mut(rect).unwrap().trs.rotate(&rot);
            let rot = na::UnitQuaternion::from_axis_angle(&na::Vector3::z_axis(), -delta / 2.0);
            model.nodes.get_mut(lines).unwrap().trs.rotate(&rot);
        }

        let frame = vkr.begin_frame();
        if frame.is_none() {
            continue;
        }

        if vkr.resized {
            let node = model.nodes.get_mut(camera_node).unwrap();
            let camera = model.cameras.get_mut(node.camera).unwrap();
            let (width, height) = vkr.win.as_ref().unwrap().window.drawable_size();
            let aspect = width as f32 / height as f32;
            *camera = Camera::orthographic(-aspect, aspect, -1.0, 1.0, 0.1, 1.0);
        }

        let mut frame = frame.unwrap();

        frame.bind(&mut line_pipeline, &model, camera_node);
        frame.draw::<Line>(
            &mut line_pipeline,
            &model,
            &lines_primitive,
            lines,
            Handle::none(),
        );
        frame.bind(&mut triangle_pipeline, &model, camera_node);
        frame.draw::<Vertex>(&mut triangle_pipeline, &model, &rect_primitive, rect, texture);

        vkr.gui.update(delta, &mut frame.res, |ui| {
            im::Window::new(im::im_str!("Debug"))
                .no_decoration()
                .always_auto_resize(true)
                .save_settings(false)
                .focus_on_appearing(false)
                .no_nav()
                .position([16.0, 16.0], im::Condition::Always)
                .bg_alpha(0.33)
                .build(ui, || {
                    let node = model.nodes.get_mut(camera_node).unwrap();
                    let translation = node.trs.get_translation();
                    ui.text(format!(
                        "Camera: ({:.2}, {:.2}, {:.2})",
                        translation.x, translation.y, translation.z
                    ));
                });
        });

        vkr.end_frame(frame, delta);
    }

    vkr.dev.wait();
}
