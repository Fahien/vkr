// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{cell::RefCell, convert::TryFrom, rc::Rc};

use super::*;

use ash::*;
use imgui as im;

struct Gui {
    // No need to cache this image
    image: Image,

    ctx: im::Context,
}

impl Gui {
    fn build_font(dev: &Dev, ctx: &mut im::Context) -> Image {
        let mut fonts = ctx.fonts();
        let font = fonts.build_rgba32_texture();

        let format = vk::Format::R8G8B8A8_UNORM;
        Image::from_data(dev, font.data, font.width, font.height, format)
    }

    fn new(dev: &Dev, width: u32, height: u32) -> Self {
        let mut ctx = im::Context::create();

        let io = ctx.io_mut();
        io.display_size[0] = width as f32;
        io.display_size[1] = height as f32;

        let image = Self::build_font(dev, &mut ctx);

        Self { image, ctx }
    }

    fn update(&mut self, command_buffer: &vk::CommandBuffer, delta: f32) {
        self.ctx.io_mut().delta_time = delta;
        let ui = self.ctx.frame();

        let data = ui.render();

        if data.draw_lists_count() == 0 {
            return ();
        }

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

                        let rect = vk::Rect2D::builder()
                            .offset(vk::Offset2D::builder().x(x).y(y).build())
                            .extent(vk::Extent2D::builder().width(width).height(height).build())
                            .build();

                        // set scissor

                        // draw indexed
                    }
                    _ => (),
                }
            }
        }
    }
}
