// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{
    ffi::{c_char, CString},
    marker::PhantomData,
};

use ash::{extensions::ext::DebugUtils, vk};

#[cfg(feature = "win")]
use crate::win::Win;

pub struct CtxBuilder<'w> {
    debug: bool,
    #[cfg(feature = "win")]
    win: Option<&'w Win>,
    _phantom: PhantomData<&'w u8>,
}

impl<'w> Default for CtxBuilder<'w> {
    fn default() -> Self {
        Self::new()
    }
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

        #[cfg(target_os = "macos")]
        let portabilty_enumeration_name = CString::new("VK_KHR_portability_enumeration").unwrap();
        #[cfg(target_os = "macos")]
        extension_names.push(portabilty_enumeration_name.as_ptr());

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

pub struct Ctx {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
}

impl Ctx {
    pub fn builder<'w>() -> CtxBuilder<'w> {
        CtxBuilder::new()
    }

    pub fn new(extension_names: &[*const c_char]) -> Self {
        let layers = [CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
        let layer_names: Vec<*const i8> = layers.iter().map(|name| name.as_ptr()).collect();

        let entry = unsafe { ash::Entry::load() }.expect("Failed to create ash entry");
        let app_info = vk::ApplicationInfo {
            p_application_name: "Test" as *const str as _,
            api_version: vk::make_api_version(0, 1, 2, 0),
            ..Default::default()
        };
        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(extension_names)
            .enabled_layer_names(&layer_names)
            .flags(vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR);
        let instance = unsafe { entry.create_instance(&create_info, None) }
            .expect("Failed to create Vulkan instance");

        Self { entry, instance }
    }
}

impl Drop for Ctx {
    fn drop(&mut self) {
        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}
