// Copyright © 2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::error::Error;

use spirv_builder::SpirvBuilder;

fn main() -> Result<(), Box<dyn Error>> {
    SpirvBuilder::new("res/shader")
        .spirv_version(1, 0)
        .print_metadata(true)
        .build()?;
    Ok(())
}
