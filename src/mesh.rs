// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::*;
use std::{cell::RefCell, rc::Rc};

use super::*;

pub struct Primitive {
    pub vertex_count: u32,
    pub vertices: Buffer,
    pub indices: Option<Buffer>,
    pub material: Handle<Material>,
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
            material: Handle::none(), // default material
        }
    }

    /// Returns a new primitive quad with side length 1 centered at the origin
    pub fn quad(allocator: &Rc<RefCell<vk_mem::Allocator>>, uv_scale: [f32; 2]) -> Self {
        let vertices = vec![
            Vertex {
                pos: na::Vector3::new(-0.5, -0.5, 0.0),
                color: Color::white(),
                normal: na::Vector3::new(0.0, 0.0, 1.0),
                uv: na::Vector2::new(0.0 * uv_scale[0], 1.0 * uv_scale[1]),
            },
            Vertex {
                pos: na::Vector3::new(0.5, -0.5, 0.0),
                color: Color::white(),
                normal: na::Vector3::new(0.0, 0.0, 1.0),
                uv: na::Vector2::new(1.0 * uv_scale[0], 1.0 * uv_scale[1]),
            },
            Vertex {
                pos: na::Vector3::new(0.5, 0.5, 0.0),
                color: Color::white(),
                normal: na::Vector3::new(0.0, 0.0, 1.0),
                uv: na::Vector2::new(1.0 * uv_scale[0], 0.0 * uv_scale[1]),
            },
            Vertex {
                pos: na::Vector3::new(-0.5, 0.5, 0.0),
                color: Color::white(),
                normal: na::Vector3::new(0.0, 0.0, 1.0),
                uv: na::Vector2::new(0.0 * uv_scale[0], 0.0 * uv_scale[1]),
            },
        ];
        let indices = vec![0, 1, 2, 2, 3, 0];

        let mut ret = Self::new(allocator, &vertices);
        ret.set_indices(&indices);
        ret
    }

    pub fn set_indices(&mut self, ii: &[u16]) {
        let mut indices =
            Buffer::new::<u16>(&self.vertices.allocator, vk::BufferUsageFlags::INDEX_BUFFER);
        indices.upload_arr(ii);
        self.indices = Some(indices);
    }

    pub fn cube(allocator: &Rc<RefCell<vk_mem::Allocator>>) -> Self {
        let vertices = vec![
            // Front
            Vertex {
                pos: na::Vector3::new(-0.5, -0.5, 0.5),
                color: Color::white(),
                normal: na::Vector3::new(0.0, 0.0, 1.0),
                uv: na::Vector2::new(0.0, 0.0),
            },
            Vertex {
                pos: na::Vector3::new(0.5, -0.5, 0.5),
                color: Color::white(),
                normal: na::Vector3::new(0.0, 0.0, 1.0),
                uv: na::Vector2::new(1.0, 0.0),
            },
            Vertex {
                pos: na::Vector3::new(0.5, 0.5, 0.5),
                color: Color::white(),
                normal: na::Vector3::new(0.0, 0.0, 1.0),
                uv: na::Vector2::new(1.0, 1.0),
            },
            Vertex {
                pos: na::Vector3::new(-0.5, 0.5, 0.5),
                color: Color::white(),
                normal: na::Vector3::new(0.0, 0.0, 1.0),
                uv: na::Vector2::new(0.0, 1.0),
            },
            // Right
            Vertex {
                pos: na::Vector3::new(0.5, -0.5, 0.5),
                color: Color::white(),
                normal: na::Vector3::new(1.0, 0.0, 0.0),
                uv: na::Vector2::new(0.0, 0.0),
            },
            Vertex {
                pos: na::Vector3::new(0.5, -0.5, -0.5),
                color: Color::white(),
                normal: na::Vector3::new(1.0, 0.0, 0.0),
                uv: na::Vector2::new(1.0, 0.0),
            },
            Vertex {
                pos: na::Vector3::new(0.5, 0.5, -0.5),
                color: Color::white(),
                normal: na::Vector3::new(1.0, 0.0, 0.0),
                uv: na::Vector2::new(1.0, 1.0),
            },
            Vertex {
                pos: na::Vector3::new(0.5, 0.5, 0.5),
                color: Color::white(),
                normal: na::Vector3::new(1.0, 0.0, 0.0),
                uv: na::Vector2::new(0.0, 1.0),
            },
            // Back
            Vertex {
                pos: na::Vector3::new(0.5, -0.5, -0.5),
                color: Color::white(),
                normal: na::Vector3::new(0.0, 0.0, -1.0),
                uv: na::Vector2::new(0.0, 0.0),
            },
            Vertex {
                pos: na::Vector3::new(-0.5, -0.5, -0.5),
                color: Color::white(),
                normal: na::Vector3::new(0.0, 0.0, -1.0),
                uv: na::Vector2::new(1.0, 0.0),
            },
            Vertex {
                pos: na::Vector3::new(-0.5, 0.5, -0.5),
                color: Color::white(),
                normal: na::Vector3::new(0.0, 0.0, -1.0),
                uv: na::Vector2::new(1.0, 1.0),
            },
            Vertex {
                pos: na::Vector3::new(0.5, 0.5, -0.5),
                color: Color::white(),
                normal: na::Vector3::new(0.0, 0.0, -1.0),
                uv: na::Vector2::new(0.0, 1.0),
            },
            // Left
            Vertex {
                pos: na::Vector3::new(-0.5, -0.5, -0.5),
                color: Color::white(),
                normal: na::Vector3::new(-1.0, 0.0, 0.0),
                uv: na::Vector2::new(0.0, 0.0),
            },
            Vertex {
                pos: na::Vector3::new(-0.5, -0.5, 0.5),
                color: Color::white(),
                normal: na::Vector3::new(-1.0, 0.0, 0.0),
                uv: na::Vector2::new(1.0, 0.0),
            },
            Vertex {
                pos: na::Vector3::new(-0.5, 0.5, 0.5),
                color: Color::white(),
                normal: na::Vector3::new(-1.0, 0.0, 0.0),
                uv: na::Vector2::new(1.0, 1.0),
            },
            Vertex {
                pos: na::Vector3::new(-0.5, 0.5, -0.5),
                color: Color::white(),
                normal: na::Vector3::new(-1.0, 0.0, 0.0),
                uv: na::Vector2::new(0.0, 1.0),
            },
            // Top
            Vertex {
                pos: na::Vector3::new(-0.5, 0.5, 0.5),
                color: Color::white(),
                normal: na::Vector3::new(0.0, 1.0, 0.0),
                uv: na::Vector2::new(0.0, 0.0),
            },
            Vertex {
                pos: na::Vector3::new(0.5, 0.5, 0.5),
                color: Color::white(),
                normal: na::Vector3::new(0.0, 1.0, 0.0),
                uv: na::Vector2::new(1.0, 0.0),
            },
            Vertex {
                pos: na::Vector3::new(0.5, 0.5, -0.5),
                color: Color::white(),
                normal: na::Vector3::new(0.0, 1.0, 0.0),
                uv: na::Vector2::new(1.0, 1.0),
            },
            Vertex {
                pos: na::Vector3::new(-0.5, 0.5, -0.5),
                color: Color::white(),
                normal: na::Vector3::new(0.0, 1.0, 0.0),
                uv: na::Vector2::new(0.0, 1.0),
            },
            // Bottom
            Vertex {
                pos: na::Vector3::new(-0.5, -0.5, -0.5),
                color: Color::white(),
                normal: na::Vector3::new(0.0, -1.0, 0.0),
                uv: na::Vector2::new(0.0, 0.0),
            },
            Vertex {
                pos: na::Vector3::new(0.5, -0.5, -0.5),
                color: Color::white(),
                normal: na::Vector3::new(0.0, -1.0, 0.0),
                uv: na::Vector2::new(1.0, 0.0),
            },
            Vertex {
                pos: na::Vector3::new(0.5, -0.5, 0.5),
                color: Color::white(),
                normal: na::Vector3::new(0.0, -1.0, 0.0),
                uv: na::Vector2::new(1.0, 1.0),
            },
            Vertex {
                pos: na::Vector3::new(-0.5, -0.5, 0.5),
                color: Color::white(),
                normal: na::Vector3::new(0.0, -1.0, 0.0),
                uv: na::Vector2::new(0.0, 1.0),
            },
        ];

        let indices: Vec<u16> = vec![
            0, 1, 2, 0, 2, 3, // front face
            4, 5, 6, 4, 6, 7, // right
            8, 9, 10, 8, 10, 11, // back
            12, 13, 14, 12, 14, 15, // left
            16, 17, 18, 16, 18, 19, // top
            20, 21, 22, 20, 22, 23, // bottom
        ];

        let mut ret = Self::new(allocator, &vertices);
        ret.set_indices(&indices);
        ret
    }
}

pub struct Mesh {
    pub primitives: Vec<Handle<Primitive>>,
    pub texture: Handle<Texture>,
}

impl Mesh {
    pub fn new(primitives: Vec<Handle<Primitive>>, texture: Handle<Texture>) -> Self {
        Self {
            primitives,
            texture,
        }
    }
}
