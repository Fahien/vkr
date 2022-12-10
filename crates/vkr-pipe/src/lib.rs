// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::error::Error;
use std::fs::{create_dir_all, read_to_string, File};
use std::io::Write;
use std::path::PathBuf;

use glsl::parser::Parse as _;
use glsl::syntax::{self, ShaderStage};
use glsl::transpiler;
use glsl::transpiler::spirv::ShaderKind;
use proc_macro2::TokenStream;
use shaderc::CompilationArtifact;

mod template;
use template::*;

mod attribute;
use attribute::*;

fn transpile_translation_unit(
    tu: &syntax::TranslationUnit,
    kind: ShaderKind,
) -> Result<CompilationArtifact, shaderc::Error> {
    // write as GLSL in an intermediate buffer
    let mut glsl_buffer = String::new();
    transpiler::glsl::show_translation_unit(&mut glsl_buffer, tu);

    // pass the GLSL-formatted string to shaderc
    let mut compiler = shaderc::Compiler::new().unwrap();
    let options = shaderc::CompileOptions::new().unwrap();
    let kind = kind.into();
    compiler.compile_into_spirv(&glsl_buffer, kind, "glsl input", "main", Some(&options))
}

macro_rules! p {
    ($($tokens: tt)*) => {
        println!("cargo:warning={}", format!($($tokens)*))
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn pretty_print_item(ts: proc_macro2::TokenStream) -> String {
    let file = syn::parse_file(&ts.to_string()).unwrap();
    prettyplease::unparse(&file)
}

fn shader_kind_to_str(kind: ShaderKind) -> &'static str {
    match kind {
        ShaderKind::Vertex => "vert",
        ShaderKind::Fragment => "frag",
        _ => unimplemented!(),
    }
}

fn transpile_glsl(
    info: &CompileInfo,
    kind: ShaderKind,
) -> Result<CompilationArtifact, Box<dyn Error>> {
    let kind_str = shader_kind_to_str(kind);
    let code_path = format!("{}.{}.glsl", info.prefix, kind_str);
    let code = read_to_string(code_path)?;
    let unit = ShaderStage::parse(code)?;
    Ok(transpile_translation_unit(&unit, kind)?)
}

pub fn transpile(info: CompileInfo) -> Result<(), Box<dyn Error>> {
    let pipeline_name = PathBuf::from(&info.prefix)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let mut out_path = PathBuf::from(&info.out);
    out_path.push(format!("{}.rs", pipeline_name));
    if out_path.exists() {
        return Ok(());
    }

    let vert_artifact = transpile_glsl(&info, ShaderKind::Vertex)?;
    let vert_spv_data = vert_artifact.as_binary_u8();

    let frag_artifact = transpile_glsl(&info, ShaderKind::Fragment)?;
    let frag_spv_data = frag_artifact.as_binary_u8();

    // Reflection rust code
    let pipeline_struct_name: TokenStream =
        format!("{}Pipeline", capitalize(&pipeline_name)).parse()?;
    let rust_code = get_pipeline_template(pipeline_struct_name, vert_spv_data, frag_spv_data);
    create_dir_all(&info.out)?;
    File::create(out_path)?.write_all(pretty_print_item(rust_code).as_bytes())?;

    let module = spirv_reflect::create_shader_module(vert_spv_data)?;

    let mut attributes = vec![];
    let input_variables = module.enumerate_input_variables(None)?;

    // We accumulate offset as we process attributes
    let mut offset = 0;
    for input in &input_variables {
        p!(
            "{}:\t{}({})\tVec{}\tf{}",
            info.prefix,
            input.name,
            input.location,
            input.numeric.vector.component_count,
            input.numeric.scalar.width
        );

        let format = numeric_to_format(&input.numeric);
        let attribute = Attribute::new(input.location, format, offset);
        attributes.push(attribute);
        offset += numeric_size(&input.numeric);
    }

    Ok(())
}

pub struct CompileInfo {
    /// Path to the shader without `.vert.glsl` at the end
    pub prefix: String,

    /// Output directory where to store the generated pipeline source
    pub out: String,
}

impl CompileInfo {
    pub fn new<S: Into<String>>(prefix: S, out: S) -> Self {
        Self {
            prefix: prefix.into(),
            out: out.into(),
        }
    }
}
