// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use im::internal::RawWrapper;
use memoffset::offset_of;

use crate::*;

vkr_pipe::pipewriter!("crates/shader/gui");

impl PipelineGui {
    fn bind_impl(&self, _frame: &mut Frame, _model: &Model, _node: Handle<Node>) {}
    fn draw_impl(&self, _frame: &mut Frame, _model: &Model, _node: Handle<Node>) {}
}

pub struct Gui {
    /// Not common as camera and model, therefore we store it here
    set_layouts: Vec<vk::DescriptorSetLayout>,

    pipeline_pool: PipelinePool,

    sampler: Sampler,
    view: ImageView,
    /// This is the font bitmap image, no need to cache it
    _image: Image,

    width: f32,
    height: f32,
    scale: [f32; 2],

    pub mouse_down: [bool; 5],

    pub ctx: im::Context,

    device: Rc<Device>,
}

impl VertexInput for im::DrawVert {
    fn get_bindings() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .stride(std::mem::size_of::<Self>() as u32)
            .build()
    }

    fn get_attributes() -> Vec<vk::VertexInputAttributeDescription> {
        let pos = vk::VertexInputAttributeDescription::builder()
            .location(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(offset_of!(im::DrawVert, pos) as u32)
            .build();

        let uv = vk::VertexInputAttributeDescription::builder()
            .location(1)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(offset_of!(im::DrawVert, uv) as u32)
            .build();

        let col = vk::VertexInputAttributeDescription::builder()
            .location(2)
            .format(vk::Format::R8G8B8A8_UNORM)
            .offset(offset_of!(im::DrawVert, col) as u32)
            .build();

        vec![pos, uv, col]
    }

    fn get_set_layouts(device: &Device) -> Vec<vk::DescriptorSetLayout> {
        let bindings = vec![vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .build()];

        vec![create_set_layout(device, &bindings)]
    }

    fn get_constants() -> Vec<vk::PushConstantRange> {
        vec![vk::PushConstantRange::builder()
            .offset(0)
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .size(std::mem::size_of::<na::Matrix4<f32>>() as u32)
            .build()]
    }

    fn write_set_image(
        device: &Device,
        set: vk::DescriptorSet,
        view: &ImageView,
        sampler: &Sampler,
    ) {
        let image_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .sampler(sampler.sampler)
            .image_view(view.view)
            .build();

        let image_write = vk::WriteDescriptorSet::builder()
            .dst_set(set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&[image_info])
            .build();

        let writes = vec![image_write];
        unsafe {
            device.update_descriptor_sets(&writes, &[]);
        }
    }

    fn get_depth_state() -> vk::PipelineDepthStencilStateCreateInfo {
        vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(false)
            .depth_write_enable(false)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false)
            .build()
    }

    fn get_color_blend(subpass: u32) -> Vec<vk::PipelineColorBlendAttachmentState> {
        assert!(subpass == 1);
        vec![vk::PipelineColorBlendAttachmentState::builder()
            .blend_enable(true)
            .color_write_mask(
                vk::ColorComponentFlags::R
                    | vk::ColorComponentFlags::G
                    | vk::ColorComponentFlags::B,
            )
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .src_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .build()]
    }
}

impl Gui {
    fn build_font(dev: &Dev, ctx: &mut im::Context) -> Image {
        let mut fonts = ctx.fonts();

        // @todo Use Roboto
        //fonts.add_font(&[im::FontSource::TtfData {
        //    data: include_bytes!("../res/font/roboto-medium.ttf"),
        //    size_pixels: 24.0,
        //    config: Some(im::FontConfig {
        //        name: Some(String::from("Roboto")),
        //        ..im::FontConfig::default()
        //    }),
        //}]);

        let font = fonts.build_rgba32_texture();
        let format = vk::Format::R8G8B8A8_SRGB;
        Image::from_data(dev, font.data, font.width, font.height, format)
    }

