// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use quote::{quote, ToTokens};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ShaderType {
    Vertex,
    Fragment,
}

impl ToTokens for ShaderType {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            ShaderType::Vertex => tokens.extend(quote! { vk::ShaderStageFlags::VERTEX }),
            ShaderType::Fragment => tokens.extend(quote! { vk::ShaderStageFlags::FRAGMENT }),
        }
    }
}

pub struct Uniform {
    pub name: syn::Ident,
    /// Type of the argument
    pub ident: syn::Ident,
    pub descriptor_set: u32,
    pub binding: u32,
    pub stage: ShaderType,
}

impl Uniform {
    pub fn new(
        name: syn::Ident,
        ident: syn::Ident,
        descriptor_set: u32,
        binding: u32,
        stage: ShaderType,
    ) -> Self {
        Self {
            name,
            ident,
            descriptor_set,
            binding,
            stage,
        }
    }

    pub fn get_descriptor_type(&self) -> proc_macro2::TokenStream {
        match self.ident.to_string().as_str() {
            "Vec2" | "Vec3" | "Vec4" | "Mat3" | "Mat4" => {
                quote! { vk::DescriptorType::UNIFORM_BUFFER }
            }
            "SampledImage" => quote! { vk::DescriptorType::COMBINED_IMAGE_SAMPLER },
            unknown => todo!(
                "Failed to get descriptor type for {}: {}:{}",
                unknown,
                file!(),
                line!()
            ),
        }
    }

    pub fn get_write_set_type(&self) -> proc_macro2::TokenStream {
        match self.ident.to_string().as_str() {
            "Vec2" | "Vec3" | "Vec4" | "Mat3" | "Mat4" => quote! { &Buffer },
            "SampledImage" => quote! { &Texture },
            unknown => todo!(
                "Failed to get descriptor type for {}: {}:{}",
                unknown,
                file!(),
                line!()
            ),
        }
    }

    pub fn get_range(&self) -> Option<usize> {
        let f32_size = std::mem::size_of::<f32>();

        match self.ident.to_string().as_str() {
            "Vec2" => Some(f32_size * 2),
            "Vec3" => Some(f32_size * 3),
            "Vec4" => Some(f32_size * 4),
            "Mat3" => Some(f32_size * 9),
            "Mat4" => Some(f32_size * 16),
            _ => None,
        }
    }

    /// Returns a token stream useful for constructing a `WriteDescriptorSet`.
    /// According to the type of the uniform, this will return a buffer_info call
    /// or an image_info call, complete with the argument.
    pub fn get_info(&self) -> proc_macro2::TokenStream {
        let name = &self.name;

        match self.ident.to_string().as_str() {
            "Vec2" | "Vec3" | "Vec4" | "Mat3" | "Mat4" => {
                let range = self.get_range().unwrap();
                quote! { .buffer_info(
                    &[
                        vk::DescriptorBufferInfo::builder()
                            .range(#range as vk::DeviceSize)
                            .buffer(#name.buffer)
                            .build()
                    ]
                ) }
            }
            "SampledImage" => quote! { .image_info(
                &[
                    vk::DescriptorImageInfo::builder()
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .image_view(#name.view)
                        .sampler(#name.sampler)
                        .build()
                ]
            ) },
            unknown => todo!(
                "Failed to get descriptor type for {}: {}:{}",
                unknown,
                file!(),
                line!()
            ),
        }
    }
}

pub struct PipelineBuilder {
    pub name: String,
    pub arg_types: Vec<syn::Ident>,
    pub uniforms: Vec<Uniform>,
}

impl PipelineBuilder {
    pub fn new() -> Self {
        Self {
            name: String::default(),
            arg_types: Vec::default(),
            uniforms: Vec::default(),
        }
    }

    pub fn name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn arg_types(&mut self, arg_types: Vec<syn::Ident>) {
        self.arg_types = arg_types;
    }

    pub fn add_uniforms(&mut self, uniforms: Vec<Uniform>) {
        self.uniforms.extend(uniforms);
    }

    pub fn build(self) -> Pipeline {
        Pipeline::new(self.name, self.arg_types, self.uniforms)
    }
}

pub struct Pipeline {
    pub name: String,
    pub arg_types: Vec<syn::Ident>,
    pub uniforms: Vec<Uniform>,
}

impl Pipeline {
    pub fn builder() -> PipelineBuilder {
        PipelineBuilder::new()
    }

    pub fn new(name: String, arg_types: Vec<syn::Ident>, uniforms: Vec<Uniform>) -> Self {
        Self {
            name,
            arg_types,
            uniforms,
        }
    }
}
