// Copyright © 2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use nalgebra as na;
use vkr_core::Color;
use vkr_core::Material;

use crate::Buffer;

vkr_pipe::pipewriter!("../shader/main");

impl PipelineLine {
    fn bind_impl(&self, frame: &mut Frame, model: &Model, camera_node: Handle<Node>) {
        frame.res.command_buffer.bind_pipeline(self.get_pipeline());

        let width = frame.buffer.width as f32;
        let height = frame.buffer.height as f32;
        let viewport = vk::Viewport::builder()
            .width(width)
            .height(height)
            .max_depth(0.0)
            .min_depth(1.0)
            .build();
        frame.res.command_buffer.set_viewport(&viewport);

        let scissor = vk::Rect2D::builder()
            .extent(
                vk::Extent2D::builder()
                    .width(frame.buffer.width)
                    .height(frame.buffer.height)
                    .build(),
            )
            .build();
        frame.res.command_buffer.set_scissor(&scissor);

        let node = model.nodes.get(camera_node).unwrap();
        frame.current_view = node.trs.get_view_matrix();
        let camera = model.cameras.get(node.camera).unwrap();

        if let Some(sets) = frame
            .res
            .pipeline_cache
            .descriptors
            .node_sets
            .get(&(self.get_set_layouts()[1], camera_node))
        {
            frame
                .res
                .command_buffer
                .bind_descriptor_sets(self.get_layout(), sets, 1);

            // If there is a descriptor set, there must be a buffer, so we just unwrap
            // TODO: Optimize by uploading only if data has actually changed.
            let view_buffer = frame.res.view_buffers.get_mut_unchecked(&camera_node);
            view_buffer.upload(&frame.current_view);

            let proj_buffer = frame.res.proj_buffers.get_mut_unchecked(&node.camera);
            proj_buffer.upload(&camera.proj);
        } else {
            // TODO: Can I move this into the pipeline?
            // Allocate and write desc set for camera view
            // Camera set layout is at index 1 (use a constant?)
            let sets = frame
                .res
                .pipeline_cache
                .descriptors
                .allocate(&[self.get_set_layouts()[1]]);

            let view_buffer = frame
                .res
                .view_buffers
                .get_or_create_mut::<na::Matrix4<f32>>(&camera_node);
            let proj_buffer = frame
                .res
                .proj_buffers
                .get_or_create_mut::<na::Matrix4<f32>>(&node.camera);

            self.write_set_1(sets[0], &view_buffer, &proj_buffer);

            frame
                .res
                .command_buffer
                .bind_descriptor_sets(self.get_layout(), &sets, 1);

            frame
                .res
                .pipeline_cache
                .descriptors
                .node_sets
                .insert((self.get_set_layouts()[1], camera_node), sets);
        }
    }

    fn get_node_sets(
        &self,
        frame: &mut Frame,
        node_handle: Handle<Node>,
    ) -> Vec<vk::DescriptorSet> {
        if !frame
            .res
            .pipeline_cache
            .descriptors
            .node_sets
            .contains_key(&(self.set_layouts[0], node_handle))
        {
            let model_buffer = frame
                .res
                .model_buffers
                .get_or_create_mut::<na::Matrix4<f32>>(&node_handle);
            let model_view_buffer = frame
                .res
                .model_view_buffers
                .get_or_create_mut::<na::Matrix4<f32>>(&node_handle);

            // Allocate and write descriptors
            let sets = frame
                .res
                .pipeline_cache
                .descriptors
                .allocate(&[self.set_layouts[0]]);
            self.write_set_0(sets[0], &model_buffer, &model_view_buffer);

            frame
                .res
                .command_buffer
                .bind_descriptor_sets(self.layout, &sets, 0);

            frame
                .res
                .pipeline_cache
                .descriptors
                .node_sets
                .insert((self.set_layouts[0], node_handle), sets);
        }

        let sets = frame
            .res
            .pipeline_cache
            .descriptors
            .node_sets
            .get(&(self.set_layouts[0], node_handle))
            .unwrap();

        sets.clone()
    }

