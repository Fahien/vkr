// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{cell::RefCell, rc::Rc};

use super::*;

pub struct Primitive {
    pub vertex_count: u32,
    pub vertices: Buffer,
    pub indices: Option<Buffer>,
}

impl Primitive {
    pub fn new<T>(allocator: &Rc<RefCell<vk_mem::Allocator>>, vv: &[T]) -> Self {
        let vertex_count = vv.len() as u32;

        let mut vertices = Buffer::new::<T>(allocator, ash::vk::BufferUsageFlags::VERTEX_BUFFER);
        vertices.upload_arr(vv);

        Self {
            vertex_count,
            vertices,
            indices: None,
        }
    }

    /// Returns a new primitive quad with side length 1 centered at the origin
    pub fn quad(allocator: &Rc<RefCell<vk_mem::Allocator>>) -> Self {
        let vertices = vec![
            Vertex {
                pos: na::Vector3::new(-0.5, -0.5, 0.0),
                color: Color::white(),
                uv: na::Vector2::new(0.0, 1.0),
            },
            Vertex {
                pos: na::Vector3::new(0.5, -0.5, 0.0),
                color: Color::white(),
                uv: na::Vector2::new(1.0, 1.0),
            },
            Vertex {
                pos: na::Vector3::new(0.5, 0.5, 0.0),
                color: Color::white(),
                uv: na::Vector2::new(1.0, 0.0),
            },
            Vertex {
                pos: na::Vector3::new(-0.5, 0.5, 0.0),
                color: Color::white(),
                uv: na::Vector2::new(0.0, 0.0),
            },
        ];
        let indices = vec![0, 1, 2, 2, 3, 0];

        let mut ret = Self::new(allocator, &vertices);
        ret.set_indices(&indices);
        ret
    }

    pub fn set_indices(&mut self, ii: &[u16]) {
        let mut indices = Buffer::new::<u16>(
            &self.vertices.allocator,
            ash::vk::BufferUsageFlags::INDEX_BUFFER,
        );
        indices.upload_arr(ii);
        self.indices = Some(indices);
    }
}
