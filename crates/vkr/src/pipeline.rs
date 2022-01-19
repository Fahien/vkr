// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use enum_ordinalize::*;
use variant_count::*;

use crate::VertexInput;

vkr_pipe::pipewriter!("crates/shader/present");


#[derive(Debug, Clone, Copy, VariantCount, Ordinalize)]
pub enum Pipelines {
    LINE,
    PRESENT,
    NORMAL,
    MAIN,
}

impl PipelineNormal {
    fn bind_impl(&self, _frame: &mut Frame, _model: &Model, _node: Handle<Node>) {}
    fn draw_impl(&self, _frame: &mut Frame, _model: &Model, _node: Handle<Node>) {}
}

impl PipelinePresent {
    fn bind_impl(&self, _frame: &mut Frame, _model: &Model, _node: Handle<Node>) {}

    fn draw_impl(&self, frame: &mut Frame, _model: &Model, _node: Handle<Node>) {
        frame.res.command_buffer.bind_pipeline(self.pipeline);

        let pipeline_layout = self.get_layout();
        let set_layouts = self.get_set_layouts().clone();
        if frame.res.pipeline_cache.descriptors.present_sets.is_empty() {
            frame.res.pipeline_cache.descriptors.present_sets =
                frame.res.pipeline_cache.descriptors.allocate(&set_layouts);

            let albedo_texture = Texture::new(
                frame.buffer.albedo_view.view,
                frame.res.fallback.white_sampler.sampler,
            );
            let normal_texture = Texture::new(
                frame.buffer.normal_view.view,
                frame.res.fallback.white_sampler.sampler,
            );

            self.write_set_0(
                frame.res.pipeline_cache.descriptors.present_sets[0],
                &albedo_texture,
                &normal_texture,
            );
        }

        frame.res.command_buffer.bind_descriptor_sets(
            pipeline_layout,
            &frame.res.pipeline_cache.descriptors.present_sets,
            0,
        );
        frame
            .res
            .command_buffer
            .bind_vertex_buffer(&frame.res.fallback.present_buffer);

        frame.res.command_buffer.draw(3)
    }
}
