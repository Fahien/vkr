// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use crate::{shader::UniformType, util::inner_value};

/// This represents the spirv attribute
pub struct Spirv {
    pub uniform_type: UniformType,
    pub descriptor_set: Option<u32>,
    pub binding: Option<u32>,
}

impl Spirv {
    fn get_u32(name_value: &syn::MetaNameValue) -> Option<u32> {
        let value = inner_value!(&name_value.lit, syn::Lit::Int(i) => i);
        if let Some(value) = value {
            value.base10_parse().ok()
        } else {
            None
        }
    }

    /// This handles the case where we have a #[spirv(a, b, ..)] with multiple values
    fn parse_meta_list(list: &syn::MetaList) -> Option<Self> {
        let mut ret = Self::default();

        for nested in &list.nested {
            if let syn::NestedMeta::Meta(meta) = nested {
                match meta {
                    syn::Meta::NameValue(name_value) => {
                        if name_value.path.is_ident("descriptor_set") {
                            ret.descriptor_set = Self::get_u32(name_value);
                        }
                        if name_value.path.is_ident("binding") {
                            ret.binding = Self::get_u32(name_value);
                        }
                        if name_value.path.is_ident("input_attachment_index") {
                            ret.uniform_type = UniformType::InputAttachment
                        }
                    }
                    syn::Meta::Path(p) => {
                        if p.is_ident("push_constant") {
                            ret.uniform_type = UniformType::PushConstant;
                        } else if p.is_ident("uniform") {
                            ret.uniform_type = UniformType::UniformBuffer;
                        }
                    }
                    _ => (),
                }
            }
        }

        Some(ret)
    }

    pub fn parse(attrs: &[syn::Attribute]) -> Option<Self> {
        for attr in attrs {
            if attr.path.is_ident("spirv") {
                match attr.parse_meta().expect("Failed to parse attribute meta") {
                    syn::Meta::List(l) if l.path.is_ident("spirv") => {
                        return Self::parse_meta_list(&l)
                    }
                    _ => (),
                }
            }
        }
        None
    }
}

impl Default for Spirv {
    fn default() -> Self {
        Self {
            uniform_type: UniformType::CombinedImageSampler,
            descriptor_set: None,
            binding: None,
        }
    }
}
