// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use vkr_pipe::{transpile, CompileInfo};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let compile_info = [
        CompileInfo::new("res/shader/main", "src/pipeline"),
        CompileInfo::new("res/shader/line", "src/pipeline"),
    ];
    for info in compile_info {
        transpile(info)?;
    }
    Ok(())
}
