// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use vkr::*;

pub fn main() {
    let win = Win::new("Normal", 480, 480);
    let (width, height) = win.window.drawable_size();

    let mut vkr = Vkr::new(win);

    let triangle_pipeline = Pipeline::normal(&vkr.dev, &vkr.pass, width, height);

    let mut model = Model::new();

    let cube_primitive = Primitive::cube(&vkr.dev.allocator);
    let cube_primitive = model.primitives.push(cube_primitive);

    let cube_mesh = Mesh::new(vec![cube_primitive]);
    let cube_mesh = model.meshes.push(cube_mesh);

    let mut cube_node = Node::new();
    cube_node.mesh = cube_mesh;
    let cube_node = model.nodes.push(cube_node);

    let camera = Camera::perspective(1.0, 3.14 / 4.0, 0.1, 100.0);
    let camera = model.cameras.push(camera);

    let mut camera_node = Node::new();
    camera_node.camera = camera;
    camera_node.trs.translate(&na::Vector3::new(0.0, 0.0, 4.0));
    let camera_node = model.nodes.push(camera_node);

    'running: loop {
        if !vkr.handle_events() {
            break 'running;
        }

        let delta = vkr.timer.get_delta().as_secs_f32();

        if let Some(cube_node) = model.nodes.get_mut(cube_node) {
            let rot = na::UnitQuaternion::from_axis_angle(&na::Vector3::y_axis(), delta / 2.0);
            cube_node.trs.rotate(&rot);
        }

        let frame = vkr.begin_frame();
        if frame.is_none() {
            continue;
        }

        let mut frame = frame.unwrap();

        frame.bind(&&triangle_pipeline, &model, camera_node);

        frame.draw::<Vertex>(
            &triangle_pipeline,
            &model.nodes,
            &model.meshes,
            &model.primitives,
            &model.materials,
            &model.samplers,
            &model.views,
            &model.textures,
            cube_node,
        );

        vkr.end_frame(frame);
    }

    vkr.dev.wait();
}
