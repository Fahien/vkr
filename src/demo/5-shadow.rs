// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use vkr::*;

fn create_floor(vkr: &mut Vkr, model: &mut Model) -> Handle<Node> {
    let red_material = model
        .materials
        .push(Material::new(Color::new(1.0, 0.0, 0.0, 1.0)));

    let mut floor_primitive = Primitive::cube(&vkr.dev.allocator);
    floor_primitive.material = red_material;
    let floor_primitive = model.primitives.push(floor_primitive);

    let floor_mesh = model.meshes.push(Mesh::new(vec![floor_primitive]));

    let mut floor = Node::new();
    floor.mesh = floor_mesh;
    floor.trs.scale(&na::Vector3::new(8.0, 1.0, 8.8));
    floor.trs.translate(&na::Vector3::new(0.0, -1.0, 0.0));

    model.nodes.push(floor)
}

fn create_cube(vkr: &mut Vkr, model: &mut Model) -> Handle<Node> {
    let cube_primitive = Primitive::cube(&vkr.dev.allocator);
    let cube_primitive = model.primitives.push(cube_primitive);

    let cube_mesh = Mesh::new(vec![cube_primitive]);
    let cube_mesh = model.meshes.push(cube_mesh);

    let mut cube_node = Node::new();
    cube_node.mesh = cube_mesh;
    model.nodes.push(cube_node)
}

fn create_light(model: &mut Model) -> Handle<Node> {
    let light = Light::new(2.0, 4.0, 2.0);
    let light = model.lights.push(light);
    let mut light_node = Node::new();
    light_node.light = light;
    model.nodes.push(light_node)
}

fn create_camera(model: &mut Model) -> Handle<Node> {
    let camera = Camera::perspective(1.0);
    let camera = model.cameras.push(camera);

    let mut camera_node = Node::new();
    camera_node.camera = camera;
    camera_node.trs.translate(&na::Vector3::new(0.0, 2.0, 4.0));
    camera_node.trs.rotate(&na::UnitQuaternion::from_axis_angle(
        &na::Vector3::x_axis(),
        0.35,
    ));
    model.nodes.push(camera_node)
}

fn rotate(node: Handle<Node>, model: &mut Model, delta: f32) {
    if let Some(model_node) = model.nodes.get_mut(node) {
        let rot = na::UnitQuaternion::from_axis_angle(&na::Vector3::y_axis(), delta / 2.0);
        let rot = rot * model_node.trs.get_rotation();
        model_node.trs.set_rotation(&rot);
    }
}

pub fn main() {
    let win = Win::new("Normal", 480, 480);
    let mut vkr = Vkr::new(win);
    let mut model = Model::new();

    let floor = create_floor(&mut vkr, &mut model);
    let cube = create_cube(&mut vkr, &mut model);
    let light = create_light(&mut model);
    let camera = create_camera(&mut model);

    'running: loop {
        if !vkr.handle_events() {
            break 'running;
        }

        let delta = vkr.timer.get_delta().as_secs_f32();
        rotate(cube, &mut model, delta);

        let frame = vkr.begin_frame();
        if frame.is_none() {
            continue;
        }
        let mut frame = frame.unwrap();

        vkr.update_camera(&mut model, camera);

        frame.bind(vkr.pipelines.get_for::<Vertex>(), &model, camera);
        frame.draw::<Vertex>(&vkr.pipelines, &model, light);
        frame.draw::<Vertex>(&vkr.pipelines, &model, floor);
        frame.draw::<Vertex>(&vkr.pipelines, &model, cube);

        vkr.end_scene(&mut frame);
        vkr.gui
            .draw_debug_window(delta, &mut frame, &mut vkr.pipelines, &model, camera);
        vkr.end_frame(frame);
    }

    vkr.dev.wait();
}