    fn draw_impl(&self, frame: &mut Frame, model: &Model, node_handle: Handle<Node>) {
        let cnode = model.nodes.get(node_handle).unwrap();

        let mesh = model.meshes.get(cnode.mesh);
        if mesh.is_none() {
            return ();
        }

        let mesh = mesh.unwrap();

        let sets = self.get_node_sets(frame, node_handle);
        frame
            .res
            .command_buffer
            .bind_descriptor_sets(self.layout, &sets, 0);

        let model_view_matrix = (frame.current_view * cnode.trs.get_matrix())
            .try_inverse()
            .unwrap()
            .transpose();

        // If there is a descriptor set, there must be a uniform buffer
        let model_buffer = frame.res.model_buffers.get_mut_unchecked(&node_handle);

        let node = model.nodes.get(node_handle).unwrap();
        model_buffer.upload(&node.trs.get_matrix());

        let model_view_buffer = frame.res.model_view_buffers.get_mut_unchecked(&node_handle);
        model_view_buffer.upload(&model_view_matrix);

        for hprimitive in &mesh.primitives {
            let primitive = model.primitives.get(*hprimitive).unwrap();

            frame
                .res
                .command_buffer
                .bind_vertex_buffer(&primitive.vertices);

            if let Some(indices) = &primitive.indices {
                // Draw indexed if primitive has indices
                frame.res.command_buffer.bind_index_buffer(indices);

                let index_count = indices.size as u32 / std::mem::size_of::<u16>() as u32;
                frame.res.command_buffer.draw_indexed(index_count, 0, 0);
            } else {
                // Draw without indices
                frame.res.command_buffer.draw(primitive.vertex_count);
            }
        }
    }
}

impl PipelineMain {
    fn bind_impl(&self, frame: &mut Frame, model: &Model, camera_node: Handle<Node>) {
        frame.res.command_buffer.bind_pipeline(self.get_pipeline());

        let width = frame.buffer.width as f32;
        let height = frame.buffer.height as f32;
        let viewport = vk::Viewport::builder()
            .width(width)
            .height(height)
            .max_depth(0.0)
            .min_depth(1.0)
            .build();
        frame.res.command_buffer.set_viewport(&viewport);

        let scissor = vk::Rect2D::builder()
            .extent(
                vk::Extent2D::builder()
                    .width(frame.buffer.width)
                    .height(frame.buffer.height)
                    .build(),
            )
            .build();
        frame.res.command_buffer.set_scissor(&scissor);

        let node = model.nodes.get(camera_node).unwrap();
        frame.current_view = node.trs.get_view_matrix();
        let camera = model.cameras.get(node.camera).unwrap();

        if let Some(sets) = frame
            .res
            .pipeline_cache
            .descriptors
            .node_sets
            .get(&(self.get_set_layouts()[1], camera_node))
        {
            frame
                .res
                .command_buffer
                .bind_descriptor_sets(self.get_layout(), sets, 1);

            // If there is a descriptor set, there must be a buffer, so we just unwrap
            // TODO: Optimize by uploading only if data has actually changed.
            let view_buffer = frame.res.view_buffers.get_mut_unchecked(&camera_node);
            view_buffer.upload(&frame.current_view);

            let proj_buffer = frame.res.proj_buffers.get_mut_unchecked(&node.camera);
            proj_buffer.upload(&camera.proj);
        } else {
            // TODO: Can I move this into the pipeline?
            // Allocate and write desc set for camera view
            // Camera set layout is at index 1 (use a constant?)
            let sets = frame
                .res
                .pipeline_cache
                .descriptors
                .allocate(&[self.get_set_layouts()[1]]);

            let view_buffer = frame
                .res
                .view_buffers
                .get_or_create_mut::<na::Matrix4<f32>>(&camera_node);
            let proj_buffer = frame
                .res
                .proj_buffers
                .get_or_create_mut::<na::Matrix4<f32>>(&node.camera);

            self.write_set_1(sets[0], &view_buffer, &proj_buffer);

            frame
                .res
                .command_buffer
                .bind_descriptor_sets(self.get_layout(), &sets, 1);

            frame
                .res
                .pipeline_cache
                .descriptors
                .node_sets
                .insert((self.get_set_layouts()[1], camera_node), sets);
        }
    }

