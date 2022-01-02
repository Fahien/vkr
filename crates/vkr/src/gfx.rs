// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::{extensions::ext::DebugUtils, vk};
use std::{borrow::Cow, ffi::CStr};

use crate::*;

unsafe extern "system" fn vk_debug(
    _message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message = if callback_data.p_message.is_null() {
        Cow::from("No message")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };
    eprintln!("{:?}", message);
    ash::vk::FALSE
}

pub struct Debug {
    loader: DebugUtils,
    messenger: vk::DebugUtilsMessengerEXT,
}

impl Debug {
    fn new(ctx: &Ctx) -> Self {
        let loader = DebugUtils::new(&ctx.entry, &ctx.instance);
        let messenger = unsafe {
            loader
                .create_debug_utils_messenger(
                    &vk::DebugUtilsMessengerCreateInfoEXT::builder()
                        .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
                        .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
                        .pfn_user_callback(Some(vk_debug)),
                    None,
                )
                .expect("Failed to create Vulkan debug messenger")
        };

        Self { loader, messenger }
    }
}

impl Drop for Debug {
    fn drop(&mut self) {
        unsafe {
            self.loader
                .destroy_debug_utils_messenger(self.messenger, None);
        }
    }
}

pub struct Vkr {
    pub pipelines: DefaultPipelines,
    pub gui: Gui,
    pub sfs: SwapchainFrames, // Use box of frames?
    pub pass: Pass,           // How about multiple passes?
    pub dev: Dev,
    pub surface: Surface,
    pub debug: Debug,
    pub ctx: Ctx,
    pub win: Option<Win>,
    pub resized: bool, // Whether the window has been resized or not
    pub timer: Timer,
}

impl Vkr {
    pub fn new(win: Win) -> Self {
        let timer = Timer::new();

        let (width, height) = win.window.drawable_size();

        let ctx = Ctx::builder().win(&win).build();
        let debug = Debug::new(&ctx);

        let surface = Surface::new(&win, &ctx);
        let mut dev = Dev::new(&ctx, Some(&surface));

        let pass = Pass::new(&mut dev);
        let sfs = SwapchainFrames::new(&ctx, &surface, &mut dev, width, height, &pass);

        let gui = Gui::new(&win, &dev, &pass);

        let pipelines = DefaultPipelines::new(&dev, &pass, width, height);

        Self {
            pipelines,
            gui,
            sfs,
            pass,
            dev,
            surface,
            debug,
            ctx,
            win: Some(win),
            resized: false,
            timer,
        }
    }

    pub fn handle_events(&mut self) -> bool {
        let win = self.win.as_mut().unwrap();

        self.resized = false;

        // Handle events
        for event in win.events.poll_iter() {
            match event {
                sdl::event::Event::Window {
                    win_event: sdl::event::WindowEvent::Resized(_, _),
                    ..
                }
                | sdl::event::Event::Window {
                    win_event: sdl::event::WindowEvent::SizeChanged(_, _),
                    ..
                } => {
                    self.resized = true;
                }
                sdl::event::Event::Quit { .. }
                | sdl::event::Event::KeyDown {
                    keycode: Some(sdl::keyboard::Keycode::Escape),
                    ..
                } => return false,
                sdl::event::Event::MouseButtonDown { mouse_btn, .. } => {
                    if mouse_btn != sdl::mouse::MouseButton::Unknown {
                        let index = match mouse_btn {
                            sdl::mouse::MouseButton::Left => 0,
                            sdl::mouse::MouseButton::Right => 1,
                            sdl::mouse::MouseButton::Middle => 2,
                            sdl::mouse::MouseButton::X1 => 3,
                            sdl::mouse::MouseButton::X2 => 4,
                            sdl::mouse::MouseButton::Unknown => unreachable!(),
                        };
                        self.gui.mouse_down[index] = true;
                    }
                }
                sdl::event::Event::TextInput { ref text, .. } => {
                    for chr in text.chars() {
                        self.gui.ctx.io_mut().add_input_character(chr);
                    }
                }
                sdl::event::Event::KeyDown {
                    keycode: Some(code),
                    ..
                } => {
                    let index = code as usize;
                    let io = self.gui.ctx.io_mut();
                    if index < io.keys_down.len() {
                        io.keys_down[code as usize] = true;
                    }
                }
                sdl::event::Event::KeyUp {
                    keycode: Some(code),
                    ..
                } => {
                    let index = code as usize;
                    let keys = &mut self.gui.ctx.io_mut().keys_down;
                    if index < keys.len() {
                        keys[code as usize] = false;
                    }
                }
                _ => {}
            }
        }

        self.gui.set_mouse_state(&win.events.mouse_state());

        true
    }

    /// Returns a frame if available. When not available None is returned and drawing should be skipped
    /// TODO: Another option would be to wait until the frame is available and then return it.
    pub fn begin_frame(&mut self) -> Option<Frame> {
        let win = self.win.as_ref().unwrap();

        if self.resized {
            self.gui.set_drawable_size(win);
            self.sfs.recreate(win, &self.surface, &self.dev, &self.pass);
        }

        match self
            .sfs
            .next_frame(win, &self.surface, &self.dev, &self.pass)
        {
            Some(frame) => {
                let (width, height) = self.win.as_mut().unwrap().window.drawable_size();
                frame.begin(&self.pass, width, height);
                Some(frame)
            }
            None => None,
        }
    }

    /// Finish rendering a 3D scene and starts next (present) subpass
    pub fn end_scene(&mut self, frame: &mut Frame) {
        frame.res.command_buffer.next_subpass();

        let present_pipeline = self.pipelines.get_presentation();
        frame
            .res
            .command_buffer
            .bind_pipeline(present_pipeline.graphics);

        if frame.res.descriptors.present_sets.is_empty() {
            frame.res.descriptors.present_sets = frame
                .res
                .descriptors
                .allocate(&present_pipeline.set_layouts);
            PresentVertex::write_set(
                &self.dev.device,
                frame.res.descriptors.present_sets[0],
                &frame.buffer.albedo_view,
                &frame.buffer.normal_view,
                &frame.res.fallback.white_sampler,
            );
        }
        frame.res.command_buffer.bind_descriptor_sets(
            present_pipeline.layout,
            &frame.res.descriptors.present_sets,
            0,
        );
        frame
            .res
            .command_buffer
            .bind_vertex_buffer(&frame.res.fallback.present_buffer);
        frame.res.command_buffer.draw(3);
    }

    pub fn end_frame(&mut self, frame: Frame) {
        frame.end();

        self.sfs.present(
            frame,
            &self.win.as_ref().unwrap(),
            &self.surface,
            &self.dev,
            &self.pass,
        );
    }

    /// This function can be called before binding the camera to update it.
    /// Internally it checks if a resize happened before doing anything.
    pub fn update_camera(&self, model: &mut Model, camera_node: Handle<Node>) {
        if self.resized {
            let camera_node = model.nodes.get(camera_node).unwrap();
            let camera = model.cameras.get_mut(camera_node.camera).unwrap();
            if let Some(win) = self.win.as_ref() {
                camera.update(win);
            }
        }
    }
}

impl Drop for Vkr {
    fn drop(&mut self) {
        // Make sure device is idle before releasing Vulkan resources
        self.dev.wait();
    }
}
