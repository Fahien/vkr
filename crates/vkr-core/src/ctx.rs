// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{
    borrow::Cow,
    ffi::{CStr, CString},
    marker::PhantomData,
    os::raw::c_char,
};

use ash::{extensions::ext::DebugUtils, vk};

#[cfg(feature = "win")]
use super::Win;

pub struct CtxBuilder<'w> {
    debug: bool,
    #[cfg(feature = "win")]
    win: Option<&'w Win>,
    _phantom: PhantomData<&'w u8>,
}

impl<'w> CtxBuilder<'w> {
    pub fn new() -> Self {
        Self {
            debug: true,
            #[cfg(feature = "win")]
            win: None,
            _phantom: PhantomData,
        }
    }

    pub fn debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    #[cfg(feature = "win")]
    pub fn win(mut self, win: &'w Win) -> Self {
        self.win = Some(win);
        self
    }

    pub fn build(self) -> Ctx {
        let mut extension_names = vec![];

        if self.debug {
            extension_names.push(DebugUtils::name().as_ptr());
        }

        #[cfg(feature = "win")]
        if let Some(win) = self.win {
            let extensions = win
                .window
                .vulkan_instance_extensions()
                .expect("Failed to get SDL vulkan extensions");
            for ext in extensions.iter() {
                extension_names.push(ext.as_ptr() as _);
            }
        }

        Ctx::new(&extension_names)
    }
}

unsafe extern "system" fn vk_debug(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    if message_severity.intersects(
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
            | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING,
    ) {
        let callback_data = *p_callback_data;
        let message = if callback_data.p_message.is_null() {
            Cow::from("No message")
        } else {
            CStr::from_ptr(callback_data.p_message).to_string_lossy()
        };
        // TODO: M1 complains about this but..
        if !message.contains("vulkan_memory_model") && !message.contains("VulkanMemoryModel") {
            eprintln!("{:?}", message);
        }
    }
    ash::vk::FALSE
}

pub struct Debug {
    loader: DebugUtils,
    messenger: vk::DebugUtilsMessengerEXT,
}

impl Debug {
    fn new(entry: &ash::Entry, instance: &ash::Instance) -> Self {
        let loader = DebugUtils::new(entry, instance);
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

pub struct Ctx {
    debug: Option<Debug>,
    pub entry: ash::Entry,
    pub instance: ash::Instance,
}

impl Ctx {
    pub fn builder<'w>() -> CtxBuilder<'w> {
        CtxBuilder::new()
    }

    pub fn new(extension_names: &[*const c_char]) -> Self {
        let is_debug = extension_names.contains(&DebugUtils::name().as_ptr());

        let mut layers = vec![];
        if is_debug {
            layers.push(CString::new("VK_LAYER_KHRONOS_validation").unwrap());
        }
        let layer_names: Vec<*const i8> = layers.iter().map(|name| name.as_ptr()).collect();

        let entry = unsafe { ash::Entry::new() }.expect("Failed to create ash entry");
        let app_info = ash::vk::ApplicationInfo {
            p_application_name: "Test" as *const str as _,
            api_version: ash::vk::make_api_version(0, 1, 2, 0),
            ..Default::default()
        };
        let create_info = ash::vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(extension_names)
            .enabled_layer_names(&layer_names);
        let instance = unsafe { entry.create_instance(&create_info, None) }
            .expect("Failed to create Vulkan instance");

        let debug = if is_debug {
            println!("Enabling Vulkan debug utils");
            Some(Debug::new(&entry, &instance))
        } else {
            None
        };

        Self {
            debug,
            entry,
            instance,
        }
    }
}

impl Drop for Ctx {
    fn drop(&mut self) {
        drop(self.debug.take());
        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}
