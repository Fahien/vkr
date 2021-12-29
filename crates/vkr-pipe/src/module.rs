// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::path::{Path, PathBuf};

pub struct CrateModule {
    pub crate_path: PathBuf,
    pub name: String,
    pub shader_path: PathBuf,
    pub file: syn::File,
}

impl CrateModule {
    fn parse_file<P: AsRef<Path>>(shader_path: P) -> syn::File {
        let code = std::fs::read_to_string(&shader_path).expect(&format!(
            "Failed to read {}",
            shader_path.as_ref().display()
        ));
        syn::parse_file(&code).expect(&format!(
            "Failed to parse {}",
            shader_path.as_ref().display()
        ))
    }

    /// Returns the crate name looking into its `Cargo.toml`
    fn get_crate_name(cargo_toml: &toml::Value) -> String {
        let table = cargo_toml
            .as_table()
            .expect("Failed to get Cargo.toml table");

        for (key, value) in table {
            if key == "package" {
                let package = value.as_table().expect("Failed to get package table");
                for (key, value) in package {
                    if key == "name" {
                        return value.as_str().expect("Failed to get lib value").into();
                    }
                }
            }
        }
        panic!("Failed to get crate name");
    }

    /// Returns the shader file name looking into its `Cargo.toml`
    fn get_shader_path(cargo_toml: &toml::Value) -> PathBuf {
        let table = cargo_toml
            .as_table()
            .expect("Failed to get Cargo.toml table");

        for (key, value) in table {
            if key == "lib" {
                let lib = value.as_table().expect("Failed to get lib table");
                for (key, value) in lib {
                    if key == "path" {
                        return value.as_str().expect("Failed to get lib value").into();
                    }
                }
            }
        }
        "src/lib.rs".into() // default value
    }

    pub fn new(crate_path: PathBuf) -> Self {
        let cargo_toml_path = crate_path.join("Cargo.toml");
        let cargo_toml_str = std::fs::read_to_string(&cargo_toml_path)
            .expect(&format!("Failed to read {}", cargo_toml_path.display()));
        let cargo_toml: toml::Value = toml::from_str(&cargo_toml_str)
            .expect(&format!("Failter to parse {}", cargo_toml_path.display()));

        let name = Self::get_crate_name(&cargo_toml);
        let shader_path = crate_path.join(Self::get_shader_path(&cargo_toml));
        let file = Self::parse_file(&shader_path);

        Self {
            crate_path,
            name,
            shader_path,
            file,
        }
    }
}

#[test]
fn load_crate() {
    let cargo_toml = toml::toml!(
    [package]
    name = "simple-shader"
    [workspace]
    [lib]
    crate-type = ["lib", "dylib"]
    path = "src/simple.rs"
        );

    let name = CrateModule::get_crate_name(&cargo_toml);
    assert!(name == "simple-shader");

    let shader_path = CrateModule::get_shader_path(&cargo_toml);
    assert!(shader_path == std::path::Path::new("src/simple.rs"));
}
