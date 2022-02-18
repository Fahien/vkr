// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use super::*;

use ash::vk;
use enum_ordinalize::Ordinalize;
use memoffset::offset_of;
use nalgebra as na;
use vkr_util::{Handle, Pack};

pub trait VertexInput {
    fn get_topology() -> vk::PrimitiveTopology {
        vk::PrimitiveTopology::TRIANGLE_LIST
    }
    fn get_bindings() -> Vec<vk::VertexInputBindingDescription>;
    fn get_attributes() -> Vec<vk::VertexInputAttributeDescription>;
}

pub struct VertexInputDescription {
    /// Is this the right place for the topology?
    pub topology: vk::PrimitiveTopology,
    pub attributes: Vec<vk::VertexInputAttributeDescription>,
    pub bindings: Vec<vk::VertexInputBindingDescription>,
}

impl VertexInputDescription {
    pub fn new<V: VertexInput>() -> Self {
        Self {
            topology: V::get_topology(),
            attributes: V::get_attributes(),
            bindings: V::get_bindings(),
        }
    }
}

#[derive(Debug, Clone, Copy, Ordinalize)]
pub enum VertexInputType {
    Point,
    Vertex,
}

impl VertexInputType {
    pub fn get_vertex_input(&self) -> VertexInputDescription {
        match self {
            VertexInputType::Point => VertexInputDescription::new::<Point>(),
            VertexInputType::Vertex => VertexInputDescription::new::<Vertex>(),
        }
    }
}

impl From<usize> for VertexInputType {
    fn from(n: usize) -> Self {
        VertexInputType::from_ordinal(n as i8)
            .expect(&format!("Failed to find vertex input for ordinal: {}", n))
    }
}

#[repr(C)]
pub struct Color {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Color { r, g, b, a }
    }

    pub fn white() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }
}

#[repr(C)]
pub struct Point {
    pub pos: na::Vector3<f32>,
    pub color: Color,
    pub normal: na::Vector3<f32>,
}

impl Point {
    pub fn new(pos: na::Vector3<f32>, color: Color) -> Self {
        Self {
            pos,
            color,
            normal: na::Vector3::zeros(),
        }
    }
}

impl VertexInput for Point {
    fn get_topology() -> vk::PrimitiveTopology {
        vk::PrimitiveTopology::LINE_STRIP
    }

    fn get_bindings() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Point>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()]
    }

    fn get_attributes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Point, pos) as u32)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(offset_of!(Point, color) as u32)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(2)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Point, normal) as u32)
                .build(),
        ]
    }
}

#[repr(C)]
pub struct Line {
    pub a: Point,
    pub b: Point,
}

impl Line {
    pub fn new(a: Point, b: Point) -> Line {
        Line { a, b }
    }
}

#[repr(C)]
pub struct Vertex {
    pub pos: na::Vector3<f32>,
    pub color: Color,
    pub normal: na::Vector3<f32>,
    pub uv: na::Vector2<f32>,
}

impl Vertex {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            pos: na::Vector3::new(x, y, z),
            color: Color::white(),
            // From the screen towards the viewer
            normal: na::Vector3::new(0.0, 0.0, 1.0),
            uv: na::Vector2::new(0.0, 0.0),
        }
    }
}

impl VertexInput for Vertex {
    fn get_bindings() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()]
    }

    fn get_attributes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            // position
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Vertex, pos) as u32)
                .build(),
            // color
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(offset_of!(Vertex, color) as u32)
                .build(),
            // normal
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(2)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Vertex, normal) as u32)
                .build(),
            // texture coordinates
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(3)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(offset_of!(Vertex, uv) as u32)
                .build(),
        ]
    }
}

/// Very simple vertex used for the presentation pass
#[repr(C)]
pub struct PresentVertex {
    /// The shader just needs x and y
    pub pos: na::Vector2<f32>,
}

impl PresentVertex {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            pos: na::Vector2::new(x, y),
        }
    }
}

impl VertexInput for PresentVertex {
    fn get_bindings() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()]
    }

    fn get_attributes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            // position
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(offset_of!(Self, pos) as u32)
                .build(),
        ]
    }
}

/// Transform
pub struct Trs {
    model: na::Isometry3<f32>,
    scale: na::Vector3<f32>,
}

impl Trs {
    pub fn new() -> Self {
        Self {
            model: na::Isometry3::identity(),
            scale: na::Vector3::new(1.0, 1.0, 1.0),
        }
    }

    pub fn get_matrix(&self) -> na::Matrix4<f32> {
        // @todo Verify it works as intended
        self.model
            .to_homogeneous()
            .append_nonuniform_scaling(&self.scale)
    }

    pub fn get_view_matrix(&self) -> na::Matrix4<f32> {
        let mut matrix = self.get_matrix();

        // Invert translation
        matrix.m14 = -matrix.m14;
        matrix.m24 = -matrix.m24;
        matrix.m34 = -matrix.m34;

        matrix
    }

    pub fn get_translation(&self) -> na::Vector3<f32> {
        self.model.translation.vector
    }

    pub fn get_rotation(&self) -> na::UnitQuaternion<f32> {
        self.model.rotation
    }

