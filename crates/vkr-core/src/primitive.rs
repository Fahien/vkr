// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{cell::RefCell, rc::Rc};

use ash::vk;

use super::*;

pub struct Primitive {
    pub vertex_count: u32,
    pub vertices: Buffer,
    pub indices: Option<Buffer>,
}

impl Primitive {
    pub fn new<T>(allocator: &Rc<RefCell<vk_mem::Allocator>>, vv: &[T]) -> Self {
        let vertex_count = vv.len() as u32;

        let mut vertices = Buffer::new::<T>(allocator, vk::BufferUsageFlags::VERTEX_BUFFER);
        vertices.upload_arr(vv);

        Self {
            vertex_count,
            vertices,
            indices: None,
        }
    }

    pub fn set_indices(&mut self, ii: &[u16]) {
        let mut indices =
            Buffer::new::<u16>(&self.vertices.allocator, vk::BufferUsageFlags::INDEX_BUFFER);
        indices.upload_arr(ii);
        self.indices = Some(indices);
    }
}
