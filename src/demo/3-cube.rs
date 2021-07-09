// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use vkr::*;

pub fn main() {
    let win = Win::new("Cube", 480, 480);
    let mut vkr = Vkr::new(win);
    let mut model = Model::new();

    let image = Image::load(&vkr.dev, "res/image/test.png");
    let view = model.views.push(ImageView::new(&vkr.dev.device, &image));
    model.images.push(image);
    let sampler = model.samplers.push(Sampler::new(&vkr.dev.device));
    let lena_texture = model.textures.push(Texture::new(view, sampler));

    let mut green_material = Material::textured(lena_texture);
    green_material.color = Color::new(0.8, 0.6, 0.7, 0.3);
    let green_material = model.materials.push(green_material);

    let mut cube_primitive = Primitive::cube(&vkr.dev.allocator);
    cube_primitive.material = green_material;
    let cube_primitive = model.primitives.push(cube_primitive);

    let cube_mesh = Mesh::new(vec![cube_primitive]);
    let cube_mesh = model.meshes.push(cube_mesh);

    let mut cube_node = Node::new();
    cube_node.mesh = cube_mesh;
    let cube_node = model.nodes.push(cube_node);

    let camera = Camera::perspective(1.0);
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

        vkr.update_camera(&mut model, camera_node);

        let mut frame = frame.unwrap();
        frame.bind(&vkr.pipelines.main, &model, camera_node);
        frame.draw::<Vertex>(&vkr.pipelines.main, &model, cube_node);
        vkr.end_frame(frame);
    }

    vkr.dev.wait();
}