    pub fn new(win: &Win, dev: &Dev, _pass: &Pass) -> Self {
        let mut ctx = im::Context::create();

        let framebuffer_size = win.window.drawable_size();
        let win_size = win.window.size();

        let width = framebuffer_size.0 as f32;
        let height = framebuffer_size.1 as f32;
        let scale = [width / win_size.0 as f32, height / win_size.1 as f32];

        let io = ctx.io_mut();
        io.display_framebuffer_scale = scale;
        io.font_global_scale = scale[0];
        io.display_size[0] = width;
        io.display_size[1] = height;

        io.key_map[im::Key::Tab as usize] = sdl::keyboard::Scancode::Tab as u32;
        io.key_map[im::Key::LeftArrow as usize] = sdl::keyboard::Scancode::Left as u32;
        io.key_map[im::Key::RightArrow as usize] = sdl::keyboard::Scancode::Right as u32;
        io.key_map[im::Key::UpArrow as usize] = sdl::keyboard::Scancode::Up as u32;
        io.key_map[im::Key::DownArrow as usize] = sdl::keyboard::Scancode::Down as u32;
        io.key_map[im::Key::PageUp as usize] = sdl::keyboard::Scancode::PageUp as u32;
        io.key_map[im::Key::PageDown as usize] = sdl::keyboard::Scancode::PageDown as u32;
        io.key_map[im::Key::Home as usize] = sdl::keyboard::Scancode::Home as u32;
        io.key_map[im::Key::End as usize] = sdl::keyboard::Scancode::End as u32;
        io.key_map[im::Key::Delete as usize] = sdl::keyboard::Scancode::Delete as u32;
        io.key_map[im::Key::Backspace as usize] = sdl::keyboard::Scancode::Backspace as u32;
        io.key_map[im::Key::Enter as usize] = sdl::keyboard::Scancode::Return as u32;
        io.key_map[im::Key::Escape as usize] = sdl::keyboard::Scancode::Escape as u32;
        io.key_map[im::Key::Space as usize] = sdl::keyboard::Scancode::Space as u32;
        io.key_map[im::Key::A as usize] = sdl::keyboard::Scancode::A as u32;
        io.key_map[im::Key::C as usize] = sdl::keyboard::Scancode::C as u32;
        io.key_map[im::Key::V as usize] = sdl::keyboard::Scancode::V as u32;
        io.key_map[im::Key::X as usize] = sdl::keyboard::Scancode::X as u32;
        io.key_map[im::Key::Y as usize] = sdl::keyboard::Scancode::Y as u32;
        io.key_map[im::Key::Z as usize] = sdl::keyboard::Scancode::Z as u32;

        let image = Self::build_font(dev, &mut ctx);
        let view = ImageView::new(&dev.device, &image);
        let sampler = Sampler::new(&dev.device);

        let pipeline_pool = PipelinePool::new(dev);

        let set_layouts = im::DrawVert::get_set_layouts(&dev.device);

        Self {
            set_layouts,
            pipeline_pool,
            sampler,
            view,
            _image: image,
            width,
            height,
            scale,
            mouse_down: [false; 5],
            ctx,
            device: dev.device.clone(),
        }
    }

    pub fn set_mouse_state(&mut self, mouse_state: &sdl::mouse::MouseState) -> bool {
        let io = self.ctx.io_mut();
        io.mouse_pos[0] = mouse_state.x() as f32 * self.scale[0];
        io.mouse_pos[1] = mouse_state.y() as f32 * self.scale[1];

        io.mouse_down = [
            self.mouse_down[0] || mouse_state.left(),
            self.mouse_down[1] || mouse_state.right(),
            self.mouse_down[2] || mouse_state.middle(),
            self.mouse_down[3] || mouse_state.x1(),
            self.mouse_down[4] || mouse_state.x2(),
        ];
        self.mouse_down = [false; 5];

        let any_mouse_down = io.mouse_down.iter().any(|&b| b);
        any_mouse_down
    }

    pub fn set_drawable_size(&mut self, win: &Win) {
        let framebuffer_size = win.window.drawable_size();
        let win_size = win.window.size();
        self.width = framebuffer_size.0 as f32;
        self.height = framebuffer_size.1 as f32;
        self.scale = [
            self.width / win_size.0 as f32,
            self.height / win_size.1 as f32,
        ];

        let io = self.ctx.io_mut();
        io.display_framebuffer_scale = self.scale;
        io.font_global_scale = self.scale[0];
        io.display_size[0] = self.width;
        io.display_size[1] = self.height
    }