    /// Rotates around the origin
    pub fn set_rotation(&mut self, rot: &na::UnitQuaternion<f32>) {
        self.model.rotation = *rot;
    }

    pub fn translate(&mut self, trs: &na::Vector3<f32>) {
        let trs = na::Translation3::from(*trs);
        self.model.append_translation_mut(&trs);
    }

    /// Rotates around the current position
    pub fn rotate(&mut self, rot: &na::UnitQuaternion<f32>) {
        self.model.append_rotation_mut(rot);
    }

    pub fn scale(&mut self, scl: &na::Vector3<f32>) {
        self.scale = *scl;
    }
}

pub enum CameraType {
    ORTHOGRAPHIC,
    PERSPECTIVE,
}

pub struct Camera {
    pub id: usize,
    typ: CameraType,
    pub proj: na::Matrix4<f32>,

    pub viewport: vk::Viewport,
    pub scissor: vk::Rect2D,
}

impl Camera {
    fn new(typ: CameraType, proj: na::Matrix4<f32>) -> Self {
        Self {
            id: 0,
            typ,
            proj,
            viewport: vk::Viewport::default(),
            scissor: vk::Rect2D::default(),
        }
    }

    fn perspective_matrix(aspect: f32) -> na::Matrix4<f32> {
        let fovy = 3.14 / 4.0;
        let znear = 0.1;
        let zfar = 100.0;
        na::Perspective3::new(aspect, fovy, znear, zfar).to_homogeneous()
    }

    pub fn perspective(aspect: f32) -> Self {
        Self::new(CameraType::PERSPECTIVE, Camera::perspective_matrix(aspect))
    }

    fn orthographic_matrix(
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    ) -> na::Matrix4<f32> {
        let mid = na::Vector3::new(
            (left + right) / (right - left),
            (bottom + top) / (top - bottom),
            near / (near - far),
        );

        let scale = na::Vector3::new(
            2.0 / (right - left),
            2.0 / (top - bottom),
            1.0 / (near - far),
        );

        na::Matrix4::new(
            scale.x, 0.0, 0.0, mid.x, 0.0, -scale.y, 0.0, mid.y, 0.0, 0.0, scale.z, mid.z, 0.0,
            0.0, 0.0, 1.0,
        )
    }

    /// Parameters here are referred to the camera, where towards direction is positive.
    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        Self::new(
            CameraType::ORTHOGRAPHIC,
            Camera::orthographic_matrix(left, right, bottom, top, near, far),
        )
    }

    pub fn update(&mut self, width: u32, height: u32) {
        let aspect = width as f32 / height as f32;
        self.proj = match self.typ {
            CameraType::ORTHOGRAPHIC => {
                Camera::orthographic_matrix(-aspect, aspect, -1.0, 1.0, 0.1, 1.0)
            }

            CameraType::PERSPECTIVE => Camera::perspective_matrix(aspect),
        };

        self.viewport.width = width as f32;
        self.viewport.height = height as f32;
        self.viewport.min_depth = 1.0;

        self.scissor.extent.width = width;
        self.scissor.extent.height = height;
    }
}

type ScriptFn = Box<dyn Fn(f32, &mut Pack<Node>, Handle<Node>)>;

pub struct Script {
    pub update: ScriptFn,
}

impl Script {
    pub fn new(update: ScriptFn) -> Self {
        Self { update }
    }

    pub fn update(delta: f32, nodes: &mut Pack<Node>, scripts: &Pack<Script>, node: Handle<Node>) {
        let hscript = nodes.get(node).unwrap().script;
        if let Some(script) = scripts.get(hscript) {
            let func = &script.update;
            func(delta, nodes, node);
        }

        let children = nodes.get(node).unwrap().children.clone();
        for child in children {
            Self::update(delta, nodes, scripts, child);
        }
    }
}

pub struct Node {
    pub trs: Trs,
    pub children: Vec<Handle<Node>>,
    pub camera: Handle<Camera>,
    pub mesh: Handle<Mesh>,
    pub script: Handle<Script>,
}

impl Node {
    pub fn new() -> Self {
        Node {
            trs: Trs::new(),
            children: vec![],
            camera: Handle::none(),
            mesh: Handle::none(),
            script: Handle::none(),
        }
    }
}

pub struct Model {
    pub cameras: Pack<Camera>,
    pub nodes: Pack<Node>,
    pub images: Pack<Image>,
    pub views: Pack<ImageView>,
    pub samplers: Pack<Sampler>,
    pub textures: Pack<Texture>,
    pub materials: Pack<Material>,
    pub primitives: Pack<Primitive>,
    pub meshes: Pack<Mesh>,
    pub scripts: Pack<Script>,
}

impl Model {
    pub fn new() -> Self {
        Self {
            cameras: Pack::new(),
            nodes: Pack::new(),
            images: Pack::new(),
            views: Pack::new(),
            samplers: Pack::new(),
            textures: Pack::new(),
            materials: Pack::new(),
            primitives: Pack::new(),
            meshes: Pack::new(),
            scripts: Pack::new(),
        }
    }
}
