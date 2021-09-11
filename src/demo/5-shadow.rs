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

/// Creates a directional light. Now, since these is going to generate a shadow map
// we need to associate a camera to the node as well.
fn create_light(model: &mut Model) -> Handle<Node> {
    let light = Light::new();
    let light = model.lights.push(light);

    let size = 16.0;
    let camera = model.cameras.push(Camera::orthographic(
        -size / 2.0,
        size / 2.0,
        -size / 2.0,
        size / 2.0,
        0.125,
        size,
    ));

    let mut light_node = Node::new();
    light_node.light = light;
    light_node.camera = camera;

    light_node.trs.translate(&na::Vector3::new(4.0, 4.0, 0.0));
    light_node.trs.look_at(&na::Point3::origin());
    model.nodes.push(light_node)
}

fn create_camera(model: &mut Model) -> Handle<Node> {
    let camera = Camera::perspective(1.0);
    let camera = model.cameras.push(camera);

    let mut camera_node = Node::new();
    camera_node.camera = camera;
    camera_node.trs.translate(&na::Vector3::new(4.0, 4.0, -4.0));
    camera_node.trs.look_at(&na::Point3::origin());
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

        // First of all we should generate a shadowmap
        vkr.update_camera(&mut model, light);
        vkr.update_camera(&mut model, camera);

        // Light pre pass
        let light_pipeline = vkr.pipelines.get(Pipelines::SHADOW);
        frame.bind_light(light_pipeline, &model, light);
        frame.draw::<Vertex>(light_pipeline, &model, floor);
        frame.draw::<Vertex>(light_pipeline, &model, cube);
        vkr.end_light(&mut frame);

        let pipeline = vkr.pipelines.get(Pipelines::MAIN);
        frame.bind(pipeline, &model, camera);
        frame.draw::<Vertex>(pipeline, &model, light);
        frame.draw::<Vertex>(pipeline, &model, floor);
        frame.draw::<Vertex>(pipeline, &model, cube);

        vkr.end_scene(&mut frame);
        vkr.gui
            .draw_debug_window(delta, &mut frame, &mut vkr.pipelines, &model, camera);
        vkr.end_frame(frame);
    }

    vkr.dev.wait();
}
