use vkr::*;

fn create_texture(
    vkr: &Vkr,
    model: &mut Model,
    sampler: Handle<Sampler>,
    path: &str,
) -> Handle<Texture> {
    let image = Image::load(&vkr.dev, path);
    let view = ImageView::new(&vkr.dev.device, &image);
    model.images.push(image);
    let view = model.views.push(view);

    let texture = Texture::new(view, sampler);
    model.textures.push(texture)
}

fn create_back(
    vkr: &Vkr,
    model: &mut Model,
    sampler: Handle<Sampler>,
    path: &str,
    aspect: f32,
    i: i32,
) -> Handle<Node> {
    let texture = create_texture(vkr, model, sampler, path);
    let material = Material::textured(texture);
    let material = model.materials.push(material);
    let mut primitive = Primitive::quad(&vkr.dev.allocator, [2.0, 1.0]);
    primitive.material = material;
    let primitive = model.primitives.push(primitive);

    let mesh = Mesh::new(vec![primitive]);
    let mesh = model.meshes.push(mesh);
    let mut back = Node::new();
    back.mesh = mesh;
    back.trs
        .translate(&na::Vector3::new(0.0, 0.0, i as f32 * 0.01));
    let screen_width = 2.0 * aspect;
    // A node used as a parallax background should be twice as wide of the background
    let node_width = 2.0 * screen_width;
    back.trs.scale(&na::Vector3::new(node_width, 2.0, 1.0));

    let script = Script::new(Box::new(move |delta, nodes: &mut Pack<Node>, hnode| {
        let speed = 0.0125 * i as f32;
        let x = -speed * delta;
        let node = nodes.get_mut(hnode).unwrap();
        node.trs.translate(&na::Vector3::new(x, 0.0, 0.0));

        let trs = node.trs.get_translation();
        if trs.x < -0.25 {
            node.trs
                .translate(&na::Vector3::new(-trs.x + 0.25, 0.0, 0.0));
        }
    }));

    back.script = model.scripts.push(script);
    model.nodes.push(back)
}

fn create_scene(vkr: &Vkr, model: &mut Model) -> Handle<Node> {
    // Use same sampler with repeat mode
    let sampler = Sampler::new(&vkr.dev.device);
    let sampler = model.samplers.push(sampler);

    let (width, height) = vkr.win.as_ref().unwrap().window.drawable_size();
    let aspect = width as f32 / height as f32;

    let mut scene = Node::new();

    for i in 0..5 {
        let back = create_back(
            vkr,
            model,
            sampler,
            &format!("res/image/city/back{}.png", i),
            aspect,
            i,
        );
        scene.children.push(back);
    }

    model.nodes.push(scene)
}

fn main() {
    let win = Win::new("Parallax", 1920 / 2, 1080 / 2);
    let (width, height) = win.window.drawable_size();
    let aspect = width as f32 / height as f32;

    let mut vkr = Vkr::new(win);

    let pipeline = Pipeline::main(&vkr.dev, &vkr.pass, width, height);

    let mut model = Model::new();

    let scene = create_scene(&vkr, &mut model);

    let mut camera_node = Node::new();
    camera_node.trs.translate(&na::Vector3::new(0.0, 0.0, 0.5));

    camera_node.camera = model
        .cameras
        .push(Camera::orthographic(-aspect, aspect, -1.0, 1.0, 0.1, 1.0));
    let camera_node = model.nodes.push(camera_node);

    while vkr.handle_events() {
        let delta = vkr.timer.get_delta().as_secs_f32();

        Script::update(delta, &mut model.nodes, &model.scripts, scene);

        if let Some(mut frame) = vkr.begin_frame() {
            frame.bind(&pipeline, &model, camera_node);
            frame.draw::<Vertex>(&pipeline, &model, scene);

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
                        ui.text("Background");
                        let scene_children = model.nodes.get(scene).unwrap().children.clone();
                        for (i, child) in scene_children.iter().enumerate() {
                            let child = model.nodes.get_mut(*child).unwrap();
                            let mut translation = child.trs.get_translation();
                            let mut trs = [translation.x, translation.y, translation.z];
                            if im::Drag::new(&im::im_str!("{}", i))
                                .display_format(im::im_str!("%.2f"))
                                .speed(0.01)
                                .build_array(ui, &mut trs)
                            {
                                translation.x = trs[0] - translation.x;
                                translation.y = trs[1] - translation.y;
                                translation.z = trs[2] - translation.z;
                                child.trs.translate(&translation);
                            }
                        }
                    });
            });

            vkr.end_frame(frame);
        }
    }

    vkr.dev.wait();
}