    fn get_node_sets(
        &self,
        frame: &mut Frame,
        node_handle: Handle<Node>,
    ) -> Vec<vk::DescriptorSet> {
        if !frame
            .res
            .pipeline_cache
            .descriptors
            .node_sets
            .contains_key(&(self.set_layouts[0], node_handle))
        {
            let model_buffer = frame
                .res
                .model_buffers
                .get_or_create_mut::<na::Matrix4<f32>>(&node_handle);
            let model_view_buffer = frame
                .res
                .model_view_buffers
                .get_or_create_mut::<na::Matrix4<f32>>(&node_handle);

            // Allocate and write descriptors
            let sets = frame
                .res
                .pipeline_cache
                .descriptors
                .allocate(&[self.set_layouts[0]]);
            self.write_set_0(sets[0], &model_buffer, &model_view_buffer);

            frame
                .res
                .command_buffer
                .bind_descriptor_sets(self.layout, &sets, 0);

            frame
                .res
                .pipeline_cache
                .descriptors
                .node_sets
                .insert((self.set_layouts[0], node_handle), sets);
        }

        let sets = frame
            .res
            .pipeline_cache
            .descriptors
            .node_sets
            .get(&(self.set_layouts[0], node_handle))
            .unwrap();

        sets.clone()
    }

    fn get_material_sets(
        &self,
        frame: &mut Frame,
        model: &Model,
        material_handle: Handle<Material>,
    ) -> Vec<vk::DescriptorSet> {
        if !frame
            .res
            .pipeline_cache
            .descriptors
            .material_sets
            .contains_key(&(self.set_layouts[2], material_handle))
        {
            let material_color = frame
                .res
                .material_buffers
                .get_or_create_mut::<Color>(&material_handle);

            let material = model.materials.get(material_handle).unwrap();
            let material_albedo = model
                .textures
                .get(material.albedo)
                .unwrap_or(&frame.res.fallback.white_texture);

            // TODO Use enum for set layouts
            let sets = frame
                .res
                .pipeline_cache
                .descriptors
                .allocate(&[self.set_layouts[2]]);
            self.write_set_2(sets[0], material_color, material_albedo);

            frame
                .res
                .command_buffer
                .bind_descriptor_sets(self.layout, &sets, 2);

            frame
                .res
                .pipeline_cache
                .descriptors
                .material_sets
                .insert((self.set_layouts[2], material_handle), sets);
        }

        let sets = frame
            .res
            .pipeline_cache
            .descriptors
            .material_sets
            .get(&(self.set_layouts[2], material_handle))
            .unwrap();

        sets.clone()
    }

    fn draw_impl(&self, frame: &mut Frame, model: &Model, node_handle: Handle<Node>) {
        let cnode = model.nodes.get(node_handle).unwrap();

        let mesh = model.meshes.get(cnode.mesh);
        if mesh.is_none() {
            return ();
        }

        let mesh = mesh.unwrap();

        let sets = self.get_node_sets(frame, node_handle);
        frame
            .res
            .command_buffer
            .bind_descriptor_sets(self.layout, &sets, 0);

        let model_view_matrix = (frame.current_view * cnode.trs.get_matrix())
            .try_inverse()
            .unwrap()
            .transpose();

        // If there is a descriptor set, there must be a uniform buffer
        let model_buffer = frame.res.model_buffers.get_mut_unchecked(&node_handle);

        let node = model.nodes.get(node_handle).unwrap();
        model_buffer.upload(&node.trs.get_matrix());

        let model_view_buffer = frame.res.model_view_buffers.get_mut_unchecked(&node_handle);
        model_view_buffer.upload(&model_view_matrix);

        for hprimitive in &mesh.primitives {
            let primitive = model.primitives.get(*hprimitive).unwrap();

            let sets = self.get_material_sets(frame, model, primitive.material);
            frame
                .res
                .command_buffer
                .bind_descriptor_sets(self.layout, &sets, 2);

            // How about grouping by material?
            {
                let material_buffer = frame
                    .res
                    .material_buffers
                    .get_mut_unchecked(&primitive.material);

                let material = match model.materials.get(primitive.material) {
                    Some(m) => m,
                    None => &frame.res.fallback.white_material,
                };
                material_buffer.upload(material);
            }

            frame
                .res
                .command_buffer
                .bind_vertex_buffer(&primitive.vertices);

            if let Some(indices) = &primitive.indices {
                // Draw indexed if primitive has indices
                frame.res.command_buffer.bind_index_buffer(indices);

                let index_count = indices.size as u32 / std::mem::size_of::<u16>() as u32;
                frame.res.command_buffer.draw_indexed(index_count, 0, 0);
            } else {
                // Draw without indices
                frame.res.command_buffer.draw(primitive.vertex_count);
            }
        }
    }
}