    pub fn update<F: FnOnce(&im::Ui)>(
        &mut self,
        delta: f32,
        frame_cache: &mut FrameCache,
        draw: F,
    ) {
        self.ctx.io_mut().delta_time = delta;
        let ui = self.ctx.frame();

        draw(&ui);

        let data = ui.render();

        if data.draw_lists_count() == 0 {
            return ();
        }

        let mut vertex_data = vec![];
        let mut index_data = vec![];
        for cmd_list in data.draw_lists() {
            let vtx_buffer = cmd_list.vtx_buffer();
            vertex_data.extend_from_slice(vtx_buffer);

            let idx_buffer = cmd_list.idx_buffer();
            index_data.extend_from_slice(idx_buffer);
        }

        let pipeline = self.pipeline_pool.get(ShaderVkrGuiShaders::Gui, 1);

        // Bind GUI pipeline
        frame_cache
            .command_buffer
            .bind_pipeline(pipeline.get_pipeline());

        let viewport = vk::Viewport::builder()
            .width(self.width)
            .height(self.height)
            .build();
        frame_cache.command_buffer.set_viewport(&viewport);

        // UI scale and translate via push constants
        let mut transform = na::Matrix4::<f32>::identity();

        let scale = na::Vector3::new(2.0 / self.width, 2.0 / self.height, 1.0);
        transform.append_nonuniform_scaling_mut(&scale);

        let shift = na::Vector3::new(-1.0, -1.0, 0.0);
        transform.append_translation_mut(&shift);

        let layout = pipeline.get_layout();

        let constants = unsafe {
            std::slice::from_raw_parts(
                transform.as_ptr() as *const u8,
                std::mem::size_of::<na::Matrix4<f32>>(),
            )
        };
        frame_cache.command_buffer.push_constants(
            layout,
            vk::ShaderStageFlags::VERTEX,
            0,
            constants,
        );

        // Bind descriptors
        if frame_cache.pipeline_cache.descriptors.gui_sets.is_empty() {
            frame_cache.pipeline_cache.descriptors.gui_sets = frame_cache
                .pipeline_cache
                .descriptors
                .allocate(&self.set_layouts);

            im::DrawVert::write_set_image(
                &self.device,
                frame_cache.pipeline_cache.descriptors.gui_sets[0],
                &self.view,
                &self.sampler,
            )
        }
        frame_cache.command_buffer.bind_descriptor_sets(
            layout,
            &frame_cache.pipeline_cache.descriptors.gui_sets,
            0,
        );

        // Upload vertex and index buffers
        frame_cache.gui_vertex_buffer.upload_arr(&vertex_data);
        frame_cache.gui_index_buffer.upload_arr(&index_data);
        // Bind vertex and index buffers
        frame_cache
            .command_buffer
            .bind_vertex_buffer(&frame_cache.gui_vertex_buffer);
        frame_cache
            .command_buffer
            .bind_index_buffer(&frame_cache.gui_index_buffer);

        let mut vertex_offset = 0;
        let mut index_offset = 0;

        for cmd_list in data.draw_lists() {
            for cmd in cmd_list.commands() {
                match cmd {
                    im::DrawCmd::Elements {
                        count,
                        cmd_params:
                            im::DrawCmdParams {
                                clip_rect,
                                vtx_offset,
                                idx_offset,
                                ..
                            },
                    } => {
                        let x = clip_rect[0] as i32;
                        let y = clip_rect[1] as i32;
                        let width = (clip_rect[2] - clip_rect[0]) as u32;
                        let height = (clip_rect[3] - clip_rect[1]) as u32;

                        let scissor = vk::Rect2D::builder()
                            .offset(vk::Offset2D::builder().x(x).y(y).build())
                            .extent(vk::Extent2D::builder().width(width).height(height).build())
                            .build();

                        frame_cache.command_buffer.set_scissor(&scissor);

                        frame_cache.command_buffer.draw_indexed(
                            count as u32,
                            index_offset + idx_offset as u32,
                            vertex_offset + vtx_offset as i32,
                        );
                    }
                    im::DrawCmd::RawCallback { callback, raw_cmd } => unsafe {
                        callback(cmd_list.raw(), raw_cmd)
                    },
                    _ => (),
                }
            }

            vertex_offset += cmd_list.vtx_buffer().len() as i32;
            index_offset += cmd_list.idx_buffer().len() as u32;
        }
    }

    /// Helper function to be called before ending a frame
    pub fn draw_debug_window(
        &mut self,
        delta: f32,
        frame: &mut Frame,
        model: &Model,
        camera: Handle<Node>,
    ) {
        self.update(delta, &mut frame.res, |ui| {
            im::Window::new(im::im_str!("Debug"))
                .no_decoration()
                .always_auto_resize(true)
                .save_settings(false)
                .focus_on_appearing(false)
                .no_nav()
                .position([16.0, 16.0], im::Condition::Always)
                .bg_alpha(0.33)
                .build(ui, || {
                    // Pipeline
                    //let mut current = if pipelines.debug.is_none() {
                    //    0
                    //} else {
                    //    pipelines.debug.unwrap() as usize
                    //};

                    ui.text("Pipeline:");

                    let _items = [
                        im::im_str!("None"),
                        im::im_str!("Albedo"),
                        im::im_str!("Normal"),
                    ];
                    ui.text(" · ");
                    ui.same_line(0.0);
                    //if im::ComboBox::new(im::im_str!("")).build_simple(
                    //    &ui,
                    //    &mut current,
                    //    &items,
                    //    &|&s| s.into(),
                    //) {
                    //    if current == 0 {
                    //        pipelines.debug = None;
                    //    } else {
                    //        pipelines.debug = Pipelines::from_ordinal(current as i8);
                    //    }
                    //}

                    // Camera
                    let camera_node = model.nodes.get(camera).unwrap();
                    let translation = camera_node.trs.get_translation();
                    let rotation = camera_node.trs.get_rotation();
                    ui.text(format!(
                        "Camera\n · trs ({:.2}, {:.2}, {:.2})\n · rot ({:.2}, {:.2}, {:.2}, {:.2})",
                        translation.x,
                        translation.y,
                        translation.z,
                        rotation.i,
                        rotation.j,
                        rotation.k,
                        rotation.w
                    ));
                });
        });
    }
}

impl Drop for Gui {
    fn drop(&mut self) {
        unsafe {
            for set_layout in &self.set_layouts {
                self.device.destroy_descriptor_set_layout(*set_layout, None);
            }
        }
    }
}
