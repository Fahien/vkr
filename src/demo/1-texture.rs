// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use vkr::*;

pub fn main() {
    let win = Win::new("Texture", 480, 480);
    let (width, height) = win.window.drawable_size();
    let mut vkr = Vkr::new(win);

    let mut line_pipeline = Pipeline::line(&vkr.dev.device, &vkr.pass, width, height);
    let mut triangle_pipeline = Pipeline::main(&vkr.dev.device, &vkr.pass, width, height);

    let mut model = Model::new();

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
    let lines_primitive = model.primitives.push(lines_primitive);
    let lines_mesh = Mesh::new(vec![lines_primitive]);
    let lines_mesh = model.meshes.push(lines_mesh);
    let mut lines = Node::new();
    lines.trs.translate(&na::Vector3::new(0.0, 0.0, -0.5));
    lines.mesh = lines_mesh;
    let lines = model.nodes.push(lines);

    let image = Image::load(&vkr.dev, "res/image/test.png");
    let view = ImageView::new(&vkr.dev.device, &image);
    model.images.push(image);
    let view = model.views.push(view);
    let sampler = model.samplers.push(Sampler::new(&vkr.dev.device));
    let texture = Texture::new(view, sampler);
    let texture = model.textures.push(texture);
    let material = Material::textured(texture);
    let material = model.materials.push(material);
    let mut rect_primitive = Primitive::quad(&vkr.dev.allocator, [1.0, 1.0]);
    rect_primitive.material = material;
    let rect_primitive = model.primitives.push(rect_primitive);
    let rect_mesh = Mesh::new(vec![rect_primitive]);
    let rect_mesh = model.meshes.push(rect_mesh);
    let mut rect = Node::new();
    rect.trs.translate(&na::Vector3::new(0.0, 0.3, -0.2));
    rect.mesh = rect_mesh;
    let rect = model.nodes.push(rect);

    let camera = Camera::orthographic(-1.0, 1.0, -1.0, 1.0, 0.1, 1.0);
    let camera = model.cameras.push(camera);
    let mut camera_node = Node::new();
    camera_node.camera = camera;
    camera_node.trs.translate(&na::Vector3::new(0.3, 0.3, 0.0));
    let camera_node = model.nodes.push(camera_node);

    'running: loop {
        if !vkr.handle_events() {
            break 'running;
        }

        let delta = vkr.timer.get_delta().as_secs_f32();

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
        frame.draw::<Line>(&mut line_pipeline, &model, lines);
        frame.bind(&mut triangle_pipeline, &model, camera_node);
        frame.draw::<Vertex>(&mut triangle_pipeline, &model, rect);

        vkr.gui
            .draw_debug_window(delta, &mut frame, &model, camera_node);

        vkr.end_frame(frame);
    }

    vkr.dev.wait();
}
