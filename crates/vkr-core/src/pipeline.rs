// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use ash::vk::{self, VertexInputAttributeDescription, VertexInputBindingDescription};

pub trait Pipeline {
    fn get_pipeline(&self) -> vk::Pipeline;
    fn get_layout(&self) -> vk::PipelineLayout;
    fn get_set_layout(&self) -> vk::DescriptorSetLayout;
}

pub trait VertexInput {
    fn get_attributes() -> Vec<VertexInputAttributeDescription>;
    fn get_bindings() -> Vec<VertexInputBindingDescription>;
}
