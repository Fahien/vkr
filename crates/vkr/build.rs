// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::error::Error;
use std::fs::{create_dir, read_to_string, File};
use std::io::BufWriter;
use std::path::Path;

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

fn compile_shaders() -> Result<(), Box<dyn Error>> {
    let shader_prefixes = [
        "res/shader/main.vert",
        "res/shader/main.frag",
        "res/shader/line.vert",
        "res/shader/line.frag",
    ];
    for prefix in shader_prefixes {
        transpile(prefix)?;
    }
    Ok(())
}

fn create_test_image() -> Result<(), Box<dyn Error>> {
    let image_dir = Path::new(r"res/image");
    if !Path::exists(image_dir) {
        create_dir(image_dir)?;
    }

    let path = Path::new(r"res/image/test.png");
    if Path::exists(path) {
        return Ok(());
    }

    let file = File::create(path)?;
    let w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, 2, 2);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header()?;
    // 4 pixels
    let data = [
        180, 100, 10, 255, 20, 190, 10, 205, 40, 10, 200, 255, 80, 100, 200, 255,
    ];
    writer.write_image_data(&data)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    compile_shaders()?;
    create_test_image()?;
    Ok(())
}
