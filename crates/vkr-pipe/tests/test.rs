// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::error::Error;

use vkr_pipe::{transpile, CompileInfo};

#[test]
fn simple() -> Result<(), Box<dyn Error>> {
    transpile(CompileInfo::new("tests/shader/simple", "tests/pipeline"))
}
