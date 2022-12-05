// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::error::Error;
use std::fs::{read_to_string, File};

use glsl::parser::Parse as _;
use glsl::syntax::ShaderStage;
use glsl::transpiler::spirv::{transpile_translation_unit_to_binary, ShaderKind};

fn shader_kind_from_prefix(prefix: &str) -> Result<ShaderKind, String> {
    if prefix.ends_with("vert") {
        Ok(ShaderKind::Vertex)
    } else if prefix.ends_with("frag") {
        Ok(ShaderKind::Fragment)
    } else {
        Err(format!(
            "Failed to infer shader kind from prefix: `{}`",
            prefix
        ))
    }
}

fn transpile(prefix: &str) -> Result<(), Box<dyn Error>> {
    let code_path = format!("{}.glsl", prefix);
    let code = read_to_string(code_path)?;
    let vert_unit = ShaderStage::parse(code)?;

    let spv_path = format!("{}.spv", prefix);
    let mut file = File::create(spv_path)?;

    let shader_kind = shader_kind_from_prefix(prefix)?;
    transpile_translation_unit_to_binary(&mut file, &vert_unit, shader_kind)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ["res/shader/vert", "res/shader/frag"]
        .iter()
        .for_each(|prefix| {
            let err_message = format!("Failed to transpile {}", prefix);
            transpile(prefix).expect(&err_message);
        });

    Ok(())
}
