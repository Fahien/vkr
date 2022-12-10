use spirv_reflect::types::ReflectNumericTraits;
use vkr_core::ash::vk;

/// Returns the size in bytes of this `numeric`
pub fn numeric_size(numeric: &ReflectNumericTraits) -> u32 {
    numeric.vector.component_count * numeric.scalar.width / 8
}

pub fn numeric_to_format(numeric: &ReflectNumericTraits) -> vk::Format {
    match (numeric.vector.component_count, numeric.scalar.width) {
        (3, 32) => vk::Format::R32G32B32_SFLOAT,
        (4, 32) => vk::Format::R32G32B32A32_SFLOAT,
        _ => unimplemented!(),
    }
}

pub struct Attribute {
    pub location: u32,
    pub format: vk::Format,
    pub offset: u32,
}

impl Attribute {
    pub fn new(location: u32, format: vk::Format, offset: u32) -> Self {
        Self {
            location,
            format,
            offset,
        }
    }
}
