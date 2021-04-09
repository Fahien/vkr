// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{ffi::CString, rc::Rc};

use super::*;

use ash::{version::DeviceV1_0, *};
use im::internal::RawWrapper;
use imgui as im;
use memoffset::offset_of;

pub struct Gui {
    // Not common as camera and model, therefore we store it here
    set_layouts: Vec<vk::DescriptorSetLayout>,

    pipeline: Pipeline,

    // No need to cache this image
    sampler: Sampler,
    view: ImageView,
    image: Image,

    scale: [f32; 2],

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

    fn get_color_blend() -> Vec<vk::PipelineColorBlendAttachmentState> {
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

impl Pipeline {
    fn gui(dev: &Dev, pass: &Pass, width: u32, height: u32) -> Self {
        let shader = ShaderModule::gui(&dev.device);
        let vs = CString::new("gui_vs").expect("Failed to create entrypoint");
        let fs = CString::new("gui_fs").expect("Failed to create entrypoint");
        Self::new::<im::DrawVert>(
            dev,
            shader.get_vert(&vs),
            shader.get_frag(&fs),
            vk::PrimitiveTopology::TRIANGLE_LIST,
            pass,
            width,
            height,
        )
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

    pub fn new(win: &Win, dev: &Dev, pass: &Pass) -> Self {
        let mut ctx = im::Context::create();

        let framebuffer_size = win.window.drawable_size();
        let win_size = win.window.size();
        let scale = [
            framebuffer_size.0 as f32 / win_size.0 as f32,
            framebuffer_size.1 as f32 / win_size.1 as f32,
        ];

        let io = ctx.io_mut();
        io.display_framebuffer_scale = scale;
        io.font_global_scale = scale[0];
        io.display_size[0] = framebuffer_size.0 as f32;
        io.display_size[1] = framebuffer_size.1 as f32;

        let image = Self::build_font(dev, &mut ctx);
        let view = ImageView::new(&dev.device, &image);
        let sampler = Sampler::new(&dev.device);

        let pipeline = Pipeline::gui(dev, pass, framebuffer_size.0, framebuffer_size.1);

        let set_layouts = im::DrawVert::get_set_layouts(&dev.device);

        Self {
            set_layouts,
            pipeline,
            sampler,
            view,
            image,
            scale,
            ctx,
            device: dev.device.clone(),
        }
    }

    pub fn set_mouse_state(&mut self, mouse_state: &sdl::mouse::MouseState) {
        let io = self.ctx.io_mut();
        io.mouse_pos[0] = mouse_state.x() as f32 * self.scale[0];
        io.mouse_pos[1] = mouse_state.y() as f32 * self.scale[1];
        io.mouse_down[0] = mouse_state.left();
        io.mouse_down[1] = mouse_state.right();
        io.mouse_down[2] = mouse_state.middle();
    }

    pub fn set_drawable_size(&mut self, win: &Win) {
        let framebuffer_size = win.window.drawable_size();
        let win_size = win.window.size();
        self.scale = [
            framebuffer_size.0 as f32 / win_size.0 as f32,
            framebuffer_size.1 as f32 / win_size.1 as f32,
        ];

        let io = self.ctx.io_mut();
        io.display_framebuffer_scale = self.scale;
        io.font_global_scale = self.scale[0];
        io.display_size[0] = framebuffer_size.0 as f32;
        io.display_size[1] = framebuffer_size.1 as f32
    }

    pub fn update(&mut self, res: &mut Frameres, delta: f32) {
        self.ctx.io_mut().delta_time = delta;

        self.render(res);
    }

    fn render(&mut self, res: &mut Frameres) {
        let width = self.ctx.io().display_size[0];
        let height = self.ctx.io().display_size[1];

        let ui = self.ctx.frame();

        let mut opened = true;
        ui.show_demo_window(&mut opened);

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

        // Bind GUI pipeline
        res.command_buffer.bind_pipeline(&self.pipeline);

        let viewport = vk::Viewport::builder().width(width).height(height).build();
        res.command_buffer.set_viewport(&viewport);

        // UI scale and translate via push constants
        let mut transform = na::Matrix4::<f32>::identity();

        let scale = na::Vector3::new(2.0 / width, 2.0 / height, 1.0);
        transform.append_nonuniform_scaling_mut(&scale);

        let shift = na::Vector3::new(-1.0, -1.0, 0.0);
        transform.append_translation_mut(&shift);

        let constants = unsafe {
            std::slice::from_raw_parts(
                transform.as_ptr() as *const u8,
                std::mem::size_of::<na::Matrix4<f32>>(),
            )
        };
        res.command_buffer.push_constants(
            &self.pipeline,
            vk::ShaderStageFlags::VERTEX,
            0,
            constants,
        );

        // Bind descriptors
        if res.descriptors.gui_sets.is_empty() {
            res.descriptors.gui_sets = res.descriptors.allocate(&self.set_layouts);

            im::DrawVert::write_set_image(
                &self.device,
                res.descriptors.gui_sets[0],
                &self.view,
                &self.sampler,
            )
        }
        res.command_buffer
            .bind_descriptor_sets(&self.pipeline, &res.descriptors.gui_sets, 0);

        // Upload vertex and index buffers
        res.gui_vertex_buffer.upload_arr(&vertex_data);
        res.gui_index_buffer.upload_arr(&index_data);
        // Bind vertex and index buffers
        res.command_buffer
            .bind_vertex_buffer(&res.gui_vertex_buffer);
        res.command_buffer.bind_index_buffer(&res.gui_index_buffer);

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

                        res.command_buffer.set_scissor(&scissor);

                        res.command_buffer.draw_indexed(
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
