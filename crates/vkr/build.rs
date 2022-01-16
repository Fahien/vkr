// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::error::Error;

use spirv_builder::{Capability, MetadataPrintout, SpirvBuilder};

fn main() -> Result<(), Box<dyn Error>> {
    SpirvBuilder::new("../shader/present", "spirv-unknown-vulkan1.1")
        .print_metadata(MetadataPrintout::Full)
        .capability(Capability::InputAttachment)
        .build()?;
    SpirvBuilder::new("../shader/gui", "spirv-unknown-vulkan1.1")
        .print_metadata(MetadataPrintout::Full)
        .build()?;
    Ok(())
}
