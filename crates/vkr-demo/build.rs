// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::error::Error;

use spirv_builder::{MetadataPrintout, SpirvBuilder, Capability};

fn main() -> Result<(), Box<dyn Error>> {
    SpirvBuilder::new("../shader/main", "spirv-unknown-vulkan1.1")
        .print_metadata(MetadataPrintout::Full)
        .capability(Capability::InputAttachment)
        .build()?;
    Ok(())
}
