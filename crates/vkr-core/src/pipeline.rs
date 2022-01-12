// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::any::Any;

use ash::vk;
use vkr_util::Handle;

use crate::{Frame, Model, Node};

pub trait Pipeline: Any {
    fn as_any(&self) -> &dyn Any;
    fn get_name(&self) -> &String;
    fn get_set_layouts(&self) -> &[vk::DescriptorSetLayout];
    fn get_layout(&self) -> vk::PipelineLayout;
    fn get_pipeline(&self) -> vk::Pipeline;
    fn bind(&self, frame: &mut Frame, model: &Model, node: Handle<Node>);
    fn draw(&self, frame: &mut Frame, model: &Model, node: Handle<Node>);
}
