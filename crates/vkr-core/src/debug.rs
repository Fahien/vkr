// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{borrow::Cow, ffi::CStr};

use ash::{
    extensions::ext::DebugUtils,
    vk::{self, DebugUtilsMessengerEXT},
};

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
    vk::FALSE
}

pub struct Debug {
    loader: DebugUtils,
    messenger: DebugUtilsMessengerEXT,
}

impl Debug {
    pub fn new(entry: &ash::Entry, instance: &ash::Instance) -> Self {
        let loader = DebugUtils::new(entry, instance);

        // Debugging callback
        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(vk_debug));

        let messenger = unsafe {
            loader
                .create_debug_utils_messenger(&debug_info, None)
                .expect("Failed to create Vulkan debug callback")
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
