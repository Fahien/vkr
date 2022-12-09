// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::error::Error;
use std::fs::{create_dir_all, File};
use std::io::BufWriter;
use std::path::Path;

fn create_test_image() -> Result<(), Box<dyn Error>> {
    let image_dir = Path::new(r"res/image");
    if !Path::exists(image_dir) {
        create_dir_all(image_dir)?;
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
    create_test_image()?;
    Ok(())
}
